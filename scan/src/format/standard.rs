use std::path::Path;
use image::{RgbaImage, RgbImage};
use walkdir::DirEntry;
use common::models::system_time_to_naive_datetime;
use crate::format::{Format, MediaMetadata};

pub struct Standard;

impl Format<StandardError> for Standard {
    const EXTENSIONS: &'static [&'static str] = &["jpeg", "jpg", "png"];
    const METADATA_VERSION: i32 = 0;
    const THUMBNAIL_VERSION: i32 = 0;

    fn is_photo() -> bool {
        true
    }
    
    fn get_metadata(path: &Path) -> Result<MediaMetadata, StandardError> {
        let file_meta = path.metadata()?;
        let (width, height) = image::image_dimensions(path)?;

        Ok(MediaMetadata {
            name: path.file_name().unwrap().to_string_lossy().to_string(),
            width,
            height,
            size: file_meta.len() as u32,
            created_at: system_time_to_naive_datetime(file_meta.created().unwrap()),
            duration: None,
            longitude: None,
            latitude: None,
            is_screenshot: false,
        })
    }

    fn generate_thumbnail(path: &Path, width: u32, height: u32) -> Result<RgbImage, StandardError>{
        let image = image::open(path)?;
        let thumbnail = image.thumbnail(width, height);
        Ok(thumbnail.to_rgb8())
    }
}

#[derive(thiserror::Error, Debug)]
pub enum StandardError {
    #[error("image error: {0}")]
    ImageError(#[from] image::ImageError),
    #[error("iO error: {0}")]
    IoError(#[from] std::io::Error),
}