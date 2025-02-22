use sqlx::{Executor, SqliteExecutor};
use common::models::media::Media;
use common::types::{AcquireClone, DbPool, SqliteAcquire};
use crate::queue::Queue;
use crate::tasks::{Task, TaskError};

#[derive(Debug, thiserror::Error)]
pub enum RunError {
    #[error("error running task: {0}")]
    TaskError(#[from] TaskError),
    #[error("error running task: {0}")]
    SqlxError(#[from] sqlx::Error),
}

pub async fn run_queue(db: &mut impl AcquireClone, tasks: &[&str]) -> Result<(u32, u32), RunError> {
    let mut success = 0;
    let mut failed = 0;

    for task in tasks {
        let task = Task::new(task, db.acquire_clone()).await?;
        while let Some(queue) = Queue::get_next(db.acquire_clone(), task.name()).await? {
            let media = Media::from_id(db.acquire_clone(), &queue.media_id).await?;
            match task.run_and_store(db.acquire_clone(), &media).await {
                Ok(_) => {
                    queue.delete(db.acquire_clone()).await?;
                    success += 1;
                }
                Err(e) => {
                    eprintln!("error running task {} on {}: {:?}", e, media.path, e);
                    failed += 1;
                }
            }
        }
    }
    Ok((success, failed))
}


// takes a new media object and add it to all queues for compatible tasks
pub async fn add_to_compatible_queues(db: &mut impl AcquireClone, media: &Media, tasks: &[&str]) -> Result<Vec<&'static str>, (Vec<&'static str>, RunError)> {
    let mut added: Vec<&'static str> = Vec::new();

    for task in tasks {
        if Task::compatible(task, media).await {
            let mut queue =  Queue {
                id: 0,
                media_id: media.id,
                task: task.to_string(),
                created_at: chrono::Utc::now().naive_utc(),
            };
            queue.create(db.acquire_clone()).await.map_err(|e| (added.clone(), e.into()))?;
            added.push(Task::name_from_str(task).map_err(|e| (added.clone(), e.into()))?);
        }
    }

    Ok(added)
}
