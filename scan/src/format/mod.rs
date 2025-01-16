pub mod standard;
pub mod heif;
pub mod video;
pub mod raw;

use std::cmp::max;
use std::path::Path;
use std::time::Duration;
use image::{RgbImage};
use sqlx::types::chrono;
use walkdir::DirEntry;

#[derive(Debug)]
pub struct MediaMetadata {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub size: u32,
    pub created_at: chrono::NaiveDateTime,
    pub duration: Option<Duration>,
}

pub trait Format<T> {
    const EXTENSIONS: &'static [&'static str];

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