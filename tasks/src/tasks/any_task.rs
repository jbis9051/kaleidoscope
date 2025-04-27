use crate::tasks::RemoteTask;

#[macro_export]
macro_rules! impl_task {
    (
        @background [$($background_task: ident,)*],
        $size: literal,
        @background_remote [$($background_remote_task: ident,)*],
        @custom [$($custom_task: ident,)*],
        @custom_remote [$($custom_remote_task: ident,)*]
    ) => {
        pub enum AnyTask {
            $(
                $background_task($background_task),
            )*
        }

        // TODO: this is some actual garbage fix it at some point
        impl AnyTask {
            pub const BACKGROUND_TASK_NAMES: [&'static str; $size] = [$(<$background_task>::NAME,)*];

            pub fn name(&self) -> &'static str {
                match self {
                    $(
                        AnyTask::$background_task(_) => $background_task::NAME,
                    )*
                }
            }


            pub fn name_from_str(task: &str) -> Result<&'static str, TaskError> {
                match task {
                    $(
                        $background_task::NAME => Ok($background_task::NAME),
                    )*
                    _ => Err(TaskError::TaskNotFound(task.to_string())),
                }
            }

            pub async fn compatible(task: &str, media: &Media) -> bool {
                match task {
                    $(
                        $background_task::NAME => $background_task::compatible(media).await,
                    )*
                    _ => false,
                }
            }


            pub async fn new(task: &str, db: &mut impl AcquireClone, tasks: &Table, app_config: &AppConfig) -> Result<Self, TaskError> {
                match task {
                    $(
                        $background_task::NAME => {
                            let config: <$background_task as Task>::Config = tasks.get($background_task::NAME).map(|v| v.clone().try_into()).transpose()?.unwrap_or_default();
                            let task = $background_task::new(db, &config, &app_config).await.map_err(|e| TaskError::TaskError(e.into()))?;
                            Ok(Self::$background_task(task))
                        }
                    )*,
                    _ => Err(TaskError::TaskNotFound(task.to_string())),
                }
            }

            pub async fn run(&self, db: &mut impl AcquireClone, media: &Media) -> Result<Box<dyn Debug>, TaskError> {
                match self {
                    $(
                        AnyTask::$background_task(task) => {
                            task.run(db, media).await.map_err(|e| TaskError::TaskError(e.into())).map(|d| Box::new(d) as Box<dyn Debug>)
                        }
                    )*,
                }
            }

            pub async fn run_and_store(&self, db: &mut impl AcquireClone, media: &mut Media) -> Result<(), TaskError> {
                match self {
                    $(
                        AnyTask::$background_task(task) => {
                            task.run_and_store(db, media).await.map_err(|e| TaskError::TaskError(e.into()))
                        }
                    )*,
                }
            }

            pub async fn outdated(&self, db: &mut impl AcquireClone, media: &Media) -> Result<bool, TaskError> {
                match self {
                    $(
                        AnyTask::$background_task(task) => {
                            task.outdated(db, media).await.map_err(|e| TaskError::TaskError(e.into()))
                        }
                    )*,
                }
            }
            
            pub fn background_remotable(task: &str) -> bool {
                match task {
                    $(
                        <$background_remote_task as Task>::NAME => true,
                    )*
                    _ => false,
                }
            }
            
            async fn run_remote(&self, db: &mut impl AcquireClone, media: &Media, remote_configs: &Table) -> Result<Box<dyn Debug>, TaskError> {
                match self {
                    $(
                        AnyTask::$background_remote_task(task) => {
                            let config: <$background_remote_task as RemoteTask>::ClientTaskConfig = remote_configs.get($background_remote_task::NAME).map(|v| v.clone().try_into()).transpose()?.expect("remote is not enabled for this task");
                            task.run_remote(db, media, &config).await.map_err(|e| TaskError::TaskError(e.into())).map(|d| Box::new(d) as Box<dyn Debug>)
                        }
                    )*,
                    _ => panic!("not remotable")
                }
            }
            
            async fn run_remote_and_store(&self, db: &mut impl AcquireClone, media: &mut Media, remote_configs: &Table) -> Result<(), TaskError> {
                match self {
                    $(
                        AnyTask::$background_remote_task(task) => {
                            let config: <$background_remote_task as RemoteTask>::ClientTaskConfig = remote_configs.get($background_remote_task::NAME).map(|v| v.clone().try_into()).transpose()?.expect("remote is not enabled for this task");
                            task.run_remote_and_store(db, media, &config).await.map_err(|e| TaskError::TaskError(e.into()))
                        }
                    )*,
                    _ => panic!("not remotable")
                }
            }
            
            fn should_remote(task: &str, remote_configs: &Table) -> bool {
                if !Self::background_remotable(task) && !Self::custom_remotable(task){
                    return false;
                }
                remote_configs.get(task).is_some()
            }
            
            pub async fn run_anywhere(&self, db: &mut impl AcquireClone, media: &Media, remote_configs: &Table) -> Result<Box<dyn Debug>, TaskError> {
                if Self::should_remote(self.name(), remote_configs) {
                    self.run_remote(db, media, remote_configs).await
                } else {
                    self.run(db, media).await
                }
            }
            
            pub async fn run_and_store_anywhere(&self, db: &mut impl AcquireClone, media: &mut Media, remote_configs: &Table) -> Result<(), TaskError> {
                if Self::should_remote(self.name(), remote_configs) {
                    self.run_remote_and_store(db, media, remote_configs).await
                } else {
                    self.run_and_store(db, media).await
                }
            }

             pub async fn new_remote(task: &str, db: &mut impl AcquireClone, tasks: &Table, runner_config: &RemoteRunnerGlobalConfig) -> Result<Self, TaskError> {
                match task {
                    $(
                        $background_remote_task::NAME => {
                            let config: <$background_remote_task as RemoteTask>::RunnerTaskConfig = tasks.get($background_remote_task::NAME).map(|v| v.clone().try_into()).transpose()?.unwrap_or_default();
                            let task = $background_remote_task::new_remote(db, &config, &runner_config).await.map_err(|e| TaskError::TaskError(e.into()))?;
                            Ok(Self::$background_remote_task(task))
                        }
                    )*
                    _ => Err(TaskError::TaskNotFound(task.to_string())),
                }
            }

            pub async fn remote_handler(&self, request: Request, db: impl AcquireClone + Send + 'static, tasks: &Table, runner_config: &RemoteRunnerGlobalConfig) -> Result<Response, ErrorResponse> {
                match self {
                    $(
                        Self::$background_remote_task(task) => {
                            let config: <$background_remote_task as RemoteTask>::RunnerTaskConfig = tasks.get($background_remote_task::NAME).map(|v| v.clone().try_into()).transpose().map_err(|e| TaskError::InvalidTaskConfig(e))?.unwrap_or_default();
                            task.remote_handler(request, db, &config, &runner_config).await
                        }
                    )*
                    _ => unreachable!()
                }
            }
            
            pub fn customable(task: &str) -> bool {
                match task {
                    $(
                        <$custom_task as Task>::NAME => true,
                    )*
                    _ => false,
                }
            }
            
             pub fn custom_remotable(task: &str) -> bool {
                match task {
                    $(
                        <$custom_remote_task as Task>::NAME => true,
                    )*
                    _ => false,
                }
            }
            
            pub async fn run_custom_anywhere(task: &str, db: &mut impl AcquireClone, remote_configs: &Table, app_config: &AppConfig, args_str: &str) -> Result<String, TaskError> {
                if Self::should_remote(task, remote_configs) {
                    Self::run_custom_remote(task, db, remote_configs, app_config, args_str).await
                } else {
                    Self::run_custom(task, db, &app_config.tasks, app_config, args_str).await
                }
            }
            
            
            // TODO: Value instead of &str?
            pub async fn run_custom(task: &str, db: &mut impl AcquireClone, tasks: &Table, app_config: &AppConfig, args_str: &str) -> Result<String, TaskError> {
                match task {
                    $(<$custom_task as Task>::NAME => { 
                        let config: <$custom_task as Task>::Config = tasks.get($custom_task::NAME).map(|v| v.clone().try_into()).transpose()?.unwrap_or_default();
                        let args: <$custom_task as CustomTask>::Args = serde_json::from_str(&args_str).unwrap();
                        let res = <$custom_task as CustomTask>::run_custom(db, &config, app_config, args).await.map_err(|e| TaskError::TaskError(e.into()))?;
                        Ok(serde_json::to_string(&res).expect("failed to serialize custom task response"))
                    })*
                    _ => unreachable!()
                }
            }
            
            pub async fn run_custom_remote(task: &str, db: &mut impl AcquireClone, remote_configs: &Table, app_config: &AppConfig, args_str: &str) -> Result<String, TaskError> {
                match task {
                    $(<$custom_remote_task as Task>::NAME => { 
                        let config: <$custom_remote_task as RemoteTask>::ClientTaskConfig = remote_configs.get($custom_remote_task::NAME).map(|v| v.clone().try_into()).transpose()?.expect("no remote config found");
                        let args: <$custom_remote_task as CustomTask>::Args = serde_json::from_str(&args_str).unwrap();
                        let res = <$custom_remote_task as CustomRemoteTask>::run_custom_remote(db, &config, app_config, args).await.map_err(|e| TaskError::TaskError(e.into()))?;
                        Ok(serde_json::to_string(&res).expect("failed to serialize custom task response"))
                    })*
                    _ => unreachable!()
                }
            }

            pub async fn remote_custom_handler(task: &str, request: Request, db: impl AcquireClone + Send + 'static, tasks: &Table, runner_config: &RemoteRunnerGlobalConfig) -> Result<Response, ErrorResponse> {
                match task {
                    $(<$custom_remote_task as Task>::NAME => { 
                            let config: <$custom_remote_task as RemoteTask>::RunnerTaskConfig = tasks.get($custom_remote_task::NAME).map(|v| v.clone().try_into()).transpose().map_err(|e| TaskError::InvalidTaskConfig(e))?.unwrap_or_default();
                            <$custom_remote_task as CustomRemoteTask>::remote_custom_handler(request, db, &config, &runner_config).await
                        }
                    )*
                    _ => unreachable!()
                }
            }
            
        }
    };
}
