pub mod thumbnail;
pub mod whisper;
pub mod ocr;
mod any_task;
pub mod vllm;

use common::models::media::Media;
use common::types::{AcquireClone};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::process::ExitStatus;
use axum::extract::Request;
use axum::http::StatusCode;
use axum::response::{ErrorResponse, IntoResponse, Response};
use serde::de::DeserializeOwned;
use toml::Table;
use common::runner_config::RemoteRunnerGlobalConfig;
use common::scan_config::AppConfig;
use crate::impl_task;
use crate::tasks::thumbnail::ThumbnailGenerator;
use crate::tasks::whisper::Whisper;
use crate::tasks::ocr::VisionOCR;
use crate::tasks::vllm::VLLM;

const MODEL_DIR: &str = "models";

pub trait Task {
    type Error: Debug;
    const NAME: &'static str;
    type Config: Serialize + DeserializeOwned + Default;
}

pub trait RemoteTask {
    // this is the runner (server) configuration
    type RunnerTaskConfig: DeserializeOwned + Default;
    // this is the client configuration
    // contains url, password, etc. to connect to the runner
    type ClientTaskConfig: DeserializeOwned;
}


pub trait CustomTask: Task {
    type Args: DeserializeOwned;
    type Output: Serialize;
    
    // this usually should not be modifying the db
    async fn run_custom(db: &mut impl AcquireClone, config: &Self::Config, app_config: &AppConfig, args: Self::Args) -> Result<Self::Output, Self::Error>;
}

pub trait CustomRemoteTask: CustomTask + RemoteTask {
    /// on success, returns a timeout in seconds for the client to check the status of the job (request) or None if the job is complete
    async fn remote_custom_handler(request: Request, db: impl AcquireClone + Send + 'static, runner_config: &Self::RunnerTaskConfig, remote_server_config: &RemoteRunnerGlobalConfig) -> Result<Response, ErrorResponse>;
    
    async fn run_custom_remote(
        &self,
        db: &mut impl AcquireClone,
        client_config: &Self::ClientTaskConfig,
        app_config: &AppConfig,
        args: Self::Args,
    ) -> Result<(), Self::Error>;
}

pub trait BackgroundTask: Task + Sized {
    type Data: Debug;
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

pub trait RemoteBackgroundTask: RemoteTask + BackgroundTask {

    async fn new_remote(db: &mut impl AcquireClone, runner_config: &Self::RunnerTaskConfig, remote_global_config: &RemoteRunnerGlobalConfig) -> Result<Self, Self::Error>;

    /// on success, returns a timeout in seconds for the client to check the status of the job (request) or None if the job is complete
    async fn remote_handler(&self, request: Request, db: impl AcquireClone + Send + 'static, runner_config: &Self::RunnerTaskConfig, remote_server_config: &RemoteRunnerGlobalConfig) -> Result<Response, ErrorResponse>;

    async fn run_remote(
        &self,
        db: &mut impl AcquireClone,
        media: &Media,
        remote_config: &Self::ClientTaskConfig,
    ) -> Result<Self::Data, Self::Error>;

    async fn run_remote_and_store(
        &self,
        db: &mut impl AcquireClone,
        media: &mut Media,
        remote_config: &Self::ClientTaskConfig,
    ) -> Result<(), Self::Error>;
}

impl_task!(
    @background [ThumbnailGenerator, Whisper, VisionOCR,],
    3,
    @background_remote [VisionOCR, Whisper,],
    @custom [VLLM,],
    @custom_remote []
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
    Infallible,
    #[error("custom task failed with error")]
    CustomTaskError((ExitStatus, Vec<u8>)),
}

impl IntoResponse for TaskError {
    fn into_response(self) -> Response {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("remote task error: {:?}", self)).into_response()
    }
}