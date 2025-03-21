use ffmpeg_next::codec::Context;
use ffmpeg_next::format::Pixel;
use ffmpeg_next::software::scaling::{context::Context as ScaleContext, flag::Flags};
use ffmpeg_next::util::frame::video::Video as VideoFrame;
use image::RgbImage;
use nom_exif::{MediaParser, MediaSource, TrackInfo};
use std::path::Path;
use std::time::Duration;
use crate::media_processors::exif::extract_exif_nom;
use crate::media_processors::format::{resize_dimensions, Format, FormatType, MediaMetadata, Thumbnailable};
use crate::models::system_time_to_naive_datetime;

pub struct Video;

impl Format<VideoError> for Video {
    const FORMAT_TYPE: FormatType = FormatType::Video;
    const EXTENSIONS: &'static [&'static str] = &["mp4", "mov"];
    const METADATA_VERSION: i32 = 2;
    fn is_photo() -> bool {
        false
    }

    fn get_metadata(path: &Path) -> Result<MediaMetadata, VideoError> {
        let file_meta = path.metadata()?;
        ffmpeg_next::init().unwrap();
        let context = ffmpeg_next::format::input(&path)?;
        let stream = context
            .streams()
            .best(ffmpeg_next::media::Type::Video)
            .ok_or(VideoError::FfmpegError(ffmpeg_next::Error::StreamNotFound))?;
        let codec = Context::from_parameters(stream.parameters())?;
        let meta = codec.decoder().video()?;

        let metadata = {
            let ms = MediaSource::file_path(path)?;
            if !ms.has_track() {
                None
            } else {
                let mut parser = MediaParser::new();
                let exif: TrackInfo = parser.parse(ms)?;

                Some(exif)
            }
        }.map(|e| extract_exif_nom(&e));
        
        let seconds = stream.duration() as f64 * f64::from(stream.time_base());
        let milliseconds = (seconds * 1000.0).round() as u64;

        Ok(MediaMetadata {
            name: path.file_name().unwrap().to_string_lossy().to_string(),
            width: meta.width(),
            height: meta.height(),
            duration: Some(Duration::from_millis(milliseconds)),
            created_at: system_time_to_naive_datetime(file_meta.created().unwrap()),
            size: file_meta.len() as u32,
            latitude: metadata.as_ref().and_then(|e| e.latitude),
            longitude: metadata.as_ref().and_then(|e| e.longitude),
            is_screenshot: metadata.as_ref().map(|e| e.is_screenshot).unwrap_or(false),
        })
    }

}

impl Thumbnailable<VideoError> for Video {
    const THUMBNAIL_VERSION: i32 = 1;
    fn generate_thumbnail(path: &Path, width: u32, height: u32) -> Result<RgbImage, VideoError> {
        ffmpeg_next::init().unwrap();
        let mut context = ffmpeg_next::format::input(&path)?;
        let stream = context
            .streams()
            .best(ffmpeg_next::media::Type::Video)
            .ok_or(VideoError::FfmpegError(ffmpeg_next::Error::StreamNotFound))?;
        let codec = Context::from_parameters(stream.parameters())?;
        let mut decoder = codec.decoder().video()?;

        let mut decoded = VideoFrame::empty();
        let stream_index = stream.index();

        for (stream, packet) in context.packets() {
            if stream.index() == stream_index {
                decoder.send_packet(&packet)?;
                if decoder.receive_frame(&mut decoded).is_ok() {
                    break;
                }
            }
        }

        // https://stackoverflow.com/a/69161058/7886229
        // Round to the next 32bit divisible width
        let round_width = if decoder.width() % 32 != 0 {
            decoder.width() + 32 - (decoder.width() % 32)
        } else {
            decoder.width()
        };

        let mut scaler = ScaleContext::get(
            decoder.format(),
            decoder.width(),
            decoder.height(),
            Pixel::RGB24,
            round_width,
            decoder.height(),
            Flags::FAST_BILINEAR,
        )?;

        let mut rgb_frame = VideoFrame::empty();
        scaler.run(&decoded, &mut rgb_frame)?;

        let rgb_image = RgbImage::from_raw(
            round_width,
            decoder.height(),
            rgb_frame.data(0).to_vec(),
        )
            .unwrap();

        let (nw, nh) = resize_dimensions(round_width, decoder.height(), width, height, false);

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
    #[error("nom exif error: {0}")]
    NomExifError(#[from] nom_exif::Error),
}
