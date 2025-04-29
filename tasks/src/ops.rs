use std::borrow::Borrow;
use tokio::io::AsyncBufReadExt;
use tokio::io::{AsyncWriteExt, BufReader};
use std::time::{Duration, Instant};
use chrono::Utc;
use log::{debug, error};
use serde_json::json;
use tokio::process::Command;
use common::models::queue::Queue;
use crate::tasks::{BackgroundTask, AnyTask, TaskError};
use common::models::media::Media;
use common::scan_config::{AppConfig, CustomConfig};
use common::types::{AcquireClone, SqliteAcquire};
use tokio::sync::mpsc;
use toml::Table;
use common::ipc::RunProgressSer;
use common::models::custom_task_media::CustomTaskMedia;
use crate::custom_task::{call_fn, FnCall};

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
    pub task: String,
    pub media_id: i32,
    pub queue_id: Option<i32>,
    pub error: Option<TaskError>,
    pub time: Duration, // time taken to run the task in seconds
}
impl From<RunProgress> for RunProgressSer {
    fn from(progress: RunProgress) -> Self {
        Self {
            index: progress.index,
            total: progress.total,
            task: progress.task,
            media_id: progress.media_id,
            queue_id: progress.queue_id,
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
    remote_configs: &Table,
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
        let task = AnyTask::new(task, db, config, app_config).await?;
        while let Some(queue) = Queue::get_next(db.acquire_clone(), task.name()).await? {
            queue.delete(db.acquire_clone()).await?;

            let mut media = Media::from_id(db.acquire_clone(), &queue.media_id).await?;
            let start = Instant::now();
            match task.run_and_store_anywhere(db, &mut media, remote_configs).await {
                Ok(_) => {
                    if let Some(progress) = &progress {
                        if let Err(e) = progress.try_send(RunProgress {
                            index: success + failed,
                            total,
                            task: queue.task,
                            media_id: queue.media_id,
                            queue_id: Some(queue.id),
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
                            task: queue.task,
                            media_id: queue.media_id,
                            queue_id: Some(queue.id),
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
        if AnyTask::compatible(task, media).await {
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
            added.push(AnyTask::name_from_str(task).map_err(|e| (added.clone(), e.into()))?);
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
        let task = AnyTask::new(task_name, db, config, app_config)
            .await
            .map_err(|e| (added.clone(), e.into()))?;
        for media in medias {
            if AnyTask::compatible(task_name, media).await
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
        added.push((AnyTask::name_from_str(task_name).map_err(|e| (added.clone(), e.into()))?, count));
    }

    Ok(added)
}



pub async fn run_custom(
    db: &mut impl AcquireClone,
    media: &Media,
    app_config: &AppConfig,
    custom: &CustomConfig
) -> Result<(), TaskError> {
    let mut child = Command::new(&app_config.python_path)
        .arg(&custom.path)
        .env("KALEIDOSCOPE_PYTHON_DIR", &app_config.scripts_dir)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("unable to spawn custom command");

    let mut stdin = child.stdin.take().expect("Failed to open stdin");

    stdin
        .write_all(
            &serde_json::to_vec(&json!({
                "media": media,
                "version": custom.version,
            }))
                .unwrap(),
        )
        .await
        .expect("unable to write initial input");
    stdin
        .write_all(b"\n")
        .await
        .expect("unable to write initial input");

    let stdout = child.stdout.take().expect("Failed to open stdout");
    let mut reader = BufReader::new(stdout).lines();
    while let Some(line) = reader.next_line().await.unwrap() {
        debug!("run_custom script outputted: {}", line);
        let fn_call: FnCall = serde_json::from_str(&line).expect("unable to parse input");
        let res = call_fn(db, media, app_config, custom.version, fn_call).await?;
        debug!("run_custom responded: {}", res);
        if res != "null" {
            let _ = stdin.write_all(res.as_bytes()).await;
            let _ = stdin.write_all(b"\n").await;
        }
    }

    let out = child
        .wait_with_output()
        .await
        .expect("unable to wait for child process");

    if !out.status.success() {
        return Err(TaskError::CustomTaskError((out.status, out.stderr)));
    }

    Ok(())
}

async fn get_medias(db: impl SqliteAcquire<'_>, task_name: &str, custom: &CustomConfig) -> Result<Vec<Media>, sqlx::Error> {
    // we don't care about asc, limit, order by
    let media_query = custom.query.to_count_query();
    let mut query = sqlx::QueryBuilder::new("SELECT DISTINCT media.*, MAX(custom_task_media.version) as max_version FROM media LEFT JOIN custom_task_media ON custom_task_media.media_id = media.id ");
    media_query
        .sqlize(&mut query)
        .expect("unable to add queries");
    query.push(" \
    GROUP BY media.id, custom_task_media.task_name \
    HAVING \
        custom_task_media.task_name IS NULL \
        OR custom_task_media.task_name != ");
    query.push_bind(task_name);
    query.push(" OR (custom_task_media.task_name = ");
    query.push_bind(task_name);
    query.push("AND max_version < ");
    query.push_bind(custom.version);
    query.push(" )");
    let query = query.build();
    let mut conn = db.acquire().await.unwrap();
    let medias: Vec<Media> = {
        query
            .fetch_all(&mut *conn)
            .await
            .unwrap()
            .into_iter()
            .map(|r| r.borrow().into())
            .collect()
    };
    Ok(medias)
}


pub async fn run_custom_tasks(
    db: &mut impl AcquireClone,
    app_config: &AppConfig,
    progress: Option<mpsc::Sender<RunProgress>>,
) -> Result<(u32, u32), TaskOperationError> {
    let mut success = 0;
    let mut failed = 0;

    let mut total = 0;
    
    let mut medias = Vec::with_capacity(app_config.custom.len());
    
    for (task_name, config) in &app_config.custom {
        let task_medias = get_medias(db.acquire_clone(), task_name, config).await?;
        total += task_medias.len();
        medias.push(task_medias);
    }

    for ((task_name, config), medias) in app_config.custom.iter().zip(medias.iter()) {
        for media in medias {
            let start = Instant::now();
            match run_custom(db, &media, &app_config, &config).await {
                Ok(_) => {
                    media.remove_outdated_custom(db.acquire_clone(), task_name, config.version).await.expect("couldn't remove old custom metadata");
                    media.remove_custom_task_media(db.acquire_clone(), task_name).await.expect("couldn't complete marker");
                    let mut custom = CustomTaskMedia {
                        id: 0,
                        media_id: media.id,
                        task_name: task_name.to_string(),
                        version: config.version,
                        created_at: Utc::now().naive_utc(),
                    };
                    custom.create(db.acquire_clone()).await.expect("couldn't create");

                    if let Some(progress) = &progress {
                        if let Err(e) = progress.try_send(RunProgress {
                            index: success + failed,
                            total: total as u32,
                            task: task_name.clone(),
                            media_id: media.id,
                            time: start.elapsed(),
                            error: None,
                            queue_id: None,
                        }) {
                            error!("error sending progress: {:?}", e);
                        }
                    }

                    success += 1;
                }
                Err(e) => {
                    media.remove_custom_for_version(db.acquire_clone(), config.version).await.expect("couldn't remove custom metadata");
                    error!("error running task {} on {}: {:?}", e, media.path, e);

                    if let Some(progress) = &progress {
                        if let Err(e) = progress.try_send(RunProgress {
                            index: success + failed,
                            total: total as u32,
                            task: task_name.clone(),
                            media_id: media.id,
                            time: start.elapsed(),
                            error: Some(e),
                            queue_id: None,
                        }) {
                            error!("error sending progress: {:?}", e);
                        }
                    }

                    failed += 1;
                }
            }
            
        }
    }
    Ok((success, failed))
}
