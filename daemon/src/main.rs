use common::ipc::{IpcFileRequest, IpcFileResponse, IpcRequest, QueueProgress};
use common::models::media::Media;
use common::models::queue::Queue;
use common::scan_config::AppConfig;
use nix::libc::pid_t;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::fs::Permissions;
use std::io::SeekFrom;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::Path;
use std::sync::Arc;
use tasks::ops::RunProgress;
use tasks::tasks::AnyTask;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncSeekExt, AsyncWriteExt, BufReader, Take};
use tokio::net::{UnixListener, UnixStream};
use tokio::process::Command;
use tokio::sync::oneshot::Receiver;
use tokio::sync::{mpsc, RwLock};

static QUEUE_PROGRESS: Lazy<Arc<RwLock<QueueProgress>>> =
    Lazy::new(|| Arc::new(RwLock::new(QueueProgress::Initial)));

#[tokio::main]
async fn main() {
    let env_var = common::env::EnvVar::from_env();
    let config_path = std::env::args().nth(1).expect("No config file provided");

    let path = Path::new(&config_path);
    if !path.exists() {
        panic!("config file does not exist: {}", config_path);
    }
    // ensure the config file is owned by root
    let metadata = path.metadata().unwrap();
    let uid = metadata.uid();
    if uid != 0 && !env_var.dev_mode {
        panic!("config file must be owned by root!");
    }

    // ensure the config file is not writable by others
    if !metadata.permissions().readonly() && !env_var.dev_mode {
        panic!("config file must not be writable by others!");
    }

    let mut config = AppConfig::from_path(&config_path);

    let user = nix::unistd::User::from_name(&config.client_user)
        .expect("Unable to get user")
        .expect("User not found");
    let group = nix::unistd::Group::from_name(&config.client_group)
        .expect("Unable to get group")
        .expect("Group not found");

    if user.uid.is_root() {
        panic!("client_user must not be root!");
    }

    let pool = SqlitePool::connect(&format!("sqlite://{}", config.db_path))
        .await
        .unwrap();

    if env_var.db_migrate {
        println!("Migrating database");
        sqlx::migrate!("../db/migrations")
            .run(&pool)
            .await
            .expect("Failed to migrate database");
        println!("Migration complete");
    }

    config.canonicalize();

    let server_binary = "kaleidoscope-server";

    let (tx, rx) = tokio::sync::oneshot::channel();

    let config2 = config.clone();
    println!("starting unix socket server");
    let mut handle = tokio::spawn(start_server(pool.clone(), config2, rx, env_var.dev_mode));

    let _ = tokio::spawn(queue_runner(pool, config.clone()));

    let my_path = std::env::current_exe().unwrap();
    let my_dir = my_path.parent().unwrap();
    let server_path = my_dir.join(server_binary);
    let server_path = server_path.to_str().unwrap();
    println!("starting command (kaleidoscope) server");
    let mut slave = Command::new(server_path) // we spawn the server as a child process so it's easier to manage and secure the IPC
        .arg(config_path)
        .uid(user.uid.as_raw())
        .gid(group.gid.as_raw())
        .env_clear()
        .env("CONFIG", serde_json::to_string(&config).unwrap())
        .env("dev_mode", env_var.dev_mode.to_string())
        .stdin(std::process::Stdio::null())
        .spawn()
        .expect("failed to start server");

    tx.send(slave.id().expect("unable to obtain slave id"))
        .unwrap();

    // wait until the child dies or the server dies
    tokio::select! {
        _ = &mut handle => {
            println!("unix socket server died, killing kaleidoscope server");
            slave.kill().await.unwrap();
        },
        exit_code = slave.wait() => {
            println!("kaleidoscope server died with code {:?}, exiting...", exit_code);
            handle.abort();
        }
    }
}

pub async fn start_server(pool: SqlitePool, config: AppConfig, rx: Receiver<u32>, dev_mode: bool) {
    // delete the socket if it already exists
    if Path::new(&config.socket_path).exists() {
        std::fs::remove_file(&config.socket_path).unwrap();
    }
    let socket = UnixListener::bind(config.socket_path.clone()).unwrap();

    // set permissions on the socket
    // TODO: we might want to encrypt the data over the socket, but I don't think it's necessary right now
    tokio::fs::set_permissions(&config.socket_path, Permissions::from_mode(0o666))
        .await
        .unwrap();

    let slave_pid = rx.await.unwrap();
    println!("slave pid: {}", slave_pid);

    while let Ok((stream, _)) = socket.accept().await {
        let cred = stream.peer_cred().unwrap();
        let connecting_pid = cred.pid().unwrap();

        if connecting_pid != slave_pid as pid_t {
            // only permit our slave to connect, this prevents other processes from using our daemon
            panic!(
                "connecting pid does not match slave pid: {} != {}",
                connecting_pid, slave_pid
            );
        }

        tokio::spawn(handle_slave(config.clone(), pool.clone(), stream, dev_mode));
    }
}

macro_rules! return_on_err {
    ($res:expr, $dev_mode: tt) => {
        match $res {
            Ok(res) => res,
            Err(e) => {
                if $dev_mode {
                    eprintln!("{}", e);
                    return;
                } else {
                    return;
                }
            }
        }
    };
}

