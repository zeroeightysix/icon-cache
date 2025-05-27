mod raw;

#[cfg(test)]
mod tests {
    use super::*;
    use zerocopy::{IntoBytes, TryFromBytes, byteorder::network_endian::U32};
    use crate::raw::DirectoryList;

    #[test]
    #[cfg(feature = "alloc")]
    fn it_works() {
        let bytes: [U32; 4] = [3.into(), 1.into(), 2.into(), 3.into()];
        let bytes = bytes.as_bytes();

        let result = DirectoryList::try_ref_from_bytes(bytes).unwrap();

        println!("{:?}", result);
    }
}
