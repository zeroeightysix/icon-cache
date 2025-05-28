//! Complete and user-friendly zero-copy wrappers for the GTK icon cache which is present on most
//! linux systems.
//!
//! GTK's icon cache maintains a hash-indexed map from icon names (e.g. `open-menu`) to a list of
//! images representing that icon, each in a different directory, usually denoting that icon's size,
//! whether it's scalable, etc.
//!
//! This crate provides a safe wrapper around this cache and is designed for use with `mmap`.
//! To get started, look at [IconCache].

use std::error::Error;
use std::ffi::CStr;
use std::path::Path;
use zerocopy::FromBytes;

pub mod raw;

/// Thin wrapper around an in-memory icon cache.
///
/// This is `icon-cache`'s main entrypoint. To look up an icon, use the [icon](IconCache::icon) function.
///
/// `IconCache`'s fields may be interesting for advanced uses, but if all you need is to look up
/// icons—use [icon](IconCache::icon).
#[derive(derive_more::Debug, Copy, Clone)]
pub struct IconCache<'a> {
    /// The raw bytes representing the cache
    #[debug(skip)]
    pub bytes: &'a [u8],
    /// Cache header file: contains version and hash & directory list offsets
    pub header: &'a raw::Header,
    /// Internal hash table storing mapping from icon names to icon information
    pub hash: &'a raw::Hash,
    /// List of directories within the theme, relative to the theme's root
    pub directory_list: DirectoryList<'a>,
}

impl<'a> IconCache<'a> {
    pub fn new_from_bytes(bytes: &'a [u8]) -> Result<Self, Box<dyn Error + 'a>> {
        let (header, _) = raw::Header::ref_from_prefix(bytes)?;

        let hash = header.hash.at(bytes)?;
        let directory_list = header.directory_list.at(bytes)?;
        let directory_list = DirectoryList {
            bytes,
            raw_list: directory_list,
        };

        Ok(IconCache {
            bytes,
            header,
            hash,
            directory_list,
        })
    }

    /// Look up an icon by name in the cache. `icon_name` accepts any type that turns into a byte
    /// slice: typically `str` suffices.
    ///
    /// Returns `None` if no icon by that name exists within the icon theme, or if parsing failed.
    pub fn icon(&self, icon_name: impl AsRef<[u8]>) -> Option<Icon<'a>> {
        let icon_name = icon_name.as_ref();
        let hash = icon_str_hash(icon_name);
        let n_buckets = self.hash.n_buckets.get();
        let bucket = hash % n_buckets;

        let icons = self.icon_chain(bucket)?.iter(self.bytes);

        for icon in icons {
            let Ok(name) = icon.name.str_at(self.bytes) else {
                continue;
            };

            if name.to_bytes() == icon_name {
                return Some(Icon {
                    name,
                    image_list: ImageList::from_icon(icon, self.bytes)?,
                });
            }
        }

        None
    }

    pub fn iter(&self) -> impl Iterator<Item = Icon<'a>> {
        (0..self.hash.n_buckets.get())
            .filter_map(|bucket| self.icon_chain(bucket))
            .flat_map(|chain| chain.iter(self.bytes))
            .filter_map(|icon| {
                Some(Icon {
                    name: icon.name.str_at(self.bytes).ok()?,
                    image_list: ImageList::from_icon(icon, self.bytes)?,
                })
            })
    }

    fn icon_chain(&self, bucket: u32) -> Option<&'a raw::Icon> {
        debug_assert!(bucket < self.hash.n_buckets.get());

        let offset = self.hash.icon[bucket as usize];
        // A bucket may be empty!
        if offset.is_null() {
            return None;
        }

        offset.at(self.bytes).ok()
    }
}

/// List of directories in the icon theme with paths relative to the root of the icon theme.
#[derive(derive_more::Debug, Copy, Clone)]
pub struct DirectoryList<'a> {
    #[debug(skip)]
    #[allow(unused)] // clippy thinks this is unused but it isn't? maybe because of the Debug
    bytes: &'a [u8],
    pub raw_list: &'a raw::DirectoryList,
}

impl<'a> DirectoryList<'a> {
    /// Returns the amount of directories in this list
    #[inline(always)]
    pub fn len(&self) -> u32 {
        self.raw_list.n_directories.get()
    }

    /// Returns `true` if the list is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Access a directory by its index in the list.
    ///
    /// Returns `None` if the index larger than the length of the list.
    pub fn dir(&self, idx: u32) -> Option<&'a Path> {
        if idx >= self.len() {
            return None;
        }

        self.raw_list.directory[idx as usize]
            .path_at(self.bytes)
    }

    /// Returns an iterator over the directory list
    pub fn iter(&self) -> impl Iterator<Item = &'a Path> {
        (0..self.len()).filter_map(|idx| self.dir(idx))
    }
}

/// An icon, identified by its name, and the list of images provided by the icon theme for this icon.
#[derive(Debug, Copy, Clone)]
pub struct Icon<'a> {
    pub name: &'a CStr,
    pub image_list: ImageList<'a>,
}

#[derive(derive_more::Debug, Copy, Clone)]
pub struct ImageList<'a> {
    #[debug(skip)]
    bytes: &'a [u8],
    pub raw_list: &'a raw::ImageList,
}

