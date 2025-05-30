//! FFI-compatible 'raw' structs matching the exact layout of the icon cache file.

use std::ffi::{CStr, FromBytesUntilNulError};
use std::marker::PhantomData;
use std::path::Path;
use zerocopy::{
    byteorder::network_endian::{U16, U32},
    *,
};

#[repr(C)]
#[derive(derive_more::Debug, FromBytes, KnownLayout, Immutable, Eq, PartialEq)]
pub struct Offset<V, T: ?Sized> {
    pub offset: V,
    #[debug(skip)]
    marker: PhantomData<T>,
}

impl<V, T> Offset<V, T>
where
    V: Into<u32> + Copy,
    T: TryFromBytes + KnownLayout + Immutable + ?Sized,
{
    pub fn at<'a>(&self, bytes: &'a [u8]) -> Result<&'a T, TryCastError<&'a [u8], T>> {
        let offset = self.offset.into() as usize;
        T::try_ref_from_prefix(&bytes[offset..]).map(|(t, _)| t)
    }
}

impl<V> Offset<V, CStr>
where
    V: Into<u32> + Copy,
{
    pub fn str_at<'a>(&self, bytes: &'a [u8]) -> Result<&'a CStr, FromBytesUntilNulError> {
        let offset = self.offset.into() as usize;
        CStr::from_bytes_until_nul(&bytes[offset..])
    }
}

impl<V> Offset<V, Path>
where
    V: Into<u32> + Copy,
{
    pub fn path_at<'a>(&self, bytes: &'a [u8]) -> Option<&'a Path> {
        let offset = self.offset.into() as usize;
        let cstr = CStr::from_bytes_until_nul(&bytes[offset..]).ok()?;
        let str = cstr.to_str().ok()?;
        Some(Path::new(str))
    }
}

impl<V, T: ?Sized> Clone for Offset<V, T>
where
    V: Clone,
{
    fn clone(&self) -> Self {
        Self {
            offset: self.offset.clone(),
            marker: Default::default(),
        }
    }
}

impl<V, T: ?Sized> Copy for Offset<V, T> where V: Copy {}

impl<V, T> Offset<V, T>
where
    T: ?Sized,
{
    pub fn new(offset: impl Into<V>) -> Self {
        Self {
            offset: offset.into(),
            marker: Default::default(),
        }
    }
}

impl<T> Offset<U32, T>
where
    T: ?Sized,
{
    #[inline(always)]
    pub fn is_null(&self) -> bool {
        self.offset == 0 || self.offset == 0xFFFFFFFF
    }
}

#[repr(C)]
#[derive(Debug, FromBytes, KnownLayout, Immutable, Eq, PartialEq)]
pub struct Header {
    pub major_version: U16,
    pub minor_version: U16,
    pub hash: Offset<U32, Hash>,
    pub directory_list: Offset<U32, DirectoryList>,
}

#[repr(C)]
#[derive(Debug, FromBytes, KnownLayout, Immutable, Eq, PartialEq)]
pub struct DirectoryList {
    pub n_directories: U32,
    pub directory: [Offset<U32, Path>],
}

#[repr(C)]
#[derive(Debug, FromBytes, KnownLayout, Immutable, Eq, PartialEq)]
pub struct Hash {
    pub n_buckets: U32,
    pub icon: [Offset<U32, Icon>],
}

#[repr(C)]
#[derive(Debug, FromBytes, KnownLayout, Immutable, Eq, PartialEq)]
pub struct Icon {
    pub chain: Offset<U32, Icon>,
    pub name: Offset<U32, CStr>,
    pub image_list: Offset<U32, ImageList>,
}

impl Icon {
    pub(crate) fn iter<'a>(&'a self, bytes: &'a [u8]) -> impl Iterator<Item = &'a Icon> {
        let mut icon = Some(self);

        std::iter::from_fn(move || {
            let result = icon;

            if let Some(result) = result {
                if result.chain.is_null() {
                    icon = None; // Next `result` will be None
                    return Some(result);
                }

                icon = result.chain.at(bytes).ok()
            }

            result
        })
    }
}

#[repr(C)]
#[derive(Debug, FromBytes, KnownLayout, Immutable, Eq, PartialEq)]
pub struct ImageList {
    pub n_images: U32,
    pub images: [Image],
}

#[repr(C)]
#[derive(Debug, Copy, Clone, FromBytes, KnownLayout, Immutable, Eq, PartialEq)]
pub struct Image {
    pub directory_index: U16,
    pub icon_flags: Flags,
    pub image_data: Offset<U32, ImageData>,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default, FromBytes, Immutable, Eq, PartialEq)]
pub struct Flags {
    value: U16,
}

impl Flags {
    pub const HAS_SUFFIX_XPM: U16 = U16::new(1);
    pub const HAS_SUFFIX_SVG: U16 = U16::new(2);
    pub const HAS_SUFFIX_PNG: U16 = U16::new(4);
    pub const HAS_ICON_FILE: U16 = U16::new(8);

    pub fn new(value: U16) -> Self {
        Flags { value }
    }

    pub fn bits(&self) -> U16 {
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
#[derive(Debug, Copy, Clone, FromBytes, KnownLayout, Immutable, Eq, PartialEq)]
pub struct ImageData {
    pub image_pixel_data: Offset<U32, ()>,
    pub image_meta_data: Offset<U32, MetaData>,
    pub image_pixel_data_type: Offset<U32, ()>,
    pub image_pixel_data_length: Offset<U32, ()>,
    // pixel_data
}

#[repr(C)]
#[derive(Debug, Copy, Clone, FromBytes, KnownLayout, Immutable, Eq, PartialEq)]
pub struct MetaData {
    pub embedded_rect: Offset<U32, EmbeddedRect>,
    pub attach_point_list: Offset<U32, AttachPointList>,
    pub display_name_list: Offset<U32, DisplayNameList>,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default, FromBytes, Immutable, Eq, PartialEq)]
pub struct EmbeddedRect {
    pub x0: U16,
    pub y0: U16,
    pub x1: U16,
    pub y1: U16,
}

#[repr(C)]
#[derive(Debug, FromBytes, KnownLayout, Immutable, Eq, PartialEq)]
pub struct AttachPointList {
    pub n_attach_points: U32,
    pub attach_points: [AttachPoint],
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default, FromBytes, Immutable, Eq, PartialEq)]
pub struct AttachPoint {
    pub x: U16,
    pub y: U16,
}

#[repr(C)]
#[derive(Debug, FromBytes, KnownLayout, Immutable, Eq, PartialEq)]
pub struct DisplayNameList {
    pub n_display_names: U32,
    pub display_name: [DisplayName],
}

#[repr(C)]
#[derive(Debug, Copy, Clone, FromBytes, Immutable, Eq, PartialEq)]
pub struct DisplayName {
    pub display_lang: Offset<U32, CStr>,
    pub display_name: Offset<U32, CStr>,
}
