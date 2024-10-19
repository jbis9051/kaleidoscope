
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum IpcRequest {
    File(IpcFileRequest),
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
        path: String,
        db_id: i32,
        length: u64,
    }
}