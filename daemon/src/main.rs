use std::fmt::Debug;
use std::fs::Permissions;
use common::scan_config::AppConfig;
use nix::libc::{pid_t};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::os::unix::process::CommandExt;
use std::path::Path;
use sqlx::{SqlitePool};
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::process::Command;
use tokio::sync::oneshot::Receiver;
use common::ipc::{IpcFileRequest, IpcFileResponse, IpcRequest};
use common::models::media::Media;

#[tokio::main]
async fn main() {
    let dev_mode = std::env::var("dev_mode").is_ok();
    let config_path = std::env::args().nth(1).expect("No config file provided");

    let path = Path::new(&config_path);
    if !path.exists() {
        panic!("config file does not exist: {}", config_path);
    }
    // ensure the config file is owned by root
    let metadata = path.metadata().unwrap();
    let uid = metadata.uid();
    if uid != 0 && !dev_mode {
        panic!("config file must be owned by root!");
    }

    // ensure the config file is not writable by others
    if !metadata.permissions().readonly() && !dev_mode {
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

    let pool = SqlitePool::connect(&format!("sqlite://{}", config.db_path)).await.unwrap();

    config.canonicalize();

    let server_binary = "kaleidoscope";

    let (tx, rx) = tokio::sync::oneshot::channel();

    let config2 = config.clone();
    println!("starting unix socket server");
    let mut handle = tokio::spawn(start_server(pool, config2, rx));

    let my_path = std::env::current_exe().unwrap();
    let my_dir = my_path.parent().unwrap();
    let server_path = my_dir.join(server_binary);
    let server_path = server_path.to_str().unwrap();
    println!("starting command (kaleidoscope) server");
    let mut slave = Command::new(server_path)
        .arg(config_path)
        .uid(user.uid.as_raw())
        .gid(group.gid.as_raw())
        .env_clear()
        .env("CONFIG", serde_json::to_string(&config).unwrap())
        .stdin(std::process::Stdio::null())
        .spawn()
        .expect("failed to start server");

    tx.send(slave.id().expect("unable to obtain slave id")).unwrap();

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

pub async fn start_server(pool: SqlitePool, config: AppConfig, rx: Receiver<u32>) {
    // delete the socket if it already exists
    if Path::new(&config.socket_path).exists() {
        std::fs::remove_file(&config.socket_path).unwrap();
    }
    let socket = UnixListener::bind(config.socket_path.clone()).unwrap();

    // set permissions on the socket
    tokio::fs::set_permissions(&config.socket_path, Permissions::from_mode(0o666)).await.unwrap();


    let slave_pid = rx.await.unwrap();
    println!("slave pid: {}", slave_pid);
    
    while let Ok((stream, _)) = socket.accept().await {
        let cred = stream.peer_cred().unwrap();
        let connecting_pid = cred.pid().unwrap();

        if connecting_pid != slave_pid as pid_t {
            panic!(
                "connecting pid does not match slave pid: {} != {}",
                connecting_pid, slave_pid
            );
        }

        tokio::spawn(handle_slave(config.clone(), pool.clone(), stream));
    }
}

pub async fn handle_slave(config: AppConfig, pool: SqlitePool, mut stream: UnixStream) {
    let (reader, mut writer) = stream.split();
    let mut reader = BufReader::new(reader);
    let mut buf = String::new();
    // read IpcRequest from client
    let size = reader.read_line(&mut buf).await.unwrap();
    if size == 0 {
        return;
    }
    let req: IpcRequest = serde_json::from_str(&buf).map_err(|e| {
        format!("unable to deserialize IpcRequest: {} | {}", e, buf)
    }).unwrap();
    let IpcRequest::File(req) = req;
    let (res, file) = match handle_file_request(config, &pool, &req).await {
        Ok((res, file)) => (res, Some(file)),
        Err(res) => (res, None)
    };
    writer.write_all(serde_json::to_string(&res).unwrap().as_bytes()).await.unwrap();
    writer.write_all(b"\n").await.unwrap();
    if let Some(mut file) = file {
        tokio::io::copy(&mut file, &mut writer).await.unwrap(); // we should really limit length here
    }
}

pub async fn handle_file_request(app_config: AppConfig, pool: &SqlitePool, req: &IpcFileRequest) -> Result<(IpcFileResponse, File), IpcFileResponse> {
    if !app_config.path_matches(&req.path) {
        return Err(IpcFileResponse::Error {
            error: "path not in config, the fuck u tryin do -_-".to_string(),
        });
    }


    let media = Media::from_id(pool, &req.db_id).await.unwrap();

    if media.path != req.path {
        return Err(IpcFileResponse::Error {
            error: "path mismatch".to_string(),
        });
    }

    let path = Path::new(&media.path);
    let file = File::open(path).await.unwrap();
    let length = file.metadata().await.unwrap().len();

    Ok((IpcFileResponse::Success {
        db_id: req.db_id,
        path: media.path.clone(),
        length,
    }, file))
}
