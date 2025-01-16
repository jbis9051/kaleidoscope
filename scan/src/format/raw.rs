use std::path::Path;
use image::imageops::thumbnail;
use image::RgbImage;
use imagepipe::Pipeline;
use walkdir::DirEntry;
use common::models::system_time_to_naive_datetime;
use crate::format::{Format, MediaMetadata, resize_dimensions};

pub struct Raw;

impl Format<RawError> for Raw {
    const EXTENSIONS: &'static [&'static str] = &["raf"];

    fn is_photo() -> bool {
        true
    }
    
    fn get_metadata(path: &Path) -> Result<MediaMetadata, RawError> {
        let file_meta = path.metadata()?;

        let native = file_meta.created().unwrap();

        let image = rawloader::decode_file(path)?;

        Ok(MediaMetadata {
            name: path.file_name().unwrap().to_string_lossy().to_string(),
            width: image.width as u32,
            height: image.height as u32,
            size: file_meta.len() as u32,
            created_at: system_time_to_naive_datetime(native),
            duration: None,
        })

    }

    fn generate_thumbnail(path: &Path, width: u32, height: u32) -> Result<RgbImage, RawError> {
        let mut image = Pipeline::new_from_file(path).map_err(RawError::PipelineError)?;
        let srgb = image.output_8bit(None).map_err(RawError::PipelineError)?;


        let rgb_image = RgbImage::from_raw(srgb.width as u32, srgb.height as u32, srgb.data).unwrap();
        let (nw, nh) = resize_dimensions(srgb.width as u32, srgb.height as u32, width, height, false);
        let thumbnail = thumbnail(&rgb_image, nw, nh);

        Ok(thumbnail)
    }

}

#[derive(thiserror::Error, Debug)]
pub enum RawError {
    #[error("pipeine error: {0}")]
    PipelineError(String),
    #[error("raw loader error: {0}")]
    RawLoader(#[from] rawloader::RawLoaderError),
    #[error("iO error: {0}")]
    Io(#[from] std::io::Error),
}