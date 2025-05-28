use std::error::Error;
use std::ffi::CStr;
use zerocopy::{FromBytes, TryCastError};

pub mod raw;

#[derive(derive_more::Debug, Copy, Clone)]
pub struct IconCache<'a> {
    #[debug(skip)]
    pub bytes: &'a [u8],
    pub header: &'a raw::Header,
    pub hash: &'a raw::Hash,
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

    pub fn icon(&self, icon_name: impl AsRef<[u8]>) -> Option<Icon<'a>> {
        let icon_name = icon_name.as_ref();
        let hash = icon_str_hash(icon_name);
        let n_buckets = self.hash.n_buckets.get();
        let bucket = hash % n_buckets;

        let mut icon = self.icon_chain(bucket).ok()?;
        loop {
            if let Ok(name) = icon.name.str_at(self.bytes) {
                if name.to_bytes() == icon_name {
                    return Some(Icon {
                        name,
                        image_list: ImageList {
                            bytes: self.bytes,
                            raw_list: icon.image_list.at(self.bytes).ok()?,
                        },
                    });
                }
            }

            if icon.chain.offset == 0xFFFFFFFF {
                return None;
            }

            icon = icon.chain.at(self.bytes).ok()?;
        }
    }

    pub fn icon_chain(
        &self,
        bucket: u32,
    ) -> Result<&'a raw::Icon, TryCastError<&'a [u8], raw::Icon>> {
        debug_assert!(bucket < self.hash.n_buckets.get());

        self.hash.icon[bucket as usize].at(self.bytes)
    }
}

#[derive(derive_more::Debug, Copy, Clone)]
pub struct DirectoryList<'a> {
    #[debug(skip)]
    #[allow(unused)] // clippy thinks this is unused but it isn't? maybe because of the Debug
    bytes: &'a [u8],
    pub raw_list: &'a raw::DirectoryList,
}

impl<'a> DirectoryList<'a> {
    #[inline(always)]
    fn len(&self) -> u32 {
        self.raw_list.n_directories.get()
    }

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn dir(&self, idx: u32) -> Option<&'a CStr> {
        if idx >= self.len() {
            return None;
        }

        self.raw_list.directory[idx as usize]
            .str_at(self.bytes)
            .ok()
    }

    fn iter(&self) -> impl Iterator<Item = &'a CStr> {
        (0..self.len()).filter_map(|idx| self.dir(idx))
    }
}

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
    pub fn len(&self) -> u32 {
        self.raw_list.n_images.get()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

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
            .str_at(self.bytes)
            .ok()?;

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

    pub fn iter(&self) -> impl Iterator<Item = Image<'a>> {
        (0..self.len())
            .filter_map(|idx| self.image(idx))
    }
}

#[derive(derive_more::Debug, Copy, Clone)]
pub struct Image<'a> {
    pub directory: &'a CStr,
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

pub fn icon_str_hash(key: impl AsRef<[u8]>) -> u32 {
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
    // from my system-installed Adwaita theme.
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

        assert_eq!(image.directory.to_str(), Ok("scalable/apps"));
        assert_eq!(
            image.icon_flags,
            raw::Flags::new(raw::Flags::HAS_SUFFIX_SVG)
        );
        assert!(image.image_data.is_none());

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
