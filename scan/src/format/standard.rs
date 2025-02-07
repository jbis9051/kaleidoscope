use crate::format::{Format, MediaMetadata};
use common::models::system_time_to_naive_datetime;
use exif::{Error, Exif, Tag, Value};
use image::{RgbImage};
use std::path::Path;
use crate::exif::extract_exif;

pub struct Standard;

impl Format<StandardError> for Standard {
    const EXTENSIONS: &'static [&'static str] = &["jpeg", "jpg", "png"];
    const METADATA_VERSION: i32 = 1;
    const THUMBNAIL_VERSION: i32 = 0;

    fn is_photo() -> bool {
        true
    }

    fn get_metadata(path: &Path) -> Result<MediaMetadata, StandardError> {
        let file_meta = path.metadata()?;
        let (width, height) = image::image_dimensions(path)?;

        let file = std::fs::File::open(path)?;
        let mut bufreader = std::io::BufReader::new(&file);
        let exifreader = exif::Reader::new();

        let exif_metadata = exifreader.read_from_container(&mut bufreader).ok().and_then(|e| extract_exif(&e).ok());
        
        Ok(MediaMetadata {
            name: path.file_name().unwrap().to_string_lossy().to_string(),
            width,
            height,
            size: file_meta.len() as u32,
            created_at: system_time_to_naive_datetime(file_meta.created().unwrap()),
            duration: None,
            longitude: exif_metadata.as_ref().and_then(|e| e.longitude),
            latitude: exif_metadata.as_ref().and_then(|e| e.latitude),
            is_screenshot: exif_metadata.as_ref().map(|e| e.is_screenshot).unwrap_or(false),
        })
    }

    fn generate_thumbnail(path: &Path, width: u32, height: u32) -> Result<RgbImage, StandardError> {
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
    #[error("exif error: {0}")]
    ExifError(#[from] exif::Error),
}
