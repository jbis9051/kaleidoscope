use std::time::{Duration, Instant};
use common::models::queue::Queue;
use crate::tasks::{BackgroundTask, Task, TaskError};
use common::models::media::Media;
use common::scan_config::AppConfig;
use common::types::AcquireClone;
use tokio::sync::mpsc;
use toml::Table;
use common::ipc::RunProgressSer;

#[derive(Debug, thiserror::Error)]
pub enum TaskOperationError {
    #[error("error running task: {0}")]
    TaskError(#[from] TaskError),
    #[error("error running task: {0}")]
    SqlxError(#[from] sqlx::Error),
}

#[derive(Debug)]
pub struct RunProgress {
    pub index: u32,
    pub total: u32,
    pub queue: Queue,
    pub error: Option<TaskError>,
    pub time: Duration, // time taken to run the task in seconds
}
impl From<RunProgress> for RunProgressSer {
    fn from(progress: RunProgress) -> Self {
        Self {
            index: progress.index,
            total: progress.total,
            queue: progress.queue,
            error: progress.error.map(|e| e.to_string()),
            time: progress.time.as_secs() as u32,
        }
    }
}

impl RunProgress {
    pub fn done(&self) -> bool {
        self.index == self.total - 1
    }
}

pub async fn delete_task_data(
    db: &mut impl AcquireClone,
    task: &impl BackgroundTask,
    media: &Media,
) -> Result<(), TaskOperationError> {
    unimplemented!();
}

pub async fn run_queue(
    db: &mut impl AcquireClone,
    tasks: &[&str],
    config: &Table,
    app_config: &AppConfig,
    progress: Option<mpsc::Sender<RunProgress>>,
) -> Result<(u32, u32), TaskOperationError> {
    let mut success = 0;
    let mut failed = 0;

    let mut total = 0;

    for task in tasks {
        total += Queue::count(db.acquire_clone(), task).await?;
    }

    for task in tasks {
        let task = Task::new(task, db, config, app_config).await?;
        while let Some(queue) = Queue::get_next(db.acquire_clone(), task.name()).await? {
            queue.delete(db.acquire_clone()).await?;

            let mut media = Media::from_id(db.acquire_clone(), &queue.media_id).await?;
            let start = Instant::now();
            match task.run_and_store(db, &mut media).await {
                Ok(_) => {
                    if let Some(progress) = &progress {
                        if let Err(e) = progress.try_send(RunProgress {
                            index: success + failed,
                            total,
                            queue,
                            error: None,
                            time: start.elapsed(),
                        }) {
                            eprintln!("error sending progress: {:?}", e);
                        }
                    }

                    success += 1;
                }
                Err(e) => {
                    eprintln!("error running task {} on {}: {:?}", e, media.path, e);

                    if let Some(progress) = &progress {
                        if let Err(e) = progress.try_send(RunProgress {
                            index: success + failed,
                            total,
                            queue,
                            error: Some(e),
                            time: start.elapsed(),
                        }) {
                            eprintln!("error sending progress: {:?}", e);
                        }
                    }

                    failed += 1;
                }
            }
        }
    }
    Ok((success, failed))
}

// takes a new media object and add it to all queues for compatible tasks
pub async fn add_to_compatible_queues(
    db: &mut impl AcquireClone,
    media: &Media,
    tasks: &[&str],
) -> Result<Vec<&'static str>, (Vec<&'static str>, TaskOperationError)> {
    let mut added: Vec<&'static str> = Vec::new();

    for task in tasks {
        if Task::compatible(task, media).await {
            Queue::delete_by_media_id(db.acquire_clone(), task, media.id)
                .await
                .map_err(|e| (added.clone(), e.into()))?;
            let mut queue = Queue {
                id: 0,
                media_id: media.id,
                task: task.to_string(),
                created_at: chrono::Utc::now().naive_utc(),
            };
            queue
                .create(db.acquire_clone())
                .await
                .map_err(|e| (added.clone(), e.into()))?;
            added.push(Task::name_from_str(task).map_err(|e| (added.clone(), e.into()))?);
        }
    }

    Ok(added)
}

// takes medias and adds them to all queues for compatible & outdated tasks
pub async fn add_outdated_queues(
    db: &mut impl AcquireClone,
    medias: &[Media],
    tasks: &[&str],
    config: &Table,
    app_config: &AppConfig,
) -> Result<Vec<(&'static str, u32)>, (Vec<(&'static str, u32)>, TaskOperationError)> {
    let mut added = Vec::new();

    for task_name in tasks {
        let mut count = 0;
        let task = Task::new(task_name, db, config, app_config)
            .await
            .map_err(|e| (added.clone(), e.into()))?;
        for media in medias {
            if Task::compatible(task_name, media).await
                && task
                    .outdated(db, media)
                    .await
                    .map_err(|e| (added.clone(), e.into()))?
            {
                Queue::delete_by_media_id(db.acquire_clone(), task_name, media.id)
                    .await
                    .map_err(|e| (added.clone(), e.into()))?;
                let mut queue = Queue {
                    id: 0,
                    media_id: media.id,
                    task: task_name.to_string(),
                    created_at: chrono::Utc::now().naive_utc(),
                };
                queue
                    .create(db.acquire_clone())
                    .await
                    .map_err(|e| (added.clone(), e.into()))?;
                count += 1;
            }
        }
        added.push((Task::name_from_str(task_name).map_err(|e| (added.clone(), e.into()))?, count));
    }

    Ok(added)
}
