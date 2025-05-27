use std::ffi::CStr;

mod raw;

pub fn icon_str_hash(key: &CStr) -> u32 {
    let bytes = key.to_bytes();

    if bytes.len() == 0 {
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
