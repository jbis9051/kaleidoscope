pub mod thumbnail;
pub mod whisper;
pub mod ocr;

use common::models::media::Media;
use common::types::{AcquireClone};
use serde::{Serialize};
use std::fmt::Debug;
use std::ops::Deref;
use serde::de::DeserializeOwned;
use toml::Table;
use common::scan_config::AppConfig;
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

macro_rules! impl_task {
    (
        [$($task: ident,)*], 
        $size: literal
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
                            let task = $task::new(db, &config, &app_config).await.map_err(|e| TaskError::$task(e))?;
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
                            task.run(db, media).await.map_err(|e| TaskError::$task(e)).map(|d| Box::new(d) as Box<dyn Debug>)
                        }
                    )*
                }
            }

            pub async fn run_and_store(&self, db: &mut impl AcquireClone, media: &mut Media) -> Result<(), TaskError> {
                match self {
                    $(
                        Task::$task(task) => {
                            task.run_and_store(db, media).await.map_err(|e| TaskError::$task(e))
                        }
                    )*
                }
            }

            pub async fn outdated(&self, db: &mut impl AcquireClone, media: &Media) -> Result<bool, TaskError> {
                match self {
                    $(
                        Task::$task(task) => {
                            task.outdated(db, media).await.map_err(|e| TaskError::$task(e))
                        }
                    )*
                }
            }
        }
    };
}


impl_task!(
    [ThumbnailGenerator, Whisper, VisionOCR,],
    3
);

#[derive(Debug, thiserror::Error)]
pub enum TaskError {
    #[error("task not found: {0}")]
    TaskNotFound(String),
    #[error("'thumbnail' task error: {0}")]
    ThumbnailGenerator(<ThumbnailGenerator as BackgroundTask>::Error),
    #[error("'whisper' task error: {0}")]
    Whisper(<Whisper as BackgroundTask>::Error),
    #[error("'vision_ocr' task error: {0}")]
    VisionOCR(<VisionOCR as BackgroundTask>::Error),
    #[error("error deserializing task data: {0}")]
    InvalidTaskData(#[from] serde_json::Error),
    #[error("error deserializing task config: {0}")]
    InvalidTaskConfig(#[from] toml::de::Error),
}