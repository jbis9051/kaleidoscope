pub mod hello_world;

use common::models::date;
use common::models::media::Media;
use common::question_marks;
use common::sqlize;
use common::types::{SqliteAcquire};
use common::update_set;
use serde::{Serialize};
use sqlx::sqlite::SqliteRow;
use sqlx::{Row, SqliteExecutor};
use std::borrow::Borrow;
use std::fmt::Debug;
use serde::de::DeserializeOwned;
use toml::Table;

pub trait BackgroundTask: Sized {
    type Error: Debug;

    const NAME: &'static str;
    const VERSION: u32;

    type Data: Debug;

    type Config: Serialize + DeserializeOwned + Default;

    async fn new(db: impl SqliteAcquire<'_>, config: &Self::Config) -> Result<Self, Self::Error>;
    async fn compatible(media: &Media) -> bool;
    async fn needs_update(
        &self,
        db: impl SqliteAcquire<'_>,
        media: &Media,
    ) -> bool;


    async fn run(
        &self,
        db: impl SqliteAcquire<'_>,
        media: &Media
    ) -> Result<Self::Data, Self::Error>;

    async fn run_and_store(
        &self,
        db: impl SqliteAcquire<'_>,
        media: &Media
    ) -> Result<(), Self::Error>;

    async fn remove_data(&self, db: impl SqliteAcquire<'_>, media: &Media) -> Result<(), Self::Error>;
}

#[derive(Debug, Serialize)]
pub struct BackgroundTaskDataRaw {
    pub id: i32,
    pub media_id: i32,
    pub task: String,
    pub version: u32,
    pub data: String,
    #[serde(with = "date")]
    pub created_at: chrono::NaiveDateTime,
    #[serde(with = "date")]
    pub updated_at: chrono::NaiveDateTime,
}

sqlize!(
    BackgroundTaskDataRaw,
    "background_task_data",
    id,
    [media_id, task, version, data, created_at, updated_at]
);




use hello_world::*;

macro_rules! match_task {
    ($task: expr, $call: tt($($arg: expr),*)) => {
        match $task {
            Task::HelloWorld => HelloWorldTask::$call($($arg),*).into(),
            Task::HelloWorld2 => HelloWorldTask2::$call($($arg),*).into(),
        }
    };
    ($task: expr, $call: tt($($arg: expr),*), err) => {
        match $task {
            Task::HelloWorld => HelloWorldTask::$call($($arg),*).map_err(|e| e.into()),
            Task::HelloWorld2 => HelloWorldTask2::$call($($arg),*).map_err(|e| e.into()),
        }
    };
    ($task: expr, $assoc: ident) => {
        match $task {
            Task::HelloWorld => HelloWorldTask::$assoc,
            Task::HelloWorld2 => HelloWorldTask2::$assoc,
        }
    };
}

macro_rules! impl_task {
    ([$($task: ident,)*], $size: literal) => {
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


            pub async fn new(task: &str, db: impl SqliteAcquire<'_>, tasks: &Table) -> Result<Self, TaskError> {
                match task {
                    $(
                        $task::NAME => {
                            let config: <$task as BackgroundTask>::Config = tasks.get($task::NAME).map(|v| v.clone().try_into()).transpose()?.unwrap_or_default();
                            let task = $task::new(db, &config).await.map_err(|e| TaskError::$task(e))?;
                            Ok(Self::$task(task))
                        }
                    )*
                    _ => Err(TaskError::TaskNotFound(task.to_string())),
                }
            }

            pub async fn run(&self, db: impl SqliteAcquire<'_>, media: &Media) -> Result<Box<dyn Debug>, TaskError> {
                match self {
                    $(
                        Task::$task(task) => {
                            task.run(db, media).await.map_err(|e| TaskError::$task(e)).map(|d| Box::new(d) as Box<dyn Debug>)
                        }
                    )*
                }
            }

            pub async fn run_and_store(&self, db: impl SqliteAcquire<'_>, media: &Media) -> Result<(), TaskError> {
                match self {
                    $(
                        Task::$task(task) => {
                            task.run_and_store(db, media).await.map_err(|e| TaskError::$task(e))
                        }
                    )*
                }
            }
        }
    };
}


impl_task!(
    [VideoDurationProcessor,],
    1
);

#[derive(Debug, thiserror::Error)]
pub enum TaskError {
    #[error("task not found: {0}")]
    TaskNotFound(String),
    #[error("hello world task error: {0}")]
    VideoDurationProcessor(String),
    #[error("error deserializing task data: {0}")]
    InvalidTaskData(#[from] serde_json::Error),
    #[error("error deserializing task config: {0}")]
    InvalidTaskConfig(#[from] toml::de::Error),
}