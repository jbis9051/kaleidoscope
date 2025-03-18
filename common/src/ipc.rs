
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum IpcRequest {
    FileData {
        file: IpcFileRequest,
        start: u64,
        end: u64,
    },
    FileSize {
        file: IpcFileRequest
    },
}


#[derive(Serialize, Deserialize, Debug)]
pub struct IpcFileRequest {
    pub path: String,
    pub db_id: i32,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "status")]
pub enum IpcFileResponse {
    Error {
        error: String,
    },
    Success {
        file: IpcFileRequest,
        file_size: u64,
        response_size: u64,
    }
}