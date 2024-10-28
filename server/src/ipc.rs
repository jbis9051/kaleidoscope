use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWriteExt, BufReader, Take};
use tokio::net::UnixStream;
use common::ipc::{IpcFileRequest, IpcFileResponse, IpcRequest};
use common::models::media::Media;
use common::scan_config::AppConfig;

pub async fn request_ipc_file(config: &AppConfig, media: &Media) -> Result<Take<BufReader<UnixStream>>, String> {
    let socket_path = &config.socket_path;
    let mut stream = UnixStream::connect(socket_path).await.expect("Unable to connect to socket");
    
    let ipc_req = IpcRequest::File(IpcFileRequest {
        db_id: media.id,
        path: media.path.clone(),
    });
    
    let mut ipc_req = serde_json::to_vec(&ipc_req).expect("Unable to serialize IpcRequest");
    ipc_req.push(b'\n');
    stream.write_all(&ipc_req).await.expect("Unable to write to socket");
    
    
    let mut reader = BufReader::new(stream);
    
    let mut res = String::new();
    reader.read_line(&mut res).await.expect("Unable to read from socket");
    let res: IpcFileResponse = serde_json::from_str(&res).map_err(|e| format!("Unable to deserialize IpcFileResponse: {} | {}", e, res)).unwrap();
    
    let (path, db_id, length) = match res {
        IpcFileResponse::Error { error } => return Err( error ),
        IpcFileResponse::Success { path, db_id, length } => (path, db_id, length),
    };
    
    if db_id != media.id {
        panic!("db_id does not match: {} != {}", db_id, media.id);
    }
    
    if path != media.path {
        panic!("path does not match: {} != {}", path, media.path);
    }

    Ok(reader.take(length))
}