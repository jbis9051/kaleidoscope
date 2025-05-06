use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;
use clap::{Parser, ValueEnum};
use log::{error, info};
use common::models::media::Media;
use common::scan_config::{AppConfig, CustomConfig};
use sqlx::{Connection, Pool, SqliteConnection};
use tokio::sync::mpsc;
use common::env::setup_log;
use common::types::AcquireClone;
use common::models::queue::Queue;
use tasks::ops::{add_to_compatible_queues, run_custom, run_custom_tasks, run_queue, RunProgress};
use tasks::tasks::{AnyTask, TaskError};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct CliArgs {
    #[arg(index = 1)]
    config: String,
    #[arg(short, long)]
    task: Option<String>,
    #[arg(short, long)]
    media: Option<String>,
    #[arg(short, long)]
    op: Operation,
    #[arg(short, long, default_value = "false")]
    store: bool,
    #[arg(short, long, default_value = "false")]
    custom: bool,
}

#[derive(Debug, Clone, ValueEnum)]
#[clap(rename_all = "snake_case")]
pub enum Operation {
    #[clap(alias = "q")]
    Queue,
    #[clap(alias = "r")]
    Run,
}

async fn progress_handler(mut recv: mpsc::Receiver<RunProgress>, mut db: impl AcquireClone){
    while let Some(progress) = recv.recv().await {
        let media = Media::from_id(db.acquire_clone(), &progress.media_id)
            .await
            .expect("error getting media");

        match &progress.error {
            Some(e) => {
                error!("({}/{}) - task '{}' - media {}: failed {:?}, took: {:?}", progress.index+1, progress.total, progress.task, media.path, e, progress.time);
            }
            None => {
                info!("({}/{}) - task '{}' - media {}: succeeded, took: {:?}", progress.index+1, progress.total, progress.task, media.path, progress.time);
            }
        }

        if progress.done() {
            break;
        }
    }
}

