use crate::remote_utils::multipart_helper::MultipartHelper;
use crate::remote_utils::remote_requester::{RemoteRequester, RequestError};
use crate::remote_utils::{internal, StandardClientConfig};
use crate::run_python::run_python;
use crate::tasks::{CustomRemoteTask, CustomTask, RemoteTask, Task};
use axum::extract::Request;
use axum::response::{ErrorResponse, IntoResponse, Response};
use axum::Json;
use common::remote_models::job::Job;
use common::runner_config::RemoteRunnerGlobalConfig;
use common::scan_config::AppConfig;
use common::types::AcquireClone;
use reqwest::multipart::Form;
use reqwest::StatusCode;
use std::fmt::{Debug};
use std::path::Path;
use tokio::fs;
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
        let (prompt, image_path, max_tokens, runs, temperature) = args;
        let output = run_python(python_path, script_path.to_str().unwrap(), &[&prompt, &image_path, max_tokens.to_string().as_str(), runs.to_string().as_str(), temperature.to_string().as_str()])?;
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
    // <prompt> <image_path> <max_tokens> <runs> <temperature>
    type Args = (String, String, u32, u8, f32);
    type Output = Vec<String>;

    async fn run_custom(db: &mut impl AcquireClone, config: &Self::Config, app_config: &AppConfig, args: Self::Args) -> Result<Self::Output, Self::Error> {
        Self::vllm(&app_config.scripts_dir, &app_config.python_path, &args)
    }
}

impl RemoteTask for VLLM {
    type RunnerTaskConfig = bool;
    type ClientTaskConfig = StandardClientConfig;
}

impl CustomRemoteTask for VLLM {
    async fn remote_custom_handler(request: Request, db: impl AcquireClone + Send + 'static, runner_config: &Self::RunnerTaskConfig, remote_server_config: &RemoteRunnerGlobalConfig) -> Result<Response, ErrorResponse> {
        let mut multipart = MultipartHelper::try_from_request(request).await?;
        let mut args: Self::Args = multipart.json("args").await?;
        if args.1 != "image.jpg" {
            return Err((StatusCode::BAD_REQUEST, "the second argument must be 'image.jpg'").into());
        }
        let (image_file, _) = multipart.file("image", ".jpg").await?;
        args.1 = image_file.to_str().expect("image file contains invalid UTF-8").to_string();
        let result = Self::vllm(&remote_server_config.scripts_dir, &remote_server_config.python_path, &args)
            .map_err(internal)?;

        fs::remove_file(image_file).await.map_err(internal)?;

        let response: Json<Self::Output> = result.into();

        Ok(response.into_response())
    }

    async fn run_custom_remote(db: &mut impl AcquireClone, client_config: &Self::ClientTaskConfig, app_config: &AppConfig, mut args: Self::Args) -> Result<Self::Output, Self::Error> {
        let client = RemoteRequester::new(Self::NAME.to_string(), client_config.remote.url.clone(), client_config.remote.password.clone(), false);
        
        let image_path = args.1.clone();
        args.1 = "image.jpg".to_string();
        
        let mut form = Form::new();
        form = form.text("args", serde_json::to_string(&args).expect("failed to serialize args"));
        form = form.file("image", image_path).await?;
        
        let res = client.request_multipart(form).await?;
        match res.status() {
            StatusCode::OK => {
                let out: Self::Output = res.json().await.map_err(|e| VLLMError::BadResponse(e))?;
                Ok(out)
            },
            _ => Err(VLLMError::UnexpectedResponse(res))
        }
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
    #[error("bad response {0:?}")]
    BadResponse(#[from] reqwest::Error),
    #[error("request error: {0}")]
    RequestError(#[from] RequestError)
}
