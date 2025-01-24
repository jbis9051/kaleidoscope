use std::path::{Path, PathBuf};
use image::{RgbImage};
use image::imageops::thumbnail;
use libheif_rs::{ColorSpace, HeifContext, LibHeif, RgbChroma};
use walkdir::DirEntry;
use common::models::system_time_to_naive_datetime;
use crate::format::{Format, MediaMetadata, resize_dimensions};

pub struct Heif;

impl Format<HeifError> for Heif {
    const EXTENSIONS: &'static [&'static str] = &["heif", "heic"];
    const METADATA_VERSION: u32 = 0;
    const THUMBNAIL_VERSION: u32 = 0;

    fn is_photo() -> bool {
        true
    }
    
    fn get_metadata(path: &Path) -> Result<MediaMetadata, HeifError> {
        let file_meta = path.metadata()?;

        let path_str = path.to_str().ok_or(HeifError::PathToString(path.to_path_buf()))?;
        let ctx = HeifContext::read_from_file(path_str)?;
        let handle = ctx.primary_image_handle()?;

        let native = file_meta.created().unwrap();

        Ok(MediaMetadata {
            name: path.file_name().unwrap().to_string_lossy().to_string(),
            width: handle.width(),
            height: handle.height(),
            size: file_meta.len() as u32,
            created_at: system_time_to_naive_datetime(native),
            duration: None,
            longitude: None,
            latitude: None,
        })
    }

    fn generate_thumbnail(path: &Path, width: u32, height: u32) -> Result<RgbImage, HeifError> {
        let lib_heif = LibHeif::new();
        let path_str = path.to_str().ok_or(HeifError::PathToString(path.to_path_buf()))?;
        let ctx = HeifContext::read_from_file(path_str)?;
        let handle = ctx.primary_image_handle()?;

        let image = lib_heif.decode(&handle, ColorSpace::Rgb(RgbChroma::Rgb), None)?;

        let planes = image.planes();
        let interleaved = planes.interleaved.unwrap();

        let mut rgb = Vec::new();
        let stride = interleaved.stride;
        let data = interleaved.data;

        for i in 0..interleaved.height as usize {
            let start = i * stride;
            let end = start + (interleaved.width * 3) as usize;
            rgb.extend_from_slice(&data[start..end]);
        }


        let rgb_image = RgbImage::from_raw(image.width(), image.height(), rgb).unwrap();
        let (nw, nh) = resize_dimensions(image.width(), image.height(), width, height, false);
        let thumbnail = thumbnail(&rgb_image, nw, nh);

        Ok(thumbnail)
    }
}

#[derive(thiserror::Error, Debug)]
pub enum HeifError {
    #[error("cannot convert path to string: {0}")]
    PathToString(PathBuf),
    #[error("image error: {0}")]
    Heif(#[from] libheif_rs::HeifError),
    #[error("iO error: {0}")]
    Io(#[from] std::io::Error),
}