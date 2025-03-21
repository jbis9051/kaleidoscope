pub mod standard;
pub mod heif;
pub mod video;
pub mod raw;

use std::cmp::max;
use std::path::{Path, PathBuf};
use std::time::Duration;
use image::{RgbImage};
use serde::{Deserialize, Serialize};
use sqlx::types::chrono;
use crate::format_type::FormatType;

#[derive(Debug)]
pub struct MediaMetadata {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub size: u32,
    pub created_at: chrono::NaiveDateTime,
    pub duration: Option<Duration>,
    pub longitude: Option<f64>,
    pub latitude: Option<f64>,
    pub is_screenshot: bool,
}

pub trait Format<T> {
    const EXTENSIONS: &'static [&'static str];

    const METADATA_VERSION: i32; // bump this if the metadata format changes

    fn is_supported(path: &Path) -> bool {
        let ext = path.extension().unwrap_or_default().to_str().unwrap_or_default().to_lowercase();
        Self::EXTENSIONS.contains(&ext.as_str())
    }

    fn is_photo() -> bool;

    fn get_metadata(path: &Path) -> Result<MediaMetadata, T>;
}

pub trait Thumbnailable<T>: Format<T> {
    const THUMBNAIL_VERSION: i32; // bump this if the thumbnail format changes

    fn generate_thumbnail(path: &Path, width: u32, height: u32) -> Result<RgbImage, T>;

    fn generate_full(path: &Path) -> Result<RgbImage, T> {
        let metadata = Self::get_metadata(path)?;
        let width = metadata.width;
        let height = metadata.height;
        Self::generate_thumbnail(path, width, height)
    }
}


#[macro_export]
macro_rules! match_format {
    ($format: expr, |$format_type: ident| $code: block) => {{
        use $crate::format_type::FormatType;
        use $crate::media_processors::format::*;

        match $format {
            FormatType::Standard => {
                type $format_type = standard::Standard;
                $code
            },
            FormatType::Heif => {
                type $format_type = heif::Heif;
                $code
            },
            FormatType::Video => {
                type $format_type = video::Video;
                $code
            },
            FormatType::Raw => {
                type $format_type = raw::Raw;
                $code
            },
            _ => panic!("invalid format type: {:?}", $format),
        }
    }};

    (thumbnailable $format: expr, |$format_type: ident| $code: block) => {
        match_format!(thumbnailable $format, |$format_type| $code, { panic!("invalid format type, not thumbnailable: {:?}", $format) })
    };

    (thumbnailable $format: expr, |$format_type: ident| $code: block, $code_not_thumbnailable: block) => {{
        use $crate::format_type::FormatType;
        use $crate::media_processors::format::*;

        match $format {
            FormatType::Standard => {
                type $format_type = standard::Standard;
                $code
            },
            FormatType::Heif => {
                type $format_type = heif::Heif;
                $code
            },
            FormatType::Video => {
                type $format_type = video::Video;
                $code
            },
            FormatType::Raw => {
                type $format_type = raw::Raw;
                $code
            }
            _ => $code_not_thumbnailable,
        }
    }};
}

pub struct AnyFormat {
    format: FormatType,
    path: PathBuf
}

impl AnyFormat {
    pub fn try_new(path: PathBuf) -> Option<Self> {
        let format = {
            if standard::Standard::is_supported(&path) {
                FormatType::Standard
            } else if heif::Heif::is_supported(&path) {
                FormatType::Heif
            } else if video::Video::is_supported(&path) {
                FormatType::Video
            } else if raw::Raw::is_supported(&path) {
                FormatType::Raw
            } else {
                return None;
            }
        };

        Some(Self {
            format,
            path
        })
    }

    pub fn format_type(&self) -> FormatType {
        self.format
    }

    pub fn is_photo(&self) -> bool {
        match_format!(self.format, |ActualFormat| { <ActualFormat as Format<_>>::is_photo()})
    }

    pub fn thumbnailable(&self) -> bool {
        match_format!(thumbnailable  self.format, |ActualFormat| { true }, { false })
    }

    pub fn get_metadata(&self) -> Result<MediaMetadata, MetadataError> {
        match_format!(self.format, |ActualFormat| { <ActualFormat as Format<_>>::get_metadata(&self.path).map_err(|e| e.into()) })
    }

    pub fn generate_thumbnail(&self, width: u32, height: u32) -> Result<RgbImage, MetadataError> {
        match_format!(thumbnailable  self.format, |ActualFormat| { <ActualFormat as Thumbnailable<_>>::generate_thumbnail(&self.path, width, height).map_err(|e| e.into()) })
    }

    pub fn generate_full(&self) -> Result<RgbImage, MetadataError> {
        match_format!(thumbnailable self.format, |ActualFormat| { <ActualFormat as Thumbnailable<_>>::generate_full(&self.path).map_err(|e| e.into()) })
    }

    pub fn metadata_version(&self) -> i32 {
        match_format!(self.format, |ActualFormat| { <ActualFormat as Format<_>>::METADATA_VERSION })
    }

    pub fn thumbnail_version(&self) -> i32 {
        match_format!(thumbnailable self.format, |ActualFormat| { <ActualFormat as Thumbnailable<_>>::THUMBNAIL_VERSION })
    }
}


#[derive(Debug, thiserror::Error)]
pub enum MetadataError {
    #[error("standard format error: {0}")]
    Standard(#[from] standard::StandardError),
    #[error("heif format error: {0}")]
    Heif(#[from] heif::HeifError),
    #[error("video format error: {0}")]
    Video(#[from] video::VideoError),
    #[error("raw format error: {0}")]
    Raw(#[from] raw::RawError),
}



/// Calculates the width and height an image should be resized to.
/// This preserves aspect ratio, and based on the `fill` parameter
/// will either fill the dimensions to fit inside the smaller constraint
/// (will overflow the specified bounds on one axis to preserve
/// aspect ratio), or will shrink so that both dimensions are
/// completely contained within the given `width` and `height`,
/// with empty space on one axis.
/// (*Stolen from image crate*)
pub fn resize_dimensions(
    width: u32,
    height: u32,
    nwidth: u32,
    nheight: u32,
    fill: bool,
) -> (u32, u32) {
    let wratio = nwidth as f64 / width as f64;
    let hratio = nheight as f64 / height as f64;

    let ratio = if fill {
        f64::max(wratio, hratio)
    } else {
        f64::min(wratio, hratio)
    };

    let nw = max((width as f64 * ratio).round() as u64, 1);
    let nh = max((height as f64 * ratio).round() as u64, 1);

    if nw > u64::from(u32::MAX) {
        let ratio = u32::MAX as f64 / width as f64;
        (u32::MAX, max((height as f64 * ratio).round() as u32, 1))
    } else if nh > u64::from(u32::MAX) {
        let ratio = u32::MAX as f64 / height as f64;
        (max((width as f64 * ratio).round() as u32, 1), u32::MAX)
    } else {
        (nw as u32, nh as u32)
    }
}