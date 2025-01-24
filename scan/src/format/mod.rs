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
use common::format_type;
use common::format_type::FormatType;

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
    const THUMBNAIL_VERSION: i32; // bump this if the thumbnail format changes

    fn is_supported(path: &Path) -> bool {
        let ext = path.extension().unwrap_or_default().to_str().unwrap_or_default().to_lowercase();
        Self::EXTENSIONS.contains(&ext.as_str())
    }

    fn is_photo() -> bool;

    fn get_metadata(path: &Path) -> Result<MediaMetadata, T>;

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
    ($format: expr, $call: tt($($arg: expr),*)) => {
        match $format {
            FormatType::Standard => standard::Standard::$call($($arg),*).into(),
            FormatType::Heif => heif::Heif::$call($($arg),*).into(),
            FormatType::Video => video::Video::$call($($arg),*).into(),
            FormatType::Raw => raw::Raw::$call($($arg),*).into(),
            _ => panic!("invalid format type: {:?}", $format),
        }
    };
    ($format: expr, $call: tt($($arg: expr),*), err) => {
        match $format {
            FormatType::Standard => standard::Standard::$call($($arg),*).map_err(|e| e.into()),
            FormatType::Heif => heif::Heif::$call($($arg),*).map_err(|e| e.into()),
            FormatType::Video => video::Video::$call($($arg),*).map_err(|e| e.into()),
            FormatType::Raw => raw::Raw::$call($($arg),*).map_err(|e| e.into()),
            _ => panic!("invalid format type: {:?}", $format),
        }
    };
    ($format: expr, $assoc: ident) => {
        match $format {
            FormatType::Standard => standard::Standard::$assoc,
            FormatType::Heif => heif::Heif::$assoc,
            FormatType::Video => video::Video::$assoc,
            FormatType::Raw => raw::Raw::$assoc,
            _ => panic!("invalid format type: {:?}", $format),
        }
    };
}

pub struct AnyFormat {
    format: FormatType,
    path: PathBuf
}

impl AnyFormat {
    pub fn new(path: PathBuf) -> Option<Self> {
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
        match_format!(self.format, is_photo())
    }

    pub fn get_metadata(&self) -> Result<MediaMetadata, MetadataError> {
        match_format!(self.format, get_metadata(&self.path), err)
    }

    pub fn generate_thumbnail(&self, width: u32, height: u32) -> Result<RgbImage, MetadataError> {
        match_format!(self.format, generate_thumbnail(&self.path, width, height), err)
    }

    pub fn generate_full(&self) -> Result<RgbImage, MetadataError> {
        match_format!(self.format, generate_full(&self.path), err)
    }

    pub fn metadata_version(&self) -> i32 {
        match_format!(self.format, METADATA_VERSION)
    }

    pub fn thumbnail_version(&self) -> i32 {
        match_format!(self.format, THUMBNAIL_VERSION)
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