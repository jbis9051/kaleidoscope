use std::path::PathBuf;
use common::media_processors::format::{AnyFormat, MetadataError};
use common::media_processors::RgbImage;
use common::models::media::Media;
use common::scan_config::AppConfig;
use common::types::{AcquireClone};
use log::debug;
use crate::tasks::{BackgroundTask};

pub struct ThumbnailGenerator {
    config: VideoDurationProcessorConfig,
    data_dir: PathBuf,
    app_config: AppConfig
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Default, Clone)]
pub struct VideoDurationProcessorConfig {
    pub thumb_size: u32,
}

impl ThumbnailGenerator {
    fn thumb_path(&self, media: &Media) -> PathBuf {
        self.data_dir.join(format!("{:?}-thumb.jpg", media.uuid))
    }
    
    fn full_path(&self, media: &Media) -> PathBuf {
        self.data_dir.join(format!("{:?}-full.jpg", media.uuid))
    }
}

impl BackgroundTask for ThumbnailGenerator {
    type Error = MetadataError;
    const NAME: &'static str = "thumbnail";
    const VERSION: u32 = 0;

    type Data = (RgbImage, RgbImage, i32);
    type Config = VideoDurationProcessorConfig;

    async fn new(db: &mut impl AcquireClone, config: &Self::Config, app_config: &AppConfig) -> Result<Self, Self::Error> {
        Ok(ThumbnailGenerator {
            config: config.clone(),
            data_dir: PathBuf::from(&app_config.data_dir),
            app_config: app_config.clone(),
        })
    }


    async fn compatible(media: &Media) -> bool {
        let path = PathBuf::from(&media.path);
        let format = AnyFormat::try_new(path);
        if let Some(format) = format {
            return format.thumbnailable();
        }
        false
    }

    async fn outdated(&self, db: &mut impl AcquireClone, media: &Media) -> Result<bool, Self::Error> {
        let path = PathBuf::from(&media.path);
        let format = AnyFormat::try_new(path).expect("media format is not, you should have checked it was compatible");
        // if media doesn't have a thumbnail, or the thumbnail version is less than the media thumbnail version, or the format has changed, we need to update
        Ok(!media.has_thumbnail || format.thumbnail_version() > media.thumbnail_version || media.format != format.format_type())
    }

    async fn run(&self, db: &mut impl AcquireClone, media: &Media) -> Result<Self::Data, Self::Error> {
        let path = PathBuf::from(&media.path);
        let format = AnyFormat::try_new(path).expect("media format is not, you should have checked it was compatible");

        let metadata = format.get_metadata(&self.app_config)?;

        // we want to generate a thumbnail while maintaining the aspect ratio, using thumb_size as the max size

        let mut twidth = self.config.thumb_size;
        let mut theight = self.config.thumb_size;
        
        if metadata.width > 0 && metadata.height > 0 {
            if metadata.width > metadata.height {
                theight = (metadata.height as f32 / metadata.width as f32 * twidth as f32) as u32;
            } else {
                twidth = (metadata.width as f32 / metadata.height as f32 * theight as f32) as u32;
            }
        }

        let thumbnail = format.generate_thumbnail(twidth, theight, &self.app_config)?;
        let full = format.generate_full(&self.app_config)?;

        Ok((thumbnail, full, format.thumbnail_version()))
    }

    async fn run_and_store(&self, db: &mut impl AcquireClone, media: &mut Media) -> Result<(), Self::Error> {
        let (thumbnail, full, thumbnail_version) = self.run(db, media).await?;

        let thumb_path = self.thumb_path(media);
        let full_path = self.full_path(media);

        // write thumbnail to disk
        debug!("          writing thumbnail: {:?}", thumb_path);
        thumbnail.save(thumb_path).unwrap();

        // write full to disk
        debug!("          writing full: {:?}", full_path);
        full.save(full_path).unwrap();
        
        media.has_thumbnail = true;
        media.thumbnail_version = thumbnail_version;
        media.update_by_id(db.acquire_clone()).await.unwrap();

        Ok(())
    }

    async fn remove_data(&self, db: &mut impl AcquireClone, media: &mut Media) -> Result<(), Self::Error> {
        let thumb_path = self.thumb_path(media);
        let full_path = self.full_path(media);
        
        // TODO: handle errors
        std::fs::remove_file(thumb_path);
        std::fs::remove_file(full_path);
        
        media.has_thumbnail = false;
        media.thumbnail_version = -1;
        media.update_by_id(db.acquire_clone()).await.unwrap();
        
        Ok(())
    }
}