pub async fn handle_slave(
    config: AppConfig,
    pool: SqlitePool,
    mut stream: UnixStream,
    dev_mode: bool,
) {
    let (reader, mut writer) = stream.split();
    let mut reader = BufReader::new(reader).lines();

    while let Ok(Some(line)) = reader.next_line().await {
        let req: IpcRequest = serde_json::from_str(&line)
            .map_err(|e| format!("unable to deserialize IpcRequest: {} | {}", e, line))
            .unwrap();

        match req {
            IpcRequest::FileData { file, start, end } => {
                let (res, file) = match handle_file_request(&config, &pool, &file, start, end).await
                {
                    Ok((res, file)) => (res, Some(file)),
                    Err(res) => (res, None),
                };

                return_on_err!(
                    writer
                        .write_all(serde_json::to_string(&res).unwrap().as_bytes())
                        .await,
                    dev_mode
                );

                return_on_err!(writer.write_all(b"\n").await, dev_mode);

                if let Some(mut file) = file {
                    return_on_err!(tokio::io::copy(&mut file, &mut writer).await, dev_mode);
                }
            }
            IpcRequest::FileSize { file } => {
                let res = handle_file_size_request(&config, &pool, &file)
                    .await
                    .unwrap_or_else(|res| res);

                return_on_err!(
                    writer
                        .write_all(serde_json::to_string(&res).unwrap().as_bytes())
                        .await,
                    dev_mode
                );

                return_on_err!(writer.write_all(b"\n").await, dev_mode);
            }
            IpcRequest::QueueProgress => {
                let lock = QUEUE_PROGRESS.read().await;
                let res = lock.clone();
                drop(lock);

                return_on_err!(
                    writer
                        .write_all(serde_json::to_string(&res).unwrap().as_bytes())
                        .await,
                    dev_mode
                );

                return_on_err!(writer.write_all(b"\n").await, dev_mode);
            }
        }
    }
}

pub async fn file_request_permissions(
    app_config: &AppConfig,
    pool: &SqlitePool,
    req: &IpcFileRequest,
) -> Result<(Media, File), IpcFileResponse> {
    if !app_config.path_matches(&req.path) {
        // extra security check 1: ensure the path is in the config and not some random path, config is trusted given above permission checks
        return Err(IpcFileResponse::Error {
            error: "path not in config, the fuck u tryin do -_-".to_string(),
        });
    }

    let media = Media::from_id(pool, &req.db_id).await.unwrap(); // since we don't enforce permissions on the DB file, this is somewhat vulnerable to various attacks, however it's highly limited given the above check

    if media.path != req.path {
        // extra security check 2: ensure the path is in the DB and matches the media requested, without fixing the permission this doesn't provide much more than a sanity check
        return Err(IpcFileResponse::Error {
            error: "path mismatch".to_string(),
        });
    }

    let path = Path::new(&media.path);

    let file = File::open(path).await.map_err(|e| IpcFileResponse::Error {
        error: format!("couldn't open file: {} - {:?}", media.path, e),
    })?;

    Ok((media, file))
}

pub async fn handle_file_size_request(
    app_config: &AppConfig,
    pool: &SqlitePool,
    req: &IpcFileRequest,
) -> Result<IpcFileResponse, IpcFileResponse> {
    let (media, file) = file_request_permissions(app_config, pool, req).await?;

    let file_size = file
        .metadata()
        .await
        .map_err(|e| IpcFileResponse::Error {
            error: format!("couldn't get metadata: {} - {:?}", media.path, e),
        })?
        .len();

    Ok(IpcFileResponse::Success {
        file: IpcFileRequest {
            db_id: req.db_id,
            path: media.path.clone(),
        },
        file_size,
        response_size: 0,
    })
}

pub async fn handle_file_request(
    app_config: &AppConfig,
    pool: &SqlitePool,
    req: &IpcFileRequest,
    start: u64,
    end: u64,
) -> Result<(IpcFileResponse, Take<File>), IpcFileResponse> {
    let (media, mut file) = file_request_permissions(app_config, pool, req).await?;

    let file_size = file
        .metadata()
        .await
        .map_err(|e| IpcFileResponse::Error {
            error: format!("couldn't get metadata: {} - {:?}", media.path, e),
        })?
        .len();

    file.seek(SeekFrom::Start(start))
        .await
        .map_err(|e| IpcFileResponse::Error {
            error: format!("couldn't seek to start: {} - {:?}", media.path, e),
        })?;

    let take = file.take(end - start);

    Ok((
        IpcFileResponse::Success {
            file: IpcFileRequest {
                db_id: req.db_id,
                path: media.path.clone(),
            },
            file_size,
            response_size: end - start,
        },
        take,
    ))
}

pub async fn queue_runner(pool: SqlitePool, app_config: AppConfig) {
    let (progress_tx, mut progress_rx) = mpsc::channel(10);

    let tasks = AnyTask::BACKGROUND_TASK_NAMES;
    
    let mut total = 0;
    for task in tasks {
        total += Queue::count(&pool, task).await.expect("couldn't count queue");
    }
    
    {
        let mut lock = QUEUE_PROGRESS.write().await;
        *lock = QueueProgress::Starting{
            total,
        };
    }

    let handle = tokio::spawn(async move {
        tasks::ops::run_queue(&mut &pool, &tasks, &app_config.tasks, &app_config.remote, &app_config, Some(progress_tx)).await
    });

    while let Some(progress) = progress_rx.recv().await {
        let done = progress.done();
        let mut lock = QUEUE_PROGRESS.write().await;
        *lock = QueueProgress::Progress(progress.into());
        if done {
            break;
        }
    }

    let (success, failed) = handle.await.unwrap().unwrap();

    let mut lock = QUEUE_PROGRESS.write().await;
    *lock = QueueProgress::Done(Ok((success, failed)));
}
