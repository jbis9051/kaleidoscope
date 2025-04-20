use crate::remote_utils::multipart_helper::MultipartHelper;
use crate::remote_utils::{internal, StandardClientConfig};
use crate::run_python::run_python;
use crate::tasks::{BackgroundTask, RemoteBackgroundTask, MODEL_DIR};
use axum::extract::Request;
use axum::response::{ErrorResponse, IntoResponse, Response};
use axum::Json;
use common::media_processors::format::{AnyFormat, MetadataError};
use common::models::media::Media;
use common::runner_config::RemoteRunnerConfig;
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

// where the transcription files are stored
const WHISPER_DIR: &str = "whisper";

// where the models are stored
const DOWNLOAD_ROOT: &str = "whisper_root";

// the script to run
const WHISPER_SCRIPT: &str = "fw-transcribe.py";

const VERSION: i32 = 0;

#[derive(Default, Clone)]
pub struct Whisper {
    config: WhisperConfig,
    data_dir: PathBuf,
    app_config: AppConfig,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Default, Clone)]
pub struct WhisperConfig {
    pub model: String,
    pub device: String,
    pub compute_type: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct WhisperOutput {
    pub langauge: String,
    pub confidence: f32,
    pub transcript: Vec<(f32, f32, String)>,
}

impl Debug for WhisperOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let transcript = self
            .transcript
            .iter()
            .map(|(start, end, text)| format!("{:?} - {:?}: {:?}", start, end, text))
            .collect::<Vec<_>>()
            .join("\n");
        f.write_fmt(format_args!(
            "WhisperOutput {{ langauge: {}, confidence: {}, transcript: {{ \n{}\n}} }}",
            self.langauge, self.confidence, transcript
        ))
    }
}

impl Whisper {
    pub fn parse(output: &str) -> Result<WhisperOutput, WhisperError> {
        let lines = output.lines().collect::<Vec<_>>();

        let langauge = lines[0].to_string();
        let confidence = lines[1]
            .parse::<f32>()
            .map_err(|_| WhisperError::OutputParseError)?;

        let mut transcript = Vec::new();

        for line in lines.iter().skip(2) {
            // <start>|<end>|<text>
            let parts = line.split('|').collect::<Vec<_>>();
            let start = parts[0]
                .parse::<f32>()
                .map_err(|_| WhisperError::OutputParseError)?;
            let end = parts[1]
                .parse::<f32>()
                .map_err(|_| WhisperError::OutputParseError)?;
            let text = parts[2].to_string();
            transcript.push((start, end, text));
        }

        Ok(WhisperOutput {
            langauge,
            confidence,
            transcript,
        })
    }

    pub async fn store(
        output: WhisperOutput,
        db: &mut impl AcquireClone,
        media: &mut Media,
    ) -> Result<(), WhisperError> {
        let extra = media.extra(db.acquire_clone()).await?;

        let create = extra.is_none();

        let mut media_extra = extra.unwrap_or_default();

        media_extra.media_id = media.id;
        media_extra.whisper_version = VERSION;
        media_extra.whisper_language = Some(output.langauge);
        media_extra.whisper_confidence = Some(output.confidence);
        media_extra.whisper_transcript = Some(
            serde_json::to_string(&output.transcript)
                .map_err(|_| WhisperError::OutputParseError)?,
        );

        if create {
            media_extra.create_no_bug(db.acquire_clone()).await?;
        } else {
            media_extra.update_by_id(db.acquire_clone()).await?;
        }

        Ok(())
    }

