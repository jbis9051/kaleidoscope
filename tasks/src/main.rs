use std::path::Path;
use std::time::Instant;
use clap::{Parser, ValueEnum};
use common::models::media::Media;
use common::scan_config::AppConfig;
use sqlx::{Connection, Pool, SqliteConnection};
use tokio::sync::mpsc;
use common::types::AcquireClone;
use common::models::queue::Queue;
use tasks::ops::{add_to_compatible_queues, run_queue, RunProgress};
use tasks::tasks::Task;

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
        let media = Media::from_id(db.acquire_clone(), &progress.queue.media_id)
            .await
            .expect("error getting media");

        match &progress.error {
            Some(e) => {
                eprintln!("({}/{}) - task '{}' - media {}: failed {:?}, took: {:?}", progress.index+1, progress.total, progress.queue.task, media.path, e, progress.time);
            }
            None => {
                println!("({}/{}) - task '{}' - media {}: succeeded, took: {:?}", progress.index+1, progress.total, progress.queue.task, media.path, progress.time);
            }
        }

        if progress.done() {
            break;
        }
    }
}

#[tokio::main]
async fn main() {
    let args: CliArgs = CliArgs::parse();
    let CliArgs {
        config: config_path,
        task: task_name,
        media,
        op,
        store
    } = args;

    let app_config = AppConfig::from_path(&config_path);

    let mut db = SqliteConnection::connect(&format!("sqlite:{}", app_config.db_path))
        .await
        .unwrap();

    let progress_db = SqliteConnection::connect(&format!("sqlite:{}", app_config.db_path))
        .await
        .unwrap();

    let (progress_tx, progress_rx) = mpsc::channel(10);

   let join = tokio::spawn(progress_handler(progress_rx, progress_db));

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

                let queue = run_queue(&mut db, &Task::TASK_NAMES, &app_config.tasks, &app_config, Some(progress_tx))
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

                    let queue = run_queue(&mut db, &[&task], &app_config.tasks, &app_config, Some(progress_tx))
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
                    let tasks = add_to_compatible_queues(&mut db, &media, &Task::TASK_NAMES)
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
                    if !Task::compatible(&task, &media).await {
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
                    
                    if !Task::compatible(&task, &media).await {
                        // media is not compatible, should we force?
                        if !confirm(&format!("media is not compatible with '{}'. Force?", task)) {
                            println!("aborting");
                            return;
                        }
                    }
                    
                    let task = Task::new(&task, &mut db, &app_config.tasks, &app_config).await.expect("error getting task");

                    let queue = Queue::from_media_id(&mut db, &task.name(), media.id)
                        .await
                        .expect("error getting queue");
                    
                    let start = Instant::now();
                    let res = if store {
                        task.run_and_store(&mut db, &mut media).await.expect("error running task");
                        None
                    } else {
                        Some(task.run(&mut db, &media).await
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