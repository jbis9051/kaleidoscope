use std::collections::HashMap;
use std::fmt::Debug;
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use common::remote_models::job::Job;

pub mod multipart_helper;
pub mod remote_requester;
pub mod job_util;

#[derive(Deserialize)]
pub struct StandardRemoteConfig {
    pub url: String,
    pub password: Option<String>,
}


#[derive(Deserialize)]
pub struct StandardClientConfig<T = HashMap<String, String>> {
    pub remote: StandardRemoteConfig,
    pub options: T,
}

#[derive(Serialize, Deserialize)]
pub enum RemoteTaskStatus {
    Ready,
    Busy(Job)
}

pub fn internal<E: Debug>(err: E) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, format!("internal error:{:?}", err))
}