    pub async fn whisper(
        &self,
        target: &str,
        scripts_dir: &str,
        data_dir: &str,
        python_path: &str,
    ) -> Result<WhisperOutput, WhisperError> {
        let script_path = Path::new(scripts_dir).join(WHISPER_SCRIPT);
        let download_root = Path::new(data_dir).join(DOWNLOAD_ROOT);

        let whisper_output = run_python(
            python_path,
            script_path.to_str().unwrap(),
            &[
                self.config.model.as_str(),
                self.config.device.as_str(),
                self.config.compute_type.as_str(),
                download_root.to_str().unwrap(),
                target,
            ],
        )?;

        // delete the temporary file
        tokio::fs::remove_file(&target).await?;

        if !whisper_output.status.success() {
            let output = String::from_utf8(whisper_output.stderr)
                .map_err(|_| WhisperError::OutputParseError)?;
            return Err(WhisperError::WhisperError(output));
        }

        let stdout =
            String::from_utf8(whisper_output.stdout).map_err(|_| WhisperError::OutputParseError)?;

        Self::parse(&stdout)
    }
}

impl BackgroundTask for Whisper {
    type Error = WhisperError;
    const NAME: &'static str = "transcribe_whisper";

    type Data = WhisperOutput;
    type Config = WhisperConfig;

    async fn new(
        db: &mut impl AcquireClone,
        config: &Self::Config,
        app_config: &AppConfig,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            config: config.clone(),
            data_dir: PathBuf::from(&app_config.data_dir)
                .join(MODEL_DIR)
                .join(WHISPER_DIR),
            app_config: app_config.clone(),
        })
    }

    async fn compatible(media: &Media) -> bool {
        let path = PathBuf::from(&media.path);
        let format = AnyFormat::try_new(path);
        if let Some(format) = format {
            return format.audioable();
        }
        false
    }

    async fn outdated(
        &self,
        db: &mut impl AcquireClone,
        media: &Media,
    ) -> Result<bool, Self::Error> {
        let whisper_extra = media.extra(db.acquire_clone()).await?;
        if let Some(whisper_extra) = whisper_extra {
            if whisper_extra.whisper_version >= VERSION {
                return Ok(false);
            }
        }
        Ok(true)
    }

    async fn run(
        &self,
        db: &mut impl AcquireClone,
        media: &Media,
    ) -> Result<Self::Data, Self::Error> {
        let format = AnyFormat::try_new(PathBuf::from(&media.path))
            .expect("media format is not, you should have checked it was compatible");

        let tmp_name = format!("{}.mp3", Uuid::new_v4());
        let to_path = std::env::temp_dir().join(tmp_name);

        // convert to mp3
        let output = format.convert_to_mp3(&to_path, &self.app_config)?;

        if !output.status.success() {
            let output =
                String::from_utf8(output.stderr).map_err(|_| WhisperError::OutputParseError)?;
            return Err(WhisperError::ConversionError(output));
        }

        self.whisper(
            to_path.to_str().unwrap(),
            &self.app_config.scripts_dir,
            &self.app_config.data_dir,
            &self.app_config.python_path,
        )
        .await
    }

    async fn run_and_store(
        &self,
        db: &mut impl AcquireClone,
        media: &mut Media,
    ) -> Result<(), Self::Error> {
        let output = self.run(db, media).await?;
        Self::store(output, db, media).await
    }

    async fn remove_data(
        &self,
        db: &mut impl AcquireClone,
        media: &mut Media,
    ) -> Result<(), Self::Error> {
        let whisper_extra = media.extra(db.acquire_clone()).await?;
        if let Some(mut whisper_extra) = whisper_extra {
            whisper_extra.whisper_transcript = None;
            whisper_extra.whisper_language = None;
            whisper_extra.whisper_confidence = None;
            whisper_extra.whisper_version = -1;
            whisper_extra.update_by_id(db.acquire_clone()).await?;
        }
        Ok(())
    }
}

impl RemoteBackgroundTask for Whisper {
    type RemoteClientConfig = StandardClientConfig;
    type RunnerConfig = WhisperConfig;

