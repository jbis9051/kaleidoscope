pub mod thumbnail;
pub mod whisper;
pub mod ocr;
pub mod remote;

use common::models::media::Media;
use common::types::{AcquireClone};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::ops::Deref;
use axum::extract::Request;
use axum::response::{IntoResponse, Response};
use serde::de::DeserializeOwned;
use toml::Table;
use common::runner_config::RemoteRunnerConfig;
use common::scan_config::AppConfig;
use crate::tasks::thumbnail::ThumbnailGenerator;
use crate::tasks::whisper::Whisper;
use crate::tasks::ocr::VisionOCR;
use crate::tasks::remote::RemoteTest;

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
    async fn remote_handler(&self, request: Request, db: &mut impl AcquireClone, runner_config: &Self::RunnerConfig, remote_server_config: &RemoteRunnerConfig) -> Result<Response, Self::Error>;

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


macro_rules! impl_task {
    (
        [$($task: ident,)*],
        $size: literal,
        [$($remote_task: ident,)*]
    ) => {
        pub enum Task {
            $(
                $task($task),
            )*
        }

        impl Task {
            pub const TASK_NAMES: [&'static str; $size] = [$(<$task>::NAME,)*];

            pub fn name(&self) -> &'static str {
                match self {
                    $(
                        Task::$task(_) => $task::NAME,
                    )*
                }
            }


            pub fn name_from_str(task: &str) -> Result<&'static str, TaskError> {
                match task {
                    $(
                        $task::NAME => Ok($task::NAME),
                    )*
                    _ => Err(TaskError::TaskNotFound(task.to_string())),
                }
            }

            pub async fn compatible(task: &str, media: &Media) -> bool {
                match task {
                    $(
                        $task::NAME => $task::compatible(media).await,
                    )*
                    _ => false,
                }
            }


            pub async fn new(task: &str, db: &mut impl AcquireClone, tasks: &Table, app_config: &AppConfig) -> Result<Self, TaskError> {
                match task {
                    $(
                        $task::NAME => {
                            let config: <$task as BackgroundTask>::Config = tasks.get($task::NAME).map(|v| v.clone().try_into()).transpose()?.unwrap_or_default();
                            let task = $task::new(db, &config, &app_config).await.map_err(|e| TaskError::TaskError(e.into()))?;
                            Ok(Self::$task(task))
                        }
                    )*
                    _ => Err(TaskError::TaskNotFound(task.to_string())),
                }
            }

            pub async fn run(&self, db: &mut impl AcquireClone, media: &Media) -> Result<Box<dyn Debug>, TaskError> {
                match self {
                    $(
                        Task::$task(task) => {
                            task.run(db, media).await.map_err(|e| TaskError::TaskError(e.into())).map(|d| Box::new(d) as Box<dyn Debug>)
                        }
                    )*
                }
            }

            pub async fn run_and_store(&self, db: &mut impl AcquireClone, media: &mut Media) -> Result<(), TaskError> {
                match self {
                    $(
                        Task::$task(task) => {
                            task.run_and_store(db, media).await.map_err(|e| TaskError::TaskError(e.into()))
                        }
                    )*
                }
            }

            pub async fn outdated(&self, db: &mut impl AcquireClone, media: &Media) -> Result<bool, TaskError> {
                match self {
                    $(
                        Task::$task(task) => {
                            task.outdated(db, media).await.map_err(|e| TaskError::TaskError(e.into()))
                        }
                    )*
                }
            }
            
            pub async fn remotable(&self) -> bool {
                match self {
                    $(
                        Task::$remote_task(_) => true
                    )*,
                    _ => false,
                }
            }
            
            async fn run_remote(&self, db: &mut impl AcquireClone, media: &Media, remote_configs: &Table) -> Result<Box<dyn Debug>, TaskError> {
                match self {
                    $(
                        Task::$remote_task(task) => {
                            let config: <$remote_task as RemoteBackgroundTask>::RemoteClientConfig = remote_configs.get($remote_task::NAME).map(|v| v.clone().try_into()).transpose()?.expect("remote is not enabled for this task");
                            task.run_remote(db, media, &config).await.map_err(|e| TaskError::TaskError(e.into())).map(|d| Box::new(d) as Box<dyn Debug>)
                        }
                    )*,
                    _ => panic!("not remotable")
                }
            }
            
            async fn run_remote_and_store(&self, db: &mut impl AcquireClone, media: &mut Media, remote_configs: &Table) -> Result<(), TaskError> {
                match self {
                    $(
                        Task::$remote_task(task) => {
                            let config: <$remote_task as RemoteBackgroundTask>::RemoteClientConfig = remote_configs.get($remote_task::NAME).map(|v| v.clone().try_into()).transpose()?.expect("remote is not enabled for this task");
                            task.run_remote_and_store(db, media, &config).await.map_err(|e| TaskError::TaskError(e.into()))
                        }
                    )*,
                    _ => panic!("not remotable")
                }
            }
            
            fn should_remote(&self, remote_configs: &Table) -> bool {
                match self {
                    $(
                        Task::$remote_task(_) => remote_configs.get($remote_task::NAME).is_some()
                    )*,
                    _ => false
                }
            }
            
            pub async fn run_anywhere(&self, db: &mut impl AcquireClone, media: &Media, remote_configs: &Table) -> Result<Box<dyn Debug>, TaskError> {
                if self.should_remote(remote_configs) {
                    self.run_remote(db, media, remote_configs).await
                } else {
                    self.run(db, media).await
                }
            }
            
            pub async fn run_and_store_anywhere(&self, db: &mut impl AcquireClone, media: &mut Media, remote_configs: &Table) -> Result<(), TaskError> {
                if self.should_remote(remote_configs) {
                    self.run_remote_and_store(db, media, remote_configs).await
                } else {
                    self.run_and_store(db, media).await
                }
            }
        }
    };
}


impl_task!(
    [ThumbnailGenerator, Whisper, VisionOCR, RemoteTest,],
    4,
    [RemoteTest,]
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