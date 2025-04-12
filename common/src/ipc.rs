
use serde::{Serialize, Deserialize};
use crate::models::queue::Queue;

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
    QueueProgress
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



#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RunProgressSer {
    pub index: u32,
    pub total: u32,
    pub queue: Queue,
    pub error: Option<String>,
    pub time: u32, // time taken to run the task in seconds
}


#[derive(Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum QueueProgress {
    Progress(RunProgressSer),
    Done(Result<(u32, u32), String>),
}

pub type IpcQueueProgressResponse = Option<QueueProgress>;