#[tokio::main]
async fn main() {
    setup_log("tasks");
    let args: CliArgs = CliArgs::parse();
    let CliArgs {
        config: config_path,
        task: task_name,
        media,
        op,
        store,
        custom,
    } = args;

    let mut app_config = AppConfig::from_path(&config_path);

    let mut db = SqliteConnection::connect(&format!("sqlite:{}", app_config.db_path))
        .await
        .unwrap();

    let progress_db = SqliteConnection::connect(&format!("sqlite:{}", app_config.db_path))
        .await
        .unwrap();

    let (progress_tx, progress_rx) = mpsc::channel(10);

   let join = tokio::spawn(progress_handler(progress_rx, progress_db));

    if custom {
        if !matches!(op, Operation::Run) {
            eprintln!("custom only supports run");
            return;
        }
        match (task_name, media) {
            (None, None) => {
                if !store {
                    eprintln!("store required");
                    return;
                }
                // run all in app_config
                let (succ, fail) = run_custom_tasks(&mut db, &app_config, Some(progress_tx)).await.expect("Failed to run custom tasks");
                join.await.expect("error joining progress handler");
                println!("{} tasks succeeded, {} failed", succ, fail);
            },
            (Some(task_name), None) => {
                if !store {
                    eprintln!("store required");
                    return;
                }
                // run only task_name based on app_config
                if app_config.custom.get(&task_name).is_none() {
                    eprintln!("could not find task '{}' in AppConfig", task_name);
                    return
                }
                let mut new_custom = HashMap::new();
                new_custom.insert(task_name.clone(), app_config.custom[&task_name].clone());
                app_config.custom = new_custom;
                let (succ, fail) = run_custom_tasks(&mut db, &app_config, Some(progress_tx)).await.expect("Failed to run custom tasks");
                join.await.expect("error joining progress handler");
                println!("{} tasks succeeded, {} failed", succ, fail);
            }
            (Some(task_name), Some(media)) => {
                if store {
                    eprintln!("store not supported");
                    return;
                }
                // run task_name but only with media
                let config = match app_config.custom.get(&task_name) {
                    None => {
                        eprintln!("could not find config for task '{}' in AppConfig", task_name);
                        return
                    }
                    Some(t) => t
                };

                let media_path = Path::new(&media);
                let canoc = media_path.canonicalize().expect("error canonicalizing path");
                let media = Media::from_path(&mut db, canoc.to_str().unwrap())
                    .await
                    .expect("error getting media")
                    .expect("media not found");
                let start = Instant::now();
                match run_custom(&mut db, &media, &task_name, &app_config, config).await {
                    Ok(_) => {
                        println!("ran task {} on media {}, took {:?}: succeeded", task_name, media.path, start.elapsed());
                    }
                    Err(_) => {
                        println!("ran task {} on media {}, took {:?}: failed", task_name, media.path, start.elapsed());
                    }
                }
            }
            (None, Some(_)) => {
                eprintln!("task must be provided");
            }
        }
        return;
    }

    match (task_name, media) {
        (None, None) => match op {
            Operation::Queue => {
                eprintln!("task, media must be provided when queuing");
                return;
            }
            Operation::Run => {
                if !store {
                    eprintln!("store must be true when running all tasks");
                    return;
                }

                let queue = run_queue(&mut db, &AnyTask::BACKGROUND_TASK_NAMES, &app_config.tasks, &app_config.remote, &app_config, Some(progress_tx))
                    .await
                    .expect("error running queue");
                join.await.expect("error joining progress handler");
                println!("{} tasks succeeded, {} failed", queue.0, queue.1);
            }
        },
        (Some(task), None) => {
            match op {
                Operation::Queue => {
                    eprintln!("media must be provided when queuing");
                    return;
                }
                Operation::Run => {
                    if !store {
                        eprintln!("store must be true when running specific task");
                        return;
                    }

                    // run the queue for the specified task
                    let tasks = Queue::count(&mut db, &task)
                        .await
                        .expect("error counting queue");

                    if tasks == 0 {
                        println!("no tasks in queue for '{}'", task);
                        return;
                    }

                    // let's confirm with user before running

                    if !confirm(&format!("{} tasks in queue for '{}'. Continue?", tasks, task)) {
                        println!("aborting");
                        return;
                    }

                    let queue = run_queue(&mut db, &[&task], &app_config.tasks, &app_config.remote, &app_config, Some(progress_tx))
                        .await
                        .expect("error running queue");
                    join.await.expect("error joining progress handler");
                    println!("{} tasks succeeded, {} failed", queue.0, queue.1);
                }
            }
        }
        (None, Some(media)) => {
            let media_path = Path::new(&media);
            let canoc = media_path.canonicalize().expect("error canonicalizing path");
            let media = Media::from_path(&mut db, canoc.to_str().unwrap())
                .await
                .expect("error getting media")
                .expect("media not found");

            match op {
                Operation::Queue => {
                    // add media to all compatible queues
                    let tasks = add_to_compatible_queues(&mut db, &media, &AnyTask::BACKGROUND_TASK_NAMES)
                        .await
                        .expect("error adding to compatible queues");
                    println!("added media to queues: {:?}", tasks);
                }
                Operation::Run => {
                    eprintln!("task must be provided when running specific media");
                    return;
                }
            }
        }
        (Some(task), Some(media)) => {
            let media_path = Path::new(&media);
            let canoc = media_path.canonicalize().expect("error canonicalizing path");
            let mut media = Media::from_path(&mut db, canoc.to_str().unwrap())
                .await
                .expect("error getting media")
                .expect("media not found");
            match op {
                Operation::Queue => {
                    if !AnyTask::compatible(&task, &media).await {
                        // media is not compatible, should we force?
                        if !confirm(&format!("media is not compatible with '{}'. Force?", task)) {
                            println!("aborting");
                            return;
                        }
                    }
                    // add media to the specified task queue
                    let mut queue = Queue {
                        id: 0,
                        media_id: media.id,
                        task: task.to_string(),
                        created_at: chrono::Utc::now().naive_utc(),
                    };
                    queue.create(&mut db).await.expect("error creating queue");
                    println!("added media to queue for '{}'", task);
                }
                Operation::Run => {
                    // run the specified task for the specified media
                    
                    if !AnyTask::compatible(&task, &media).await {
                        // media is not compatible, should we force?
                        if !confirm(&format!("media is not compatible with '{}'. Force?", task)) {
                            println!("aborting");
                            return;
                        }
                    }
                    
                    let task = AnyTask::new(&task, &mut db, &app_config.tasks, &app_config).await.expect("error getting task");

                    let queue = Queue::from_media_id(&mut db, &task.name(), media.id)
                        .await
                        .expect("error getting queue");
                    
                    let start = Instant::now();
                    let res = if store {
                        task.run_and_store_anywhere(&mut db, &mut media, &app_config.remote).await.expect("error running task");
                        None
                    } else {
                        Some(task.run_anywhere(&mut db, &media, &app_config.remote).await
                            .expect("error running task"))
                    };
                    
                    let end = start.elapsed();
                    
                    if store {
                        if let Some(queue) = queue {
                            queue.delete(&mut db).await.expect("error deleting queue");
                        }
                    }

                    println!("task '{}' succeeded, took: {:?}, result: {:?}", task.name(), end, res);
                }
            }
        }
    }
}


pub fn confirm(msg: &str) -> bool {
    let mut input = String::new();
    println!("{} (y/n)", msg);
    std::io::stdin()
        .read_line(&mut input)
        .expect("error reading input");
    input.trim() == "y"
}