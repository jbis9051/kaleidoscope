use image::{RgbImage};
use std::path::Path;
use serde::{Deserialize, Serialize};
use crate::media_processors::format::{resize_dimensions, Format, FormatType, MediaMetadata, MediaType, Thumbnailable};
use crate::models::system_time_to_naive_datetime;
use crate::scan_config::AppConfig;
use once_cell::sync::OnceCell;
use pdfium_render::prelude::*;

const FULL_SIZE: u32 = 1920;

static PDFIUM: OnceCell<Pdfium> = OnceCell::new();

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PdfConfig {
    pub pdfium_path: String,
}
pub struct Pdf;

impl Pdf {
    pub fn get_pdfium(pdfium_path: &str) -> &Pdfium {
        PDFIUM.get_or_init(|| { Pdfium::new(Pdfium::bind_to_library(pdfium_path).expect("could not init pdfium")) })
    }
}

impl Format for Pdf {
    type Error = PdfError;
    const FORMAT_TYPE: FormatType = FormatType::Pdf;
    const EXTENSIONS: &'static [&'static str] = &["pdf"];
    const METADATA_VERSION: i32 = 1;

    fn get_metadata(path: &Path, _: &AppConfig) -> Result<MediaMetadata, Self::Error> {
        let file_meta = path.metadata()?;

        Ok(MediaMetadata {
            name: path.file_name().unwrap().to_string_lossy().to_string(),
            width: 0,
            height: 0,
            size: file_meta.len() as u32,
            created_at: system_time_to_naive_datetime(file_meta.created().unwrap()),
            duration: None,
            longitude: None,
            latitude: None,
            is_screenshot: false,
            media_type: MediaType::Pdf,
        })
    }


}
impl Thumbnailable for Pdf {
    const THUMBNAIL_VERSION: i32 = 0;

    fn generate_thumbnail(path: &Path, width: u32, height: u32, app_config: &AppConfig) -> Result<RgbImage, Self::Error> {
        let pdfium = Self::get_pdfium(app_config.formats.pdf.pdfium_path.as_str());
        let document = pdfium.load_pdf_from_file(path, None)?;
        let page = document.pages().get(0)?;
        
        let (nw, nh) = resize_dimensions(page.width().value as u32, page.height().value as u32, width, height, false);

        let image = page.render(nw as i32, nh as i32, None)?;
        Ok(image.as_image().to_rgb8())
    }

    fn generate_full(path: &Path, app_config: &AppConfig) -> Result<RgbImage, Self::Error> {
        let pdfium = Self::get_pdfium(app_config.formats.pdf.pdfium_path.as_str());
        let document = pdfium.load_pdf_from_file(path, None)?;
        let page = document.pages().get(0)?;

        let (nw, nh) = resize_dimensions(page.width().value as u32, page.height().value as u32, FULL_SIZE, FULL_SIZE, false);

        let image = page.render(nw as i32, nh as i32, None)?;

        Ok(image.as_image().to_rgb8())
    }

}

#[derive(thiserror::Error, Debug)]
pub enum PdfError {
    #[error("iO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("pdfium error: {0}")]
    PdfiumError(#[from] PdfiumError),
}
