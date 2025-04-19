use toml::Table;
use common::models::media::Media;
use std::fmt::Debug;
use axum::extract::Request;
use axum::response::Response;
use ::tasks::tasks::BackgroundTask;
use common::types::AcquireClone;
use common::runner_config::RemoteRunnerConfig;
use tasks::tasks::remote::RemoteTest;
use tasks::tasks::RemoteBackgroundTask;

macro_rules! impl_remote_task {
    (
        [$($task: ident,)*], 
        $size: literal
    ) => {
        pub enum RemoteTask {
            $(
                $task($task),
            )*
        }

        impl RemoteTask {
            pub const TASK_NAMES: [&'static str; $size] = [$(<$task>::NAME,)*];

            pub fn name(&self) -> &'static str {
                match self {
                    $(
                        RemoteTask::$task(_) => $task::NAME,
                    )*
                    _ => unreachable!()
                }
            }


            pub fn name_from_str(task: &str) -> Result<&'static str, RemoteTaskError> {
                match task {
                    $(
                        $task::NAME => Ok($task::NAME),
                    )*
                    _ => Err(RemoteTaskError::TaskNotFound(task.to_string())),
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


            pub async fn new(task: &str, db: &mut impl AcquireClone, tasks: &Table, runner_config: &RemoteRunnerConfig) -> Result<Self, RemoteTaskError> {
                match task {
                    $(
                        $task::NAME => {
                            let config: <$task as RemoteBackgroundTask>::RunnerConfig = tasks.get($task::NAME).map(|v| v.clone().try_into()).transpose()?.unwrap_or_default();
                            let task = $task::new_remote(db, &config, &runner_config).await.map_err(|e| RemoteTaskError::TaskError(e.into()))?;
                            Ok(Self::$task(task))
                        }
                    )*
                    _ => Err(RemoteTaskError::TaskNotFound(task.to_string())),
                }
            }
            
            pub async fn remote_handler(&self, request: Request, db: &mut impl AcquireClone, tasks: &Table, runner_config: &RemoteRunnerConfig) -> Result<Response, RemoteTaskError> {
                match self {
                    $(
                        Self::$task(task) => {
                            let config: <$task as RemoteBackgroundTask>::RunnerConfig = tasks.get($task::NAME).map(|v| v.clone().try_into()).transpose()?.unwrap_or_default();
                            let data = task.remote_handler(request, db, &config, &runner_config).await.map_err(|e| RemoteTaskError::TaskError(e.into()))?;
                            Ok(data)
                        }
                    )*
                    _ => unreachable!()
                }
            }
        }
    };
}

impl_remote_task!(
    [RemoteTest,],
    1
);


#[derive(Debug, thiserror::Error)]
pub enum RemoteTaskError {
    #[error("task not found: {0}")]
    TaskNotFound(String),
    #[error("error deserializing task data: {0}")]
    InvalidTaskData(#[from] serde_json::Error),
    #[error("error deserializing task config: {0}")]
    InvalidTaskConfig(#[from] toml::de::Error),
    #[error("task error: {0}")]
    TaskError(#[from] anyhow::Error),
}