use crate::remote_utils::multipart_helper::MultipartHelper;
use crate::remote_utils::{internal, StandardClientConfig};
use crate::run_python::run_python;
use crate::tasks::{BackgroundTask, CustomTask, RemoteBackgroundTask, RemoteTask, Task, MODEL_DIR};
use axum::extract::Request;
use axum::response::{ErrorResponse, IntoResponse, Response};
use axum::Json;
use common::media_processors::format::{AnyFormat, MetadataError};
use common::models::media::Media;
use common::runner_config::RemoteRunnerGlobalConfig;
use common::scan_config::AppConfig;
use common::types::AcquireClone;
use serde::{Deserialize, Serialize};
use sqlx::types::uuid;
use std::fmt::{Debug, Pointer};
use std::ops::Add;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use chrono::{TimeDelta, Utc};
use reqwest::StatusCode;
use uuid::Uuid;
use common::media_processors::format::audio::Audio;
use common::remote_models::job::{Job, JobStatus};
use crate::remote_utils::job_util::{start_job};
use crate::remote_utils::remote_requester::{OneShotResponse, RemoteRequester, RequestError};
use crate::tasks::whisper::WhisperError;

// the script to run
const VLLM_SCRIPT: &str = "vllm.py";

const VERSION: i32 = 0;

#[derive(Default, Clone, Debug)]
pub struct VLLM;
impl Task for VLLM {
    type Error = VLLMError;
    const NAME: &'static str = "vllm";
    type Config = ();
}

impl VLLM {
    pub fn vllm(scripts_dir: &str, python_path: &str, args: &<VLLM as CustomTask>::Args) -> Result<<VLLM as CustomTask>::Output, VLLMError> {
        let script_path = Path::new(scripts_dir).join(VLLM_SCRIPT);
        let (prompt, image_path, max_tokens, runs) = args;
        let output = run_python(python_path, script_path.to_str().unwrap(), &[&prompt, &image_path, max_tokens.to_string().as_str(), runs.to_string().as_str()])?;
        if !output.status.success() {
            let output = String::from_utf8(output.stderr)
                .map_err(|_| VLLMError::OutputParseError)?;
            return Err(VLLMError::VllmError(output));
        }

        let stdout =
            String::from_utf8(output.stdout).map_err(|_| VLLMError::OutputParseError)?;

        Self::parse(&stdout, *runs)
    }

    pub fn parse(output: &str, runs: u8) -> Result<<VLLM as CustomTask>::Output, VLLMError> {
        let mut lines = output.lines();

        let _load_time = lines.next().unwrap();
        let mut output = Vec::with_capacity(runs as usize);

        for _ in 0..runs {
            let _iteration = lines.next().unwrap();
            let _time = lines.next().unwrap();
            let out: String = serde_json::from_str(lines.next().unwrap()).unwrap();
            output.push(out)
        }
        
        Ok(output)
    }
}

impl CustomTask for VLLM {
    // <prompt> <image_path> <max_tokens> <runs>
    type Args = (String, String, u32, u8);
    type Output = Vec<String>;

    async fn run_custom(db: &mut impl AcquireClone, config: &Self::Config, app_config: &AppConfig, args: Self::Args) -> Result<Self::Output, Self::Error> {
        Self::vllm(&app_config.scripts_dir, &app_config.python_path, &args)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum VLLMError {
    #[error("output parse error")]
    OutputParseError,
    #[error("python error: {0}")]
    PythonError(#[from] std::io::Error),
    #[error("vllm error: {0}")]
    VllmError(String),
    #[error("job error: {0:?}")]
    JobError(Job),
    #[error("unexpected response {0:?}")]
    UnexpectedResponse(reqwest::Response),
    #[error("request error: {0}")]
    RequestError(#[from] RequestError)
}
