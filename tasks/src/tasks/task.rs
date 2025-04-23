use crate::tasks::TaskError;

#[macro_export]
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
                    )*,
                    _ => Err(TaskError::TaskNotFound(task.to_string())),
                }
            }

            pub async fn run(&self, db: &mut impl AcquireClone, media: &Media) -> Result<Box<dyn Debug>, TaskError> {
                match self {
                    $(
                        Task::$task(task) => {
                            task.run(db, media).await.map_err(|e| TaskError::TaskError(e.into())).map(|d| Box::new(d) as Box<dyn Debug>)
                        }
                    )*,
                }
            }

            pub async fn run_and_store(&self, db: &mut impl AcquireClone, media: &mut Media) -> Result<(), TaskError> {
                match self {
                    $(
                        Task::$task(task) => {
                            task.run_and_store(db, media).await.map_err(|e| TaskError::TaskError(e.into()))
                        }
                    )*,
                }
            }

            pub async fn outdated(&self, db: &mut impl AcquireClone, media: &Media) -> Result<bool, TaskError> {
                match self {
                    $(
                        Task::$task(task) => {
                            task.outdated(db, media).await.map_err(|e| TaskError::TaskError(e.into()))
                        }
                    )*,
                }
            }
            
            pub fn remotable(task: &str) -> bool {
                match task {
                    $(
                        <$remote_task as BackgroundTask>::NAME => true,
                    )*
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
                        Task::$remote_task(_) => remote_configs.get($remote_task::NAME).is_some(),
                    )*
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

             pub async fn new_remote(task: &str, db: &mut impl AcquireClone, tasks: &Table, runner_config: &RemoteRunnerConfig) -> Result<Self, TaskError> {
                match task {
                    $(
                        $remote_task::NAME => {
                            let config: <$remote_task as RemoteBackgroundTask>::RunnerConfig = tasks.get($remote_task::NAME).map(|v| v.clone().try_into()).transpose()?.unwrap_or_default();
                            let task = $remote_task::new_remote(db, &config, &runner_config).await.map_err(|e| TaskError::TaskError(e.into()))?;
                            Ok(Self::$remote_task(task))
                        }
                    )*
                    _ => Err(TaskError::TaskNotFound(task.to_string())),
                }
            }

            pub async fn remote_handler(&self, request: Request, db: impl AcquireClone + Send + 'static, tasks: &Table, runner_config: &RemoteRunnerConfig) -> Result<Response, ErrorResponse> {
                match self {
                    $(
                        Self::$remote_task(task) => {
                            let config: <$remote_task as RemoteBackgroundTask>::RunnerConfig = tasks.get($remote_task::NAME).map(|v| v.clone().try_into()).transpose().map_err(|e| TaskError::InvalidTaskConfig(e))?.unwrap_or_default();
                            task.remote_handler(request, db, &config, &runner_config).await
                        }
                    )*
                    _ => unreachable!()
                }
            }
        }
    };
}
