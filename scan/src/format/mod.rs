pub mod standard;
pub mod heif;
pub mod video;
pub mod raw;

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

    fn is_supported(path: &DirEntry) -> bool {
        let path = path.path();
        let ext = path.extension().unwrap_or_default().to_str().unwrap_or_default().to_lowercase();
        Self::EXTENSIONS.contains(&ext.as_str())
    }

    fn is_photo() -> bool;

    fn is_valid(path: &DirEntry) -> bool;

    fn get_metadata(entry: &DirEntry) -> Result<MediaMetadata, T>;

    fn generate_thumbnail(entry: &DirEntry, width: u32, height: u32) -> Result<RgbImage, T>;

    fn generate_full(entry: &DirEntry) -> Result<RgbImage, T> {
        let metadata = Self::get_metadata(entry)?;
        let width = metadata.width;
        let height = metadata.height;
        Self::generate_thumbnail(entry, width, height)
    }
}