use common::format_type::FormatType;
use common::models::media::Media;
use common::types::{SqliteAcquire};
use crate::tasks::{BackgroundTask};

pub struct VideoDurationProcessor;

impl BackgroundTask for VideoDurationProcessor {
    type Error = String;
    const NAME: &'static str = "video_duration";
    const VERSION: u32 = 0;

    type Data = String;

    async fn new(db: impl SqliteAcquire<'_>) -> Result<Self, Self::Error> {
        Ok(Self)
    }


    async fn compatible(media: &Media) -> bool {
        matches!(media.format, FormatType::Video)
    }

    async fn needs_update(&self, db: impl SqliteAcquire<'_>, media: &Media) -> bool {
        false
    }

    async fn run(&self, db: impl SqliteAcquire<'_>, media: &Media) -> Result<Self::Data, Self::Error> {
        Ok(media.duration.expect("no duration").to_string())
    }


    async fn run_and_store(&self, db: impl SqliteAcquire<'_>, media: &Media) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn remove_data(&self, db: impl SqliteAcquire<'_>, media: &Media) -> Result<(), Self::Error> {
        Ok(())
    }
}