use image::{RgbaImage, RgbImage};
use walkdir::DirEntry;
use common::models::system_time_to_naive_datetime;
use crate::format::{Format, MediaMetadata};

pub struct Standard;

impl Format<StandardError> for Standard {
    const EXTENSIONS: &'static [&'static str] = &["jpeg", "jpg", "png"];

    fn is_photo() -> bool {
        true
    }

    fn is_valid(path: &DirEntry) -> bool {
        true
    }

    fn get_metadata(path: &DirEntry) -> Result<MediaMetadata, StandardError> {
        let file_meta = path.metadata()?;
        let image = image::open(path.path())?;

        Ok(MediaMetadata {
            name: path.file_name().to_string_lossy().to_string(),
            width: image.width(),
            height: image.height(),
            size: file_meta.len() as u32,
            created_at: system_time_to_naive_datetime(file_meta.created().unwrap()),
        })
    }

    fn generate_thumbnail(path: &DirEntry, width: u32, height: u32) -> Result<RgbImage, StandardError>{
        let image = image::open(path.path())?;
        let thumbnail = image.thumbnail(width, height);
        Ok(thumbnail.to_rgb8())
    }
}

#[derive(thiserror::Error, Debug)]
pub enum StandardError {
    #[error("image error: {0}")]
    ImageError(#[from] image::ImageError),
    #[error("iO error: {0}")]
    IoError(#[from] walkdir::Error),
}