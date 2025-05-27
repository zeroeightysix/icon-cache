use zerocopy::{
    byteorder::network_endian::{U16, U32},
    *,
};

#[repr(C)]
#[derive(Debug, FromZeros, KnownLayout, Immutable, Eq, PartialEq)]
pub struct Header {
    pub major_version: U16,
    pub minor_version: U16,
    pub hash_offset: U32,
    pub directory_list_offset: U32,
}

#[repr(C)]
#[derive(Debug, FromZeros, KnownLayout, Immutable, Eq, PartialEq)]
pub struct DirectoryList {
    pub n_directories: U32,
    pub directory_offset: [U32],
}

#[repr(C)]
#[derive(Debug, TryFromBytes, KnownLayout, Immutable, Eq, PartialEq)]
pub struct Hash {
    pub n_buckets: U32,
    pub icon_offset: [U32],
}

#[repr(C)]
#[derive(Debug, Copy, Clone, FromZeros, KnownLayout, Immutable, Eq, PartialEq)]
pub struct Icon {
    pub chain_offset: U32, // Points to another Icon
    pub name_offset: U32,  // Points to a C string
    pub image_list_offset: U32,
}

#[repr(C)]
#[derive(Debug, TryFromBytes, KnownLayout, Immutable, Eq, PartialEq)]
pub struct ImageList {
    pub n_images: U32,
    pub images: [Image],
}

#[repr(C)]
#[derive(Debug, Copy, Clone, FromZeros, KnownLayout, Immutable, Eq, PartialEq)]
pub struct Image {
    pub directory_index: U16,
    pub icon_flags: Flags,
    pub image_data_offset: U32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default, FromZeros, Immutable, Eq, PartialEq)]
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
#[derive(Debug, Copy, Clone, FromZeros, KnownLayout, Immutable, Eq, PartialEq)]
pub struct ImageData {
    pub image_pixel_data_offset: U32,
    pub image_meta_data_offset: U32,
    pub image_pixel_data_type: U32,
    pub image_pixel_data_length: U32,
    // pixel_data
}

#[repr(C)]
#[derive(Debug, Copy, Clone, FromZeros, KnownLayout, Immutable, Eq, PartialEq)]
pub struct MetaData {
    pub embedded_rect_offset: U32,
    pub attach_point_list_offset: U32,
    pub display_name_list_offset: U32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default, FromZeros, Immutable, Eq, PartialEq)]
pub struct EmbeddedRect {
    pub x0: U16,
    pub y0: U16,
    pub x1: U16,
    pub y1: U16,
}

#[repr(C)]
#[derive(Debug, TryFromBytes, KnownLayout, Immutable, Eq, PartialEq)]
pub struct AttachPointList {
    pub n_attach_points: U32,
    pub attach_points: [AttachPoint],
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default, FromZeros, Immutable, Eq, PartialEq)]
pub struct AttachPoint {
    pub x: U16,
    pub y: U16,
}

#[repr(C)]
#[derive(Debug, TryFromBytes, KnownLayout, Immutable, Eq, PartialEq)]
pub struct DisplayNameList {
    pub n_display_names: U32,
    pub display_name: [DisplayName],
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default, FromZeros, Immutable, Eq, PartialEq)]
pub struct DisplayName {
    pub display_lang_offset: U32,
    pub display_name_offset: U32,
}
