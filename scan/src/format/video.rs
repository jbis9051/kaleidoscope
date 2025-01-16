use std::path::Path;
use std::time::Duration;
use ffmpeg_next::codec::Context;
use ffmpeg_next::format::Pixel;
use image::{RgbImage};
use walkdir::DirEntry;
use common::models::system_time_to_naive_datetime;
use crate::format::{Format, MediaMetadata, resize_dimensions};
use ffmpeg_next::software::scaling::{context::Context as ScaleContext, flag::Flags};
use ffmpeg_next::util::frame::video::Video as VideoFrame;

pub struct Video;

impl Format<VideoError> for Video {
    const EXTENSIONS: &'static [&'static str] = &["mp4", "mov"];

    fn is_photo() -> bool {
        false
    }
    
    fn get_metadata(path: &Path) -> Result<MediaMetadata, VideoError> {
        let file_meta = path.metadata()?;
        ffmpeg_next::init().unwrap();
        let context = ffmpeg_next::format::input(&path)?;
        let stream = context.streams().best(ffmpeg_next::media::Type::Video).ok_or(VideoError::FfmpegError(ffmpeg_next::Error::StreamNotFound))?;
        let codec = Context::from_parameters(stream.parameters())?;
        let meta = codec.decoder().video()?;

        Ok(MediaMetadata {
            name: path.file_name().unwrap().to_string_lossy().to_string(),
            width: meta.width(),
            height: meta.height(),
            duration: Some(Duration::from_secs(stream.duration() as u64)),
            created_at: system_time_to_naive_datetime(file_meta.created().unwrap()),
            size: file_meta.len() as u32,
        })
    }

    fn generate_thumbnail(path: &Path, width: u32, height: u32) -> Result<RgbImage, VideoError> {
        ffmpeg_next::init().unwrap();
        let mut context = ffmpeg_next::format::input(&path)?;
        let stream = context.streams().best(ffmpeg_next::media::Type::Video).ok_or(VideoError::FfmpegError(ffmpeg_next::Error::StreamNotFound))?;
        let codec = Context::from_parameters(stream.parameters())?;
        let mut decoder = codec.decoder().video()?;
        
        let mut decoded = VideoFrame::empty();
        let stream_index= stream.index();

        for (stream, packet) in context.packets() {
            if stream.index() == stream_index {
                decoder.send_packet(&packet)?;
                if decoder.receive_frame(&mut decoded).is_ok() {
                    break;
                }
            }
        }

        let mut scaler = ScaleContext::get(
            decoder.format(),
            decoder.width(),
            decoder.height(),
            Pixel::RGB24,
            decoder.width(),
            decoder.height(),
            Flags::FAST_BILINEAR,
        )?;

        let mut rgb_frame = VideoFrame::empty();
        scaler.run(&decoded, &mut rgb_frame)?;

        let rgb_image = RgbImage::from_raw(decoder.width(), decoder.height(), rgb_frame.data(0).to_vec()).unwrap();
        let (nw, nh) = resize_dimensions(decoder.width(), decoder.height(), width, height, false);
        
        let thumbnail = image::imageops::thumbnail(&rgb_image, nw, nh);

        Ok(thumbnail)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum VideoError {
    #[error("ffmpeg error: {0}")]
    FfmpegError(#[from] ffmpeg_next::Error),
    #[error("iO error: {0}")]
    IoError(#[from] std::io::Error),
}