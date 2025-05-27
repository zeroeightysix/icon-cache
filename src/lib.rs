use crate::raw::{Hash, Header, ImageData};
use std::error::Error;
use std::ffi::CStr;
use zerocopy::TryFromBytes;
use zerocopy::network_endian::U32;

pub mod raw;

#[derive(derive_more::Debug, Copy, Clone)]
pub struct IconCache<'a> {
    #[debug(skip)]
    pub bytes: &'a [u8],
    pub header: &'a Header,
    pub hash: &'a Hash,
}

impl<'a> IconCache<'a> {
    pub fn new_from_bytes(bytes: &'a [u8]) -> Result<Self, Box<dyn Error + 'a>> {
        let (header, _) = Header::try_ref_from_prefix(bytes)?;

        let hash_offset = header.hash_offset.get() as usize;
        let (hash, _) = Hash::try_ref_from_prefix(&bytes[hash_offset..])?;

        Ok(IconCache {
            bytes,
            header,
            hash,
        })
    }

    pub fn icon<'c: 'a>(&'c self, icon_name: impl AsRef<[u8]>) -> Option<Icon<'a>> {
        let icon_name = icon_name.as_ref();
        let hash = icon_str_hash(icon_name);
        let n_buckets = self.hash.n_buckets.get();
        let bucket = hash % n_buckets;

        let mut chain = self.icon_chain(bucket).ok()?;
        loop {
            if chain.icon.name.to_bytes() == icon_name {
                let icon: Icon<'a> = chain.icon;

                return Some(icon);
            }

            let Some(Ok(next_chain)) = chain.next_in_chain() else {
                return None;
            };

            chain = next_chain;
        }
    }

    pub fn icon_chain(&self, bucket: u32) -> Result<IconChain<'a>, Box<dyn Error + 'a>> {
        debug_assert!(bucket < self.hash.n_buckets.get());

        let icon_offset = self.hash.icon_offset[bucket as usize].get();
        IconChain::new_at_offset(self.bytes, icon_offset)
    }

    fn at(&self, offset: U32) -> &[u8] {
        &self.bytes[offset.get() as usize..]
    }
}

#[derive(derive_more::Debug, Copy, Clone)]
pub struct IconChain<'a> {
    #[debug(skip)]
    pub bytes: &'a [u8],
    pub chain: u32,
    pub icon: Icon<'a>,
}

impl<'a> IconChain<'a> {
    fn new_at_offset(bytes: &'a [u8], offset: u32) -> Result<IconChain<'a>, Box<dyn Error + 'a>> {
        let raw_icon =
            raw::Icon::try_ref_from_prefix(&bytes[offset as usize..]).map(|(icon, _)| icon)?;

        let name = &bytes[raw_icon.name_offset.get() as usize..];
        let name = CStr::from_bytes_until_nul(name)?;

        let image_list = &bytes[raw_icon.image_list_offset.get() as usize..];
        let (image_list, _) = raw::ImageList::try_ref_from_prefix(image_list)?;

        let icon = Icon {
            name,
            image_list: ImageList(image_list),
        };

        Ok(IconChain {
            bytes,
            chain: raw_icon.chain_offset.get(),
            icon,
        })
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Icon<'a> {
    pub name: &'a CStr,
    pub image_list: ImageList<'a>,
}

#[derive(Debug, Copy, Clone)]
pub struct ImageList<'a>(pub &'a raw::ImageList);

impl<'a> ImageList<'a> {
    pub fn len(&self) -> u32 {
        self.0.n_images.get()
    }

    pub fn image(&self, idx: u32) {
        debug_assert!(idx < self.0.n_images.get());

        // let list = self.0;
        todo!()
    }
}

pub struct Image<'a> {
    pub directory: &'a CStr,
    pub icon_flags: raw::Flags,
    pub image_data: &'a ImageData,
}

impl<'a> IconChain<'a> {
    pub fn next_in_chain(&self) -> Option<Result<IconChain<'a>, Box<dyn Error + 'a>>> {
        if self.chain == 0xFFFFFFFF {
            return None;
        }

        Some(IconChain::new_at_offset(self.bytes, self.chain))
    }
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
    use zerocopy::network_endian::U16;

    #[test]
    fn test_find_specific_icon() -> Result<(), Box<dyn Error>> {
        // The included sample cache file was generated using the gtk-update-icon-cache utility
        // from my system-installed Adwaita theme.
        static SAMPLE_INDEX_FILE: &[u8] = include_bytes!("../assets/icon-theme.cache");

        let cache = IconCache::new_from_bytes(&SAMPLE_INDEX_FILE)?;

        assert_eq!(
            cache.header,
            &Header {
                major_version: U16::new(1,),
                minor_version: U16::new(0,),
                hash_offset: U32::new(12,),
                directory_list_offset: U32::new(37788,),
            }
        );

        assert_eq!(cache.hash.n_buckets, 251);

        let icon = cache.icon("preferences-other-symbolic").unwrap();

        assert_eq!(icon.name.to_str(), Ok("preferences-other-symbolic"));
        assert_eq!(icon.image_list.len(), 1);

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
}
