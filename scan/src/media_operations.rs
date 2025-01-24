use std::path::Path;
use image::RgbImage;
use log::{debug, error};
use sqlx::SqliteConnection;
use sqlx::types::chrono::Utc;
use sqlx::types::Uuid;
use common::models::media::Media;
use common::models::system_time_to_naive_datetime;
use common::scan_config::AppConfig;
use crate::format::{heif, raw, standard, video, AnyFormat, Format, MediaMetadata, MetadataError};
use crate::{hash, remove_media};

pub async fn add_media(path: &Path, config: &AppConfig, db: &mut SqliteConnection) -> Result<(), AddMediaError> {
    let format = AnyFormat::new(path.to_path_buf()).ok_or(AddMediaError::UnsupportedFormat)?;
    
    let file_created_at = system_time_to_naive_datetime(path.metadata()?.created()?);
    let path_str = path.canonicalize()?.to_string_lossy().to_string();

    // do a cheap check immediately to see if the media already exists
    if let Some(mut media) = Media::from_path(&mut *db, &path_str).await.unwrap() {
        let file_size = path.metadata()?.len() as u32;
        if media.file_created_at == file_created_at && media.size == file_size {
            return Err(AddMediaError::AlreadyExists(1));
        }
        remove_media(&mut media, db, config).await; // file has changed, remove the old media
    }
    
    let metadata = format.get_metadata()?;
    
    if let Some(mut media) = Media::from_path(&mut *db, &path_str).await.unwrap() {
        if media.created_at == metadata.created_at && media.size == metadata.size {
            // this shouldn't really happen, but it could if (1) there's a different date in the media metadata as opposed to the file metadata and (2) the file was modified while keeping the file metadata the same (including the size)
            return Err(AddMediaError::AlreadyExists(2));
        }
        remove_media(&mut media, db, config).await;
    }

    let uuid = Uuid::new_v4();
    
    generate_thumbnails(&uuid, &format, config)?;
    
    let hash = hash(path);
    let is_photo = format.is_photo();

    let mut media = Media {
        id: 0,
        uuid,
        name: metadata.name,
        created_at: metadata.created_at,
        width: metadata.width,
        height: metadata.height,
        size: metadata.size,
        path: path_str.to_string(),
        liked: false,
        is_photo,
        added_at: Utc::now().naive_utc(),
        duration: metadata.duration.map(|d| d.as_secs() as u32),
        hash,
        file_created_at,
        longitude: metadata.longitude,
        latitude: metadata.latitude,
        metadata_version: format.metadata_version(),
        thumbnail_version: format.thumbnail_version(),
    };

    media.create(&mut *db).await.unwrap();
    Ok(())
}


#[derive(thiserror::Error, Debug)]
pub enum AddMediaError {
    #[error("iO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("media already exists [check {0}]")]
    AlreadyExists(u8),
    #[error("unsupported format")]
    UnsupportedFormat,
    #[error("error getting metadata: {0}")]
    Metadata(#[from] MetadataError),
}


pub fn generate_thumbnails(uuid: &Uuid, format: &AnyFormat, config: &AppConfig) -> Result<(), AddMediaError> {
    let data_dir = Path::new(&config.data_dir);
    let thumb_path = data_dir.join(format!("{:?}-thumb.jpg", uuid));
    let full_path = data_dir.join(format!("{:?}-full.jpg", uuid));
    
    let metadata = format.get_metadata()?;

    // we want to generate a thumbnail while maintaining the aspect ratio, using thumb_size as the max size

    let mut twidth = config.thumb_size;
    let mut theight = config.thumb_size;

    if metadata.width > metadata.height {
        theight = (metadata.height as f32 / metadata.width as f32 * twidth as f32) as u32;
    } else {
        twidth = (metadata.width as f32 / metadata.height as f32 * theight as f32) as u32;
    }

    let thumbnail = format.generate_thumbnail(twidth, theight)?;
    let full = format.generate_full()?;
    
    // write thumbnail to disk
    debug!("          writing thumbnail: {:?}", thumb_path);
    thumbnail.save(thumb_path).unwrap();

    // write full to disk
    debug!("          writing full: {:?}", full_path);
    full.save(full_path).unwrap();
    
    Ok(())
}
