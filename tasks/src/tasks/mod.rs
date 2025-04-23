pub mod thumbnail;
pub mod whisper;
pub mod ocr;
mod task;

use common::models::media::Media;
use common::types::{AcquireClone};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use axum::extract::Request;
use axum::http::StatusCode;
use axum::response::{ErrorResponse, IntoResponse, Response};
use serde::de::DeserializeOwned;
use toml::Table;
use common::runner_config::RemoteRunnerConfig;
use common::scan_config::AppConfig;
use crate::impl_task;
use crate::tasks::thumbnail::ThumbnailGenerator;
use crate::tasks::whisper::Whisper;
use crate::tasks::ocr::VisionOCR;

const MODEL_DIR: &str = "models";

pub trait BackgroundTask: Sized {
    type Error: Debug;

    const NAME: &'static str;
    type Data: Debug;

    type Config: Serialize + DeserializeOwned + Default;

    async fn new(db: &mut impl AcquireClone, config: &Self::Config, app_config: &AppConfig) -> Result<Self, Self::Error>;
    async fn compatible(media: &Media) -> bool; // TODO: this should have error handling
    async fn outdated(
        &self,
        db: &mut impl AcquireClone,
        media: &Media,
    ) -> Result<bool, Self::Error>;

    async fn run(
        &self,
        db: &mut impl AcquireClone,
        media: &Media
    ) -> Result<Self::Data, Self::Error>;

    async fn run_and_store(
        &self,
        db: &mut impl AcquireClone,
        media: &mut Media
    ) -> Result<(), Self::Error>;

    async fn remove_data(&self, db: &mut impl AcquireClone, media: &mut Media) -> Result<(), Self::Error>;
}

pub trait RemoteBackgroundTask: BackgroundTask {
    // this is the client configuration
    type RemoteClientConfig: DeserializeOwned;

    // this is the runner (server) configuration
    type RunnerConfig: DeserializeOwned + Default;
    async fn new_remote(db: &mut impl AcquireClone, runner_config: &Self::RunnerConfig, remote_server_config: &RemoteRunnerConfig) -> Result<Self, Self::Error>;

    /// on success, returns a timeout in seconds for the client to check the status of the job (request) or None if the job is complete
    async fn remote_handler(&self, request: Request, db: impl AcquireClone + Send + 'static, runner_config: &Self::RunnerConfig, remote_server_config: &RemoteRunnerConfig) -> Result<Response, ErrorResponse>;

    async fn run_remote(
        &self,
        db: &mut impl AcquireClone,
        media: &Media,
        remote_config: &Self::RemoteClientConfig,
    ) -> Result<Self::Data, Self::Error>;

    async fn run_remote_and_store(
        &self,
        db: &mut impl AcquireClone,
        media: &mut Media,
        remote_config: &Self::RemoteClientConfig,
    ) -> Result<(), Self::Error>;
}

impl_task!(
    [ThumbnailGenerator, Whisper, VisionOCR,],
    3,
    [VisionOCR, Whisper,]
);

#[derive(Debug, thiserror::Error)]
pub enum TaskError {
    #[error("task not found: {0}")]
    TaskNotFound(String),
    #[error("error deserializing task data: {0}")]
    InvalidTaskData(#[from] serde_json::Error),
    #[error("error deserializing task config: {0}")]
    InvalidTaskConfig(#[from] toml::de::Error),
    #[error("task error: {0}")]
    TaskError(#[from] anyhow::Error),
    #[error("Infallible task failed")]
    Infallible
}

impl IntoResponse for TaskError {
    fn into_response(self) -> Response {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("remote task error: {:?}", self)).into_response()
    }
}