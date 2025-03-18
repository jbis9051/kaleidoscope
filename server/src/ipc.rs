use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWriteExt, BufReader, BufWriter, Take};
use tokio::net::unix::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::UnixStream;
use common::ipc::{IpcFileRequest, IpcFileResponse, IpcRequest};
use common::models::media::Media;

pub struct BufUnixStream {
    pub reader: BufReader<OwnedReadHalf>,
    pub writer: BufWriter<OwnedWriteHalf>,
}

impl BufUnixStream {
    pub fn new(stream: UnixStream) -> Self {
        let (reader, writer) = stream.into_split();
        let reader = BufReader::new(reader);
        let writer = BufWriter::new(writer);
        Self { reader, writer }
    }
}


// sends a request to the UnixStream and returns the response, stream can be reused
async fn req_res(stream: &mut BufUnixStream, req: IpcRequest) -> Result<IpcFileResponse, String> {
    let mut req = serde_json::to_vec(&req).expect("Unable to serialize IpcRequest");
    req.push(b'\n');
    stream.writer.write_all(&req).await.expect("Unable to write to socket");
    stream.writer.flush().await.expect("Unable to flush socket");

    let mut res = String::new();
    stream.reader.read_line(&mut res).await.expect("Unable to read from socket");
    let res: IpcFileResponse = serde_json::from_str(&res).map_err(|e| format!("Unable to deserialize IpcFileResponse: {} | {}", e, res)).unwrap();
    Ok(res)
}

pub async fn request_file_size(stream: &mut BufUnixStream, media: &Media) -> Result<u64, String> {
    let req = IpcRequest::FileSize {
        file: IpcFileRequest {
            db_id: media.id,
            path: media.path.clone(),
        }
    };

    let res = req_res(stream, req).await?;

    match res {
        IpcFileResponse::Error { error } => Err( error ),
        IpcFileResponse::Success { file_size, .. } => Ok( file_size ),
    }
}
// sends a request to the UnixStream and returns the response, stream can be reused, returns the amount of bytes read
pub async fn request_file(stream: &mut BufUnixStream, media: &Media, start: u64, end: u64) -> Result<u64, String> {
    let req = IpcRequest::FileData {
        file: IpcFileRequest {
            db_id: media.id,
            path: media.path.clone(),
        },
        start,
        end,
    };
    
    let res = req_res(stream, req).await?;
    
    let ( file, file_size, response_size) = match res {
        IpcFileResponse::Error { error } => return Err( error ),
        IpcFileResponse::Success { file, file_size, response_size } => ( file, file_size, response_size),
    };

    if file.db_id != media.id {
        panic!("db_id does not match: {} != {}", file.db_id, media.id);
    }

    if file.path != media.path {
        panic!("path does not match: {} != {}", file.path, media.path);
    }

    if file_size != media.size as u64 {
        panic!("file_size does not match: {} != {}", file_size, media.size);
    }

    let req_size = end - start;

    if response_size != req_size {
        panic!("response_size does not match: {} != {}", response_size, req_size);
    }
    
    Ok( response_size )
}