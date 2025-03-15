use std::path::Path;
use image::RgbImage;
use log::{debug, error};
use sha1::Digest;
use sqlx::SqliteConnection;
use sqlx::types::chrono::Utc;
use sqlx::types::Uuid;
use common::models::media::Media;
use common::models::system_time_to_naive_datetime;
use common::scan_config::AppConfig;
use crate::format::{heif, raw, standard, video, AnyFormat, Format, MediaMetadata, MetadataError};

pub async fn add_media(path: &Path, config: &AppConfig, import_id: i32, db: &mut SqliteConnection) -> Result<(), AddMediaError> {
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
        duration: metadata.duration.map(|d| d.as_millis() as u32),
        hash,
        file_created_at,
        is_screenshot: metadata.is_screenshot,
        longitude: metadata.longitude,
        latitude: metadata.latitude,
        format: format.format_type(),
        metadata_version: format.metadata_version(),
        thumbnail_version: format.thumbnail_version(),
        import_id,
    };

    media.create(&mut *db).await.unwrap();
    Ok(())
}

pub async fn update_media(media: &mut Media, config: &AppConfig, db: &mut SqliteConnection) -> Result<(), AddMediaError> {
    let path = Path::new(&media.path);
    let format = AnyFormat::new(path.to_path_buf()).ok_or(AddMediaError::UnsupportedFormat)?;

    let format_change = media.format != format.format_type();

    if format_change {
        debug!("          updating format for {:?}: {:?} --> {:?}", media.uuid, media.format, format.format_type());
        media.format = format.format_type();
    }

    // if format has changed, we need to update metadata and thumbnail regardless of version
    
    if media.metadata_version < format.metadata_version() || format_change {
        debug!("          updating metadata for {:?}: {} --> {}", media.uuid, media.metadata_version, format.metadata_version());
        let metadata = format.get_metadata()?;
        media.width = metadata.width;
        media.height = metadata.height;
        media.size = metadata.size;
        media.duration = metadata.duration.map(|d| d.as_millis() as u32);
        media.longitude = metadata.longitude;
        media.latitude = metadata.latitude;
        media.is_screenshot = metadata.is_screenshot;
        media.metadata_version = format.metadata_version();
    }

    if media.thumbnail_version < format.thumbnail_version() || format_change {
        debug!("          updating thumbnail for {:?}: {} --> {}", media.uuid, media.thumbnail_version, format.thumbnail_version());
        generate_thumbnails(&media.uuid, &format, config)?;
        media.thumbnail_version = format.thumbnail_version();
    }

    media.update_by_id(&mut *db).await.unwrap();

    Ok(())
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

pub async fn remove_media(media: &mut Media, db: &mut SqliteConnection, config: &AppConfig) {
    media.delete(&mut *db).await.unwrap();
    let thumb = Path::new(&config.data_dir).join(format!("{:?}-thumb.jpg", media.uuid));
    let full = Path::new(&config.data_dir).join(format!("{:?}-full.jpg", media.uuid));
    std::fs::remove_file(thumb);
    std::fs::remove_file(full);
}

pub fn hash(path: &Path) -> String {
    let mut hasher = sha1::Sha1::new();
    let mut file = std::fs::File::open(path).unwrap();
    std::io::copy(&mut file, &mut hasher).unwrap();
    format!("{:x}", hasher.finalize())
}


