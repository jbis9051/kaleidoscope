use std::collections::HashMap;
use std::path::Path;
use log::{debug, error};
use sqlx::SqliteConnection;
use sqlx::types::chrono::Utc;
use sqlx::types::Uuid;
use common::media_processors::format::{AnyFormat, MetadataError};
use common::models::media::Media;
use common::models::system_time_to_naive_datetime;
use common::scan_config::AppConfig;
use tasks::ops::add_to_compatible_queues;
use tasks::tasks::{BackgroundTask, AnyTask, Task};
use tasks::tasks::thumbnail::ThumbnailGenerator;

pub async fn add_media(path: &Path, config: &AppConfig, import_id: i32, media_map: &mut HashMap<String, Media>, db: &mut SqliteConnection) -> Result<(), AddMediaError> {
    let format = AnyFormat::try_new(path.to_path_buf()).ok_or(AddMediaError::UnsupportedFormat)?;

    let file_created_at = system_time_to_naive_datetime(path.metadata()?.created()?);
    let path_str = path.canonicalize()?.to_string_lossy().to_string();

    // do a cheap check immediately to see if the media already exists
    if let Some(media) = media_map.get(&path_str) {
        let file_size = path.metadata()?.len() as u32;
        if media.file_created_at == file_created_at && media.size == file_size {
            return Err(AddMediaError::AlreadyExists(1));
        }
        remove_media(&media, db, config).await; // file has changed, remove the old media
        media_map.remove(&path_str);
    }

    let metadata = format.get_metadata(config)?;

    if let Some(media) = media_map.get(&path_str) {
        if media.created_at == metadata.created_at && media.size == metadata.size {
            // this shouldn't really happen, but it could if (1) there's a different date in the media metadata as opposed to the file metadata and (2) the file was modified while keeping the file metadata the same (including the size)
            return Err(AddMediaError::AlreadyExists(2));
        }
        remove_media(&media, db, config).await;
        media_map.remove(&path_str);
    }

    let uuid = Uuid::new_v4();

    let hash = hash(path);

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
        media_type: metadata.media_type,
        added_at: Utc::now().naive_utc(),
        duration: metadata.duration.map(|d| d.as_millis() as u32),
        hash,
        file_created_at,
        is_screenshot: metadata.is_screenshot,
        longitude: metadata.longitude,
        latitude: metadata.latitude,
        has_thumbnail: false,
        format: format.format_type(),
        metadata_version: format.metadata_version(),
        thumbnail_version: -1,
        import_id,
    };

    media.create(&mut *db).await.unwrap();
    media_map.insert(path_str.to_string(), media.clone());

    add_to_compatible_queues(&mut *db, &media, &AnyTask::TASK_NAMES).await.unwrap();

    Ok(())
}

pub async fn update_media(media: &mut Media, config: &AppConfig, db: &mut SqliteConnection) -> Result<(), AddMediaError> {
    let path = Path::new(&media.path);
    let format = AnyFormat::try_new(path.to_path_buf()).ok_or(AddMediaError::UnsupportedFormat)?;

    let format_change = media.format != format.format_type();

    if format_change {
        debug!("          updating format for {:?}: {:?} --> {:?}", media.uuid, media.format, format.format_type());
        media.format = format.format_type();
    }

    // if format has changed, we need to update metadata and thumbnail regardless of version
    
    if media.metadata_version < format.metadata_version() || format_change {
        debug!("          updating metadata for {:?}: {} --> {}", media.uuid, media.metadata_version, format.metadata_version());
        let metadata = format.get_metadata(config)?;
        media.width = metadata.width;
        media.height = metadata.height;
        media.size = metadata.size;
        media.duration = metadata.duration.map(|d| d.as_millis() as u32);
        media.longitude = metadata.longitude;
        media.latitude = metadata.latitude;
        media.is_screenshot = metadata.is_screenshot;
        media.metadata_version = format.metadata_version();
    }

    // we only add to the thumbnail queue if the format has changed, thumbnail version checking is handled by the ThumbnailGenerator task itself in a later step
    if format_change {
        let added = add_to_compatible_queues(&mut *db, &media, &[ThumbnailGenerator::NAME]).await.unwrap();
        if added.len() > 0 {
            debug!("          add media to thumbnail queue for update for {:?} due to format change: {} --> {}", media.uuid, media.thumbnail_version, format.thumbnail_version());
            media.has_thumbnail = false;
        }
    }

    media.update_by_id(&mut *db).await.unwrap();

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

pub async fn remove_media(media: &Media, db: &mut SqliteConnection, config: &AppConfig) {
    media.delete(&mut *db).await.unwrap();
    let thumb = Path::new(&config.data_dir).join(format!("{:?}-thumb.jpg", media.uuid));
    let full = Path::new(&config.data_dir).join(format!("{:?}-full.jpg", media.uuid));
    std::fs::remove_file(thumb);
    std::fs::remove_file(full);
}

pub fn hash(path: &Path) -> String {
    let mut hasher = blake3::Hasher::new();
    let mut file = std::fs::File::open(path).unwrap();
    std::io::copy(&mut file, &mut hasher).unwrap();
    hasher.finalize().to_hex().to_string()
}


