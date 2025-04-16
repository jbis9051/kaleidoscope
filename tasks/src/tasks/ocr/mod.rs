pub use crate::run_python::run_python;
use crate::tasks::{BackgroundTask, MODEL_DIR};
use common::media_processors::format::{AnyFormat, FormatType, MediaType, MetadataError};
use common::models::media::Media;
use common::scan_config::AppConfig;
use common::types::AcquireClone;
use serde::{Deserialize, Serialize};
use sqlx::types::uuid;
use std::fmt::{Debug, Pointer};
use std::path::{Path, PathBuf};
use uuid::Uuid;
use crate::tasks::ocr::ffi::{vision_ocr, OCRResult};
use crate::tasks::thumbnail::ThumbnailGenerator;

mod ffi;
const VERSION: i32 = 0;

pub struct VisionOCR {
    app_config: AppConfig,
}

impl BackgroundTask for VisionOCR {
    type Error = VisionOCRError;
    const NAME: &'static str = "vision_ocr";

    type Data = Vec<OCRResult>;
    type Config = ();

    async fn new(
        db: &mut impl AcquireClone,
        config: &Self::Config,
        app_config: &AppConfig,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            app_config: app_config.clone(),
        })
    }

    async fn compatible(media: &Media) -> bool {
        let path = PathBuf::from(&media.path);
        let format = AnyFormat::try_new(path);
        if let Some(format) = format {
            if !format.thumbnailable(){
                return false;
            }
            if media.media_type != MediaType::Photo {
                return false;
            }
            return true;
        }
        false
    }

    async fn outdated(
        &self,
        db: &mut impl AcquireClone,
        media: &Media,
    ) -> Result<bool, Self::Error> {
        let extra = media.extra(db.acquire_clone()).await?;
        if let Some(extra) = extra {
            if extra.vision_ocr_version >= VERSION {
                return Ok(false);
            }
        }
        Ok(true)
    }

    async fn run(
        &self,
        db: &mut impl AcquireClone,
        media: &Media,
    ) -> Result<Self::Data, Self::Error> {
        let full_path = ThumbnailGenerator::full_path(media, &self.app_config);
        if !full_path.exists() {
            return Err(VisionOCRError::NoThumbnailFound);
        }
        let result = vision_ocr(full_path.to_str().expect("thumbnail path contains invalid UTF-8"));
        Ok(result)
    }

    async fn run_and_store(
        &self,
        db: &mut impl AcquireClone,
        media: &mut Media,
    ) -> Result<(), Self::Error> {
        let output = self.run(db, media).await?;

        let extra = media.extra(db.acquire_clone()).await?;

        let create = extra.is_none();

        let mut media_extra = extra.unwrap_or_default();

        media_extra.media_id = media.id;
        media_extra.vision_ocr_version = VERSION;
        media_extra.vision_ocr_result = Some(
            serde_json::to_string(&output).map_err(|e| VisionOCRError::OutputParseError(e))?,
        );

        if create {
            media_extra.create_no_bug(db.acquire_clone()).await?;
        } else {
            media_extra.update_by_id(db.acquire_clone()).await?;
        }

        Ok(())
    }

    async fn remove_data(
        &self,
        db: &mut impl AcquireClone,
        media: &mut Media,
    ) -> Result<(), Self::Error> {
        let whisper_extra = media.extra(db.acquire_clone()).await?;
        if let Some(mut whisper_extra) = whisper_extra {
            whisper_extra.vision_ocr_result = None;
            whisper_extra.vision_ocr_version = -1;
            whisper_extra.update_by_id(db.acquire_clone()).await?;
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum VisionOCRError {
    #[error("metadata error: {0}")]
    MetadataError(#[from] MetadataError),
    #[error("no thumbnail full found for media")]
    NoThumbnailFound,
    #[error("sqlx error: {0}")]
    SqlxError(#[from] sqlx::Error),
    #[error("failed to parse OCR output: {0}")]
    OutputParseError(serde_json::Error),
}
