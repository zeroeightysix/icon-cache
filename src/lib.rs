use zerocopy::{*, byteorder::network_endian::{U16, U32}};

#[repr(C)]
#[derive(Debug, FromZeros, KnownLayout, Immutable)]
pub struct Header {
    pub major_version: U16,
    pub minor_version: U16,
    pub hash_offset: U32,
    pub directory_list_offset: U32,
}

#[repr(C)]
#[derive(Debug, FromZeros, KnownLayout, Immutable)]
pub struct DirectoryList {
    pub n_directories: U32,
    pub directory_offset: [U32],
}

#[repr(C)]
#[derive(Debug, TryFromBytes, KnownLayout, Immutable)]
pub struct Hash {
    pub n_buckets: U32,
    pub icon_offset: [U32],
}

#[repr(C)]
#[derive(Debug, Copy, Clone, FromZeros, KnownLayout, Immutable)]
pub struct Icon {
    pub chain_offset: U32,
    pub name_offset: U32,
    pub image_list_offset: U32,
}

#[repr(C)]
#[derive(Debug, TryFromBytes, KnownLayout, Immutable)]
pub struct ImageList {
    pub n_images: U32,
    pub images: [Image],
}

#[repr(C)]
#[derive(Debug, Copy, Clone, FromZeros, KnownLayout, Immutable)]
pub struct Image {
    pub directory_index: U16,
    pub icon_flags: Flags,
    pub image_data_offset: U32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default, FromZeros, Immutable)]
pub struct Flags {
    value: U32,
}

impl Flags {
    pub const HAS_SUFFIX_XPM: U32 = U32::new(1);
    pub const HAS_SUFFIX_SVG: U32 = U32::new(2);
    pub const HAS_SUFFIX_PNG: U32 = U32::new(4);
    pub const HAS_ICON_FILE: U32 = U32::new(8);

    pub fn new(value: U32) -> Self {
        Flags { value }
    }

    pub fn bits(&self) -> U32 {
        self.value
    }

    pub fn has_suffix_xpm(&self) -> bool {
        (self.value & Self::HAS_SUFFIX_XPM) != 0
    }

    pub fn has_suffix_svg(&self) -> bool {
        (self.value & Self::HAS_SUFFIX_SVG) != 0
    }

    pub fn has_suffix_png(&self) -> bool {
        (self.value & Self::HAS_SUFFIX_PNG) != 0
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, FromZeros, KnownLayout, Immutable)]
pub struct ImageData {
    pub image_pixel_data_offset: U32,
    pub image_meta_data_offset: U32,
    pub image_pixel_data_type: U32,
    pub image_pixel_data_length: U32,
    // pixel_data
}

#[repr(C)]
#[derive(Debug, Copy, Clone, FromZeros, KnownLayout, Immutable)]
pub struct MetaData {
    pub embedded_rect_offset: U32,
    pub attach_point_list_offset: U32,
    pub display_name_list_offset: U32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default, FromZeros, Immutable)]
pub struct EmbeddedRect {
    pub x0: U16,
    pub y0: U16,
    pub x1: U16,
    pub y1: U16,
}

#[repr(C)]
#[derive(Debug, TryFromBytes, KnownLayout, Immutable)]
pub struct AttachPointList {
    pub n_attach_points: U32,
    pub attach_points: [AttachPoint]
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default, FromZeros, Immutable)]
pub struct AttachPoint {
    pub x: U16,
    pub y: U16,
}

#[repr(C)]
#[derive(Debug, TryFromBytes, KnownLayout, Immutable)]
pub struct DisplayNameList {
    pub n_display_names: U32,
    pub display_name: [DisplayName],
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default, FromZeros, Immutable)]
pub struct DisplayName {
    pub display_lang_offset: U32,
    pub display_name_offset: U32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use zerocopy::IntoBytes;

    #[test]
    #[cfg(feature = "alloc")]
    fn it_works() {
        let bytes: [U32; 4] = [3.into(), 1.into(), 2.into(), 3.into()];
        let bytes = bytes.as_bytes();

        let result = DirectoryList::try_ref_from_bytes(bytes).unwrap();

        println!("{:?}", result);
    }
}
