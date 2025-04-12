use crate::run_python::run_python;
use crate::tasks::{BackgroundTask, MODEL_DIR};
use common::media_processors::format::{AnyFormat, MetadataError};
use common::media_processors::RgbImage;
use common::models::media::Media;
use common::models::media_extra::MediaExtra;
use common::scan_config::AppConfig;
use common::types::AcquireClone;
use log::debug;
use serde::{Deserialize, Serialize};
use sqlx::types::uuid;
use std::fmt::{Debug, Pointer};
use std::path::{Path, PathBuf};
use std::time::Duration;
use uuid::Uuid;

// where the transcription files are stored
const WHISPER_DIR: &str = "whisper";

// where the models are stored
const DOWNLOAD_ROOT: &str = "whisper_root";

// the script to run
const WHISPER_SCRIPT: &str = "fw-transcribe.py";

const VERSION: i32 = 0;

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

        let script_path = Path::new(&self.app_config.scripts_dir).join(WHISPER_SCRIPT);
        let download_root = Path::new(&self.app_config.data_dir).join(DOWNLOAD_ROOT);

        let whisper_output = run_python(
            &self.app_config.python_path,
            script_path.to_str().unwrap(),
            &[
                self.config.model.as_str(),
                self.config.device.as_str(),
                self.config.compute_type.as_str(),
                download_root.to_str().unwrap(),
                to_path.to_str().unwrap(),
            ],
        )?;
        
        // delete the temporary file
        std::fs::remove_file(&to_path)?;

        if !whisper_output.status.success() {
            let output = String::from_utf8(whisper_output.stderr)
                .map_err(|_| WhisperError::OutputParseError)?;
            return Err(WhisperError::WhisperError(output));
        }

        let stdout =
            String::from_utf8(whisper_output.stdout).map_err(|_| WhisperError::OutputParseError)?;
        let lines = stdout.lines().collect::<Vec<_>>();

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

    async fn run_and_store(
        &self,
        db: &mut impl AcquireClone,
        media: &mut Media,
    ) -> Result<(), Self::Error> {
        let output = self.run(db, media).await?;
        let mut whisper_extra = MediaExtra {
            id: 0,
            media_id: media.id,
            whisper_version: VERSION,
            whisper_language: Some(output.langauge),
            whisper_confidence: Some(output.confidence),
            whisper_transcript: Some(
                serde_json::to_string(&output.transcript)
                    .map_err(|_| WhisperError::OutputParseError)?,
            ),
        };
        whisper_extra.create_no_bug(db.acquire_clone()).await?;
        Ok(())
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
}
