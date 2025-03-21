use std::path::{Path, PathBuf};
use image::{RgbImage};
use image::imageops::thumbnail;
use libheif_rs::{ColorSpace, HeifContext, ItemId, LibHeif, RgbChroma};
use crate::media_processors::exif::extract_exif;
use crate::media_processors::format::{resize_dimensions, Format, MediaMetadata, Thumbnailable};
use crate::models::system_time_to_naive_datetime;

pub struct Heif;

impl Format<HeifError> for Heif {
    const EXTENSIONS: &'static [&'static str] = &["heif", "heic"];
    const METADATA_VERSION: i32 = 1;
    fn is_photo() -> bool {
        true
    }

    fn get_metadata(path: &Path) -> Result<MediaMetadata, HeifError> {
        let file_meta = path.metadata()?;

        let path_str = path.to_str().ok_or(HeifError::PathToString(path.to_path_buf()))?;
        let ctx = HeifContext::read_from_file(path_str)?;
        let handle = ctx.primary_image_handle()?;

        let mut meta_ids: Vec<ItemId> = vec![0; 1];
        let count = handle.metadata_block_ids(&mut meta_ids, b"Exif");

        let exif_metadata = if count == 0 {
            None
        } else {
            let metadata = handle.metadata(meta_ids[0])?;

            // heic has some funky offset stuff, see here https://github.com/ImageMagick/ImageMagick/commit/bb4018a4dc61147b37d3c42d85e5893ca5e2a279#diff-cf133db60a54549531dbba5cb2d17dc34f7171cabd115ec7c85c6d3f1e84fb2b

            let mut offset = 0;
            offset |= (metadata[0] as u32) << 24;
            offset |= (metadata[1] as u32) << 16;
            offset |= (metadata[2] as u32) << 8;
            offset |= metadata[3] as u32;
            offset += 4;

            let metadata = metadata[offset as usize..].to_vec();
            let reader = exif::Reader::new();
            let exif = reader.read_raw(metadata)?;
            extract_exif(&exif).ok()
        };

        let native = file_meta.created().unwrap();

        Ok(MediaMetadata {
            name: path.file_name().unwrap().to_string_lossy().to_string(),
            width: handle.width(),
            height: handle.height(),
            size: file_meta.len() as u32,
            created_at: system_time_to_naive_datetime(native),
            duration: None,
            longitude: exif_metadata.as_ref().and_then(|e| e.longitude),
            latitude: exif_metadata.as_ref().and_then(|e| e.latitude),
            is_screenshot: exif_metadata.as_ref().map(|e| e.is_screenshot).unwrap_or(false),
        })
    }
}

impl Thumbnailable<HeifError> for Heif {
    const THUMBNAIL_VERSION: i32 = 0;

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
    #[error("exif error: {0}")]
    Exif(#[from] exif::Error),
}