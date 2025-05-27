use crate::raw::{Hash, Header};
use std::error::Error;
use std::ffi::CStr;
use zerocopy::TryFromBytes;

pub mod raw;

pub struct IconCache<'a> {
    pub bytes: &'a [u8],
    pub header: &'a Header,
    pub hash: &'a Hash,
}

#[derive(Debug, Copy, Clone)]
pub struct Icon<'a> {
    pub chain: &'a CStr,
    pub name: &'a CStr,
    pub image_list: &'a raw::ImageList,
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

    pub fn icon(&self, icon_idx: usize) -> Option<Icon> {
        if icon_idx > self.hash.n_buckets.get() as usize {
            return None;
        }

        let icon_offset = self.hash.icon_offset[icon_idx].get() as usize;
        let icon = raw::Icon::try_ref_from_prefix(&self.bytes[icon_offset..])
            .ok()
            .map(|(icon, _)| icon)?;

        let chain = CStr::from_bytes_until_nul(self.at(icon.chain_offset)).ok()?;
        let name = CStr::from_bytes_until_nul(self.at(icon.name_offset)).ok()?;
        let (image_list, _) =
            raw::ImageList::try_ref_from_prefix(self.at(icon.image_list_offset)).ok()?;

        Some(Icon {
            chain,
            name,
            image_list,
        })
    }

    fn at(&self, offset: zerocopy::network_endian::U32) -> &[u8] {
        &self.bytes[offset.get() as usize..]
    }
}

pub fn icon_str_hash(key: &CStr) -> u32 {
    let bytes = key.to_bytes();

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
    use crate::raw::DirectoryList;
    use std::ffi::{c_void, CString};
    use zerocopy::{byteorder::network_endian::U32, IntoBytes, TryFromBytes};

    use memmap2::Mmap;
    use std::error::Error;
    use std::fs::File;
    use std::ops::Deref;

    #[test]
    fn test() -> Result<(), Box<dyn Error>> {
        let file = File::open("/usr/share/icons/gnome/icon-theme.cache")?;

        let mmap = unsafe { Mmap::map(&file)? };
        let cache = IconCache::new_from_bytes(mmap.deref()).unwrap();

        let icon = cache.icon(1).unwrap();

        println!("{:?}", icon.name);
        println!("{:?}", icon.chain);

        Ok(())
    }

    #[test]
    fn g_str_hash_empty() {
        let c_str = CString::new("").unwrap();

        assert_eq!(icon_str_hash(&c_str), 0);
    }

    #[test]
    fn g_str_hash_hello_world() {
        let c_str = CString::new("hello world").unwrap();
        let ptr = c_str.as_ptr() as *const c_void;
        // assert_eq!(g_str_hash(&c_str), hash);
        // TODO: icon_str_hash is NOT g_str_hash, figure out from the actual files what
        // the expected values are
    }

    #[test]
    #[cfg(feature = "alloc")]
    fn it_works() {
        let bytes: [U32; 4] = [3.into(), 1.into(), 2.into(), 3.into()];
        let bytes = bytes.as_bytes();

        let result = DirectoryList::try_ref_from_bytes(bytes).unwrap();

        println!("{:?}", result);
    }
}