impl<'a> ImageList<'a> {
    fn from_icon(icon: &raw::Icon, bytes: &'a [u8]) -> Option<ImageList<'a>> {
        Some(Self {
            bytes,
            raw_list: icon.image_list.at(bytes).ok()?,
        })
    }

    /// Returns the amount of images in this list
    pub fn len(&self) -> u32 {
        self.raw_list.n_images.get()
    }

    /// Returns `true` if the list is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Access an image by its index in the list.
    ///
    /// Returns `None` if the index larger than the length of the list, or if the image data
    /// failed to parse.
    pub fn image(&self, idx: u32) -> Option<Image<'a>> {
        if idx >= self.len() {
            return None;
        }

        let raw_image = &self.raw_list.images[idx as usize];

        // TODO: how does the overhead of re-interpreting the header and directory list here over
        // passing those down from the cache struct, or alternatively re-introducing the ref to cache?
        let (header, _) = raw::Header::ref_from_prefix(self.bytes).ok()?;
        let directory_list = header.directory_list.at(self.bytes).ok()?;
        let directory = directory_list.directory[raw_image.directory_index.get() as usize]
            .path_at(self.bytes)?;

        let icon_flags = raw_image.icon_flags;

        let mut image_data = None;

        if raw_image.image_data.offset != 0 {
            let &raw::ImageData {
                image_pixel_data,
                image_meta_data,
                image_pixel_data_length,
                image_pixel_data_type,
            } = raw_image.image_data.at(self.bytes).ok()?;

            image_data = Some(ImageData {
                image_pixel_data: *image_pixel_data.at(self.bytes).ok()?,
                image_meta_data: image_meta_data.at(self.bytes).ok()?,
                image_pixel_data_type: *image_pixel_data_type.at(self.bytes).ok()?,
                image_pixel_data_length: *image_pixel_data_length.at(self.bytes).ok()?,
            });
        }

        Some(Image {
            directory,
            icon_flags,
            image_data,
        })
    }

    /// Returns an iterator over the image list
    pub fn iter(&self) -> impl Iterator<Item = Image<'a>> {
        (0..self.len()).filter_map(|idx| self.image(idx))
    }
}

#[derive(derive_more::Debug, Copy, Clone)]
pub struct Image<'a> {
    pub directory: &'a Path,
    pub icon_flags: raw::Flags,
    pub image_data: Option<ImageData<'a>>,
}

#[derive(derive_more::Debug, Copy, Clone)]
pub struct ImageData<'a> {
    pub image_pixel_data: (), // TODO: what type is this?
    pub image_meta_data: &'a raw::MetaData,
    pub image_pixel_data_type: (),
    pub image_pixel_data_length: (),
}

fn icon_str_hash(key: impl AsRef<[u8]>) -> u32 {
    let bytes = key.as_ref();

    if bytes.is_empty() {
        return 0;
    }

    let mut h: u32 = bytes[0] as u32;
    for &p in &bytes[1..] {
        h = (h << 5).overflowing_sub(h).0.overflowing_add(p as u32).0;
    }

    h
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::raw::Offset;
    use zerocopy::network_endian::U16;

    // The included sample cache file was generated using the gtk-update-icon-cache utility
    // from my system-installed hicolor theme.
    static SAMPLE_INDEX_FILE: &[u8] = include_bytes!("../assets/icon-theme.cache");

    #[test]
    fn test_find_specific_icon() -> Result<(), Box<dyn Error>> {
        let cache = IconCache::new_from_bytes(SAMPLE_INDEX_FILE)?;

        assert_eq!(
            cache.header,
            &raw::Header {
                major_version: U16::new(1),
                minor_version: U16::new(0),
                hash: Offset::new(12),
                directory_list: Offset::new(35812)
            }
        );

        assert_eq!(cache.hash.n_buckets, 251);

        let icon = cache.icon("mpv").unwrap();

        assert_eq!(icon.name.to_str(), Ok("mpv"));
        assert_eq!(icon.image_list.len(), 5);

        let image = &icon.image_list.image(0).unwrap();

        assert_eq!(image.directory.to_str(), Some("scalable/apps"));
        assert_eq!(
            image.icon_flags,
            raw::Flags::new(raw::Flags::HAS_SUFFIX_SVG)
        );
        assert!(image.image_data.is_none());

        Ok(())
    }

    #[test]
    fn test_icon_iter() -> Result<(), Box<dyn Error>> {
        let cache = IconCache::new_from_bytes(SAMPLE_INDEX_FILE)?;

        assert_eq!(cache.iter().count(), 563);

        Ok(())
    }

    #[test]
    fn test_image_list_iter() -> Result<(), Box<dyn Error>> {
        let cache = IconCache::new_from_bytes(SAMPLE_INDEX_FILE)?;
        let icon = cache.icon("mpv").unwrap();

        let count = icon.image_list.iter().count();
        assert_eq!(count, 5);

        Ok(())
    }

    #[test]
    fn test_directory_list_iter() -> Result<(), Box<dyn Error>> {
        let cache = IconCache::new_from_bytes(SAMPLE_INDEX_FILE)?;
        let dir_list = cache.directory_list;

        assert_eq!(dir_list.len(), 59);

        assert!(!cache.directory_list.is_empty());
        assert_eq!(cache.directory_list.iter().count(), 59);

        Ok(())
    }

    #[test]
    fn icon_str_hash_empty() {
        assert_eq!(icon_str_hash(""), 0);
    }

    #[test]
    fn icon_str_hash_hello_world() {
        assert_eq!(icon_str_hash("hello world"), 1794106052);
    }

    #[test]
    fn icon_str_hash_sym() {
        assert_eq!(icon_str_hash("preferences-other-symbolic") % 251, 243);
    }

    #[test]
    fn image_size_correct() {
        assert_eq!(size_of::<raw::Image>(), 8);
    }
}
