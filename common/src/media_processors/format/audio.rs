use std::path::Path;
use std::time::Duration;
use crate::media_processors::format::{Audioable, Format, FormatType, MediaMetadata, MediaType};
use crate::models::system_time_to_naive_datetime;
use crate::scan_config::AppConfig;

pub struct Audio;

impl Audio {
    pub fn duration<T: AsRef<Path>>(path: T) -> Result<f64, AudioError> {
        ffmpeg_next::init().unwrap();
        let context = ffmpeg_next::format::input(&path)?;
        let stream = context
            .streams()
            .best(ffmpeg_next::media::Type::Audio)
            .ok_or(AudioError::FfmpegError(ffmpeg_next::Error::StreamNotFound))?;

        Ok(stream.duration() as f64 * f64::from(stream.time_base()))
    }
}

impl Format for Audio {
    type Error = AudioError;
    const FORMAT_TYPE: FormatType = FormatType::Audio;
    const EXTENSIONS: &'static [&'static str] = &["mp3", "wav", "flac", "ogg", "m4a", "aac", "wma", "aiff", "alac", "m4a"];
    const METADATA_VERSION: i32 = 1;

    fn get_metadata(path: &Path, _: &AppConfig) -> Result<MediaMetadata, Self::Error> {
        let file_meta = path.metadata()?;
        
        let seconds = Self::duration(path)?;
        let milliseconds = (seconds * 1000.0).round() as u64;


        Ok(MediaMetadata {
            name: path.file_name().unwrap().to_string_lossy().to_string(),
            width: 0,
            height: 0,
            size: file_meta.len() as u32,
            created_at: system_time_to_naive_datetime(file_meta.created().unwrap()),
            duration: Some(Duration::from_millis(milliseconds)),
            longitude: None,
            latitude: None,
            is_screenshot: false,
            media_type: MediaType::Audio,
        })
    }
}

impl Audioable for Audio {}

#[derive(thiserror::Error, Debug)]
pub enum AudioError {
    #[error("iO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("ffmpeg error: {0}")]
    FfmpegError(#[from] ffmpeg_next::Error),
}
