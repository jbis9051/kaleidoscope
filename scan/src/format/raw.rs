use image::imageops::thumbnail;
use image::RgbImage;
use imagepipe::Pipeline;
use rawloader::RawImageData;
use walkdir::DirEntry;
use common::models::system_time_to_naive_datetime;
use crate::format::{Format, MediaMetadata};

pub struct Raw;

impl Format<RawError> for Raw {
    const EXTENSIONS: &'static [&'static str] = &["raf"];

    fn is_photo() -> bool {
        true
    }

    fn is_valid(path: &DirEntry) -> bool {
        true
    }

    fn get_metadata(entry: &DirEntry) -> Result<MediaMetadata, RawError> {
        let file_meta = entry.metadata()?;

        let native = file_meta.created().unwrap();

        let image = rawloader::decode_file(entry.path())?;

        Ok(MediaMetadata {
            name: entry.file_name().to_string_lossy().to_string(),
            width: image.width as u32,
            height: image.height as u32,
            size: file_meta.len() as u32,
            created_at: system_time_to_naive_datetime(native),
            duration: None,
        })

    }

    fn generate_thumbnail(entry: &DirEntry, width: u32, height: u32) -> Result<RgbImage, RawError> {
        let mut image = Pipeline::new_from_file(entry.path()).map_err(RawError::PipelineError)?;
        let srgb = image.output_8bit(None).map_err(RawError::PipelineError)?;


        let rgb_image = RgbImage::from_raw(srgb.width as u32, srgb.height as u32, srgb.data).unwrap();
        let thumbnail = thumbnail(&rgb_image, width, height);

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
    Io(#[from] walkdir::Error),
}