    async fn new_remote(
        db: &mut impl AcquireClone,
        runner_config: &Self::RunnerConfig,
        remote_server_config: &RemoteRunnerConfig,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            config: runner_config.clone(),
            data_dir: Default::default(),
            app_config: Default::default(),
        })
    }

    async fn remote_handler(
        &self,
        request: Request,
        db: impl AcquireClone + Send + 'static,
        runner_config: &Self::RunnerConfig,
        remote_server_config: &RemoteRunnerConfig,
    ) -> Result<Response, ErrorResponse> {
        let mut multipart = MultipartHelper::try_from_request(request).await?;
        let media_uuid = Uuid::from_str(&multipart.text("media_uuid").await?).map_err(|_| (StatusCode::BAD_REQUEST, "bad media_uuid"))?;
        let (audio_file, _) = multipart.file("audio", ".mp3").await?;

        let duration = Audio::duration(&audio_file).map_err(|_| (StatusCode::BAD_REQUEST, "bad audio_file"))?;

        let now = Utc::now();
        let estimate = now.add(TimeDelta::seconds(duration.round() as i64));

        // TODO: there are almost definitely better ways to do this (we could Arc self) but clone should be relatively cheap here
        let this = self.clone();
        let remote_server_config = remote_server_config.clone();

        let job = start_job(Self::NAME.to_string(), media_uuid, Some(estimate.naive_utc()), db, |_| {
            async move {
                let out = this
                    .whisper(
                        audio_file.to_str().unwrap(),
                        &remote_server_config.scripts_dir,
                        &remote_server_config.data_dir,
                        &remote_server_config.python_path,
                    )
                    .await.map_err(|e| format!("whisper error: {:?}", e))?;
                Ok(Some(out))
            }
        }).await.map_err(internal)?;

        Ok((StatusCode::CREATED, job.uuid.to_string()).into_response())
    }

    async fn run_remote(
        &self,
        db: &mut impl AcquireClone,
        media: &Media,
        remote_config: &Self::RemoteClientConfig,
    ) -> Result<Self::Data, Self::Error> {
        let format = AnyFormat::try_new(PathBuf::from(&media.path))
            .expect("media format is not, you should have checked it was compatible");
        let tmp_name = format!("{}.mp3", Uuid::new_v4());
        let to_path = std::env::temp_dir().join(tmp_name);

        // convert to mp3
        let output = format.convert_to_mp3(&to_path, &self.app_config)?;

        if !output.status.success() {
            let output =
                String::from_utf8(output.stderr).map_err(|_| WhisperError::OutputParseError)?;
            return Err(WhisperError::ConversionError(output));
        }

        let client = RemoteRequester::new(Self::NAME.to_string(), remote_config.remote.url.clone(), remote_config.remote.password.clone());
        let res = client.one_shot_file("audio".to_string(), &to_path, Some(media.uuid)).await?;
        match res {
            OneShotResponse::Job(job) => {
                if job.status != JobStatus::Success {
                    return Err(WhisperError::JobError(job))
                }
                let data = serde_json::from_str(&job.success_data.expect("missing success data")).expect("unable to parse success data");
                Ok(data)
            }
            OneShotResponse::Response(res) => Err(WhisperError::UnexpectedResponse(res))
        }
    }

    async fn run_remote_and_store(
        &self,
        db: &mut impl AcquireClone,
        media: &mut Media,
        remote_config: &Self::RemoteClientConfig,
    ) -> Result<(), Self::Error> {
        let out = self.run_remote(db, media, remote_config).await?;
        Self::store(out, db, media).await
    }
}

#[derive(Debug, thiserror::Error)]
pub enum WhisperError {
    #[error("metadata error: {0}")]
    MetadataError(#[from] MetadataError),
    #[error("conversion error: {0}")]
    ConversionError(String),
    #[error("output parse error")]
    OutputParseError,
    #[error("python error: {0}")]
    PythonError(#[from] std::io::Error),
    #[error("whisper error: {0}")]
    WhisperError(String),
    #[error("sqlx error: {0}")]
    SqlxError(#[from] sqlx::Error),
    #[error("job error: {0:?}")]
    JobError(Job),
    #[error("unexpected response {0:?}")]
    UnexpectedResponse(reqwest::Response),
    #[error("request error: {0}")]
    RequestError(#[from] RequestError)
}
