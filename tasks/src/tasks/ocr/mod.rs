pub use crate::run_python::run_python;
use crate::tasks::thumbnail::ThumbnailGenerator;
use crate::tasks::{BackgroundTask, RemoteBackgroundTask, RemoteTask, Task};
use axum::extract::{Request};
use axum::response::{ErrorResponse, IntoResponse, Response};
use axum::{Json, RequestExt};
use common::media_processors::format::{AnyFormat, MediaType, MetadataError};
use common::models::media::Media;
use common::runner_config::RemoteRunnerGlobalConfig;
use common::scan_config::AppConfig;
use common::types::AcquireClone;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Pointer};
use std::path::{PathBuf};
use futures::TryFutureExt;
use reqwest::multipart::Form;
use tokio::fs;

mod ffi;

use crate::remote_utils::multipart_helper::MultipartHelper;
use crate::remote_utils::{internal, StandardClientConfig};
pub use ffi::vision_ocr;
pub use ffi::OCRResult;
use crate::remote_utils::remote_requester::{OneShotResponse, RemoteRequester, RequestError};

const VERSION: i32 = 0;

pub struct VisionOCR {
    app_config: AppConfig,
}

impl VisionOCR {
    pub fn run_on_path(
        image_path: &str,
    ) -> Result<<VisionOCR as BackgroundTask>::Data, <VisionOCR as Task>::Error> {
        Ok(vision_ocr(image_path))
    }

    pub async fn store(
        db: &mut impl AcquireClone,
        media: &mut Media,
        output: <VisionOCR as BackgroundTask>::Data,
    ) -> Result<(), <VisionOCR as Task>::Error> {
        let extra = media.extra(db.acquire_clone()).await?;

        let create = extra.is_none();

        let mut media_extra = extra.unwrap_or_default();

        media_extra.media_id = media.id;
        media_extra.vision_ocr_version = VERSION;
        media_extra.vision_ocr_result =
            Some(serde_json::to_string(&output).map_err(|e| VisionOCRError::OutputParseError(e))?);

        if create {
            media_extra.create_no_bug(db.acquire_clone()).await?;
        } else {
            media_extra.update_by_id(db.acquire_clone()).await?;
        }

        Ok(())
    }
}

impl Task for VisionOCR {
    type Error = VisionOCRError;
    const NAME: &'static str = "vision_ocr";
    type Config = ();
}

impl RemoteTask for VisionOCR {
    type ClientTaskConfig = StandardClientConfig;

    type RunnerTaskConfig = bool;
}


impl BackgroundTask for VisionOCR {
    type Data = Vec<OCRResult>;

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
            if !format.thumbnailable() {
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
        let result = Self::run_on_path(
            full_path
                .to_str()
                .expect("thumbnail path contains invalid UTF-8"),
        )?;
        Ok(result)
    }

    async fn run_and_store(
        &self,
        db: &mut impl AcquireClone,
        media: &mut Media,
    ) -> Result<(), Self::Error> {
        let output = self.run(db, media).await?;
        Self::store(db, media, output).await
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

impl RemoteBackgroundTask for VisionOCR {

    async fn new_remote(
        db: &mut impl AcquireClone,
        runner_config: &Self::RunnerTaskConfig,
        remote_server_config: &RemoteRunnerGlobalConfig,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            app_config: Default::default(), // create a default app_config that we won't use
        })
    }

    async fn remote_handler(
        &self,
        request: Request,
        db: impl AcquireClone,
        runner_config: &Self::RunnerTaskConfig,
        remote_server_config: &RemoteRunnerGlobalConfig,
    ) -> Result<Response, ErrorResponse> {
        let mut multipart = MultipartHelper::try_from_request(request).await?;

        let (image_file, _) = multipart.file("image", ".jpg").await?;

        let result = Self::run_on_path(
            image_file
                .to_str()
                .expect("image file contains invalid UTF-8"),
        )
        .map_err(internal)?;
        
        fs::remove_file(image_file).await.map_err(internal)?;

        let response: Json<Vec<OCRResult>> = result.into();

        Ok(response.into_response())
    }

    async fn run_remote(
        &self,
        db: &mut impl AcquireClone,
        media: &Media,
        remote_config: &Self::ClientTaskConfig,
    ) -> Result<Self::Data, Self::Error> {
        let full_path = ThumbnailGenerator::full_path(media, &self.app_config);
        if !full_path.exists() {
            return Err(VisionOCRError::NoThumbnailFound);
        }
        let client = RemoteRequester::new(Self::NAME.to_string(), remote_config.remote.url.clone(), remote_config.remote.password.clone(), true);
        let res = client.one_shot_file("image".to_string(), &full_path, None).await?;
        if let OneShotResponse::Response(res) = res {
            let data = res.json().await?;
            return Ok(data);
        }
        panic!("expected a response not a job: {:?}", res)
    }

    async fn run_remote_and_store(
        &self,
        db: &mut impl AcquireClone,
        media: &mut Media,
        remote_config: &Self::ClientTaskConfig,
    ) -> Result<(), Self::Error> {
        let data = self.run_remote(db, media, remote_config).await?;
        Self::store(db, media, data).await
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
    #[error("io error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("reqwest error: {0}")]
    ReqwestError(#[from] reqwest::Error),
    #[error("request error: {0}")]
    RequestError(#[from] RequestError),
}
