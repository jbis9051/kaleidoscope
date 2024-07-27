mod format;

use std::hash::Hasher;
use std::path::Path;
use image::{RgbImage};
use serde::Deserialize;
use sha1::Digest;
use sqlx::{Connection, Executor, SqliteConnection};
use sqlx::types::chrono::{Utc};
use sqlx::types::Uuid;
use walkdir::{DirEntry, WalkDir};
use common::models::media::Media;
use common::models::system_time_to_naive_datetime;
use crate::format::{Format, heif, MediaMetadata, standard, video, raw};

#[derive(Deserialize)]
struct ScanConfig {
    paths: Vec<String>,
    data_dir: String,
    thumb_size: u32,
    db: String,
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("usage: {} <config file>", args[0]);
        std::process::exit(1);
    }
    let config_file = &args[1];
    let file = std::fs::read_to_string(config_file).unwrap();
    let config: ScanConfig = toml::from_str(&file).unwrap();
    let mut db = SqliteConnection::connect(&format!("sqlite:{}", config.db)).await.unwrap();

    for path in config.paths.iter() {
        println!("scanning path: {:?}", path);
        scan_dir(path, &config, &mut db).await;
    }
    
    println!("--- scanning complete, verifying database ---");

    let mut media = Media::all(&mut db).await.unwrap();

    let canoc_paths: Vec<String> = config.paths.iter().map(|p| Path::new(p).canonicalize().unwrap().to_string_lossy().to_string()).collect();

    for m in media.iter_mut() {
        // ensure this is within scope

        let media_path = m.path.clone();
        let path = Path::new(&media_path);

        if !canoc_paths.iter().any(|p| media_path.starts_with(p)) {
            println!("media path not in scan paths: {:?}", m.path);
            remove_media(m, &mut db, &config).await;
        }

        if !path.exists() {
            println!("missing media: {:?}", m.path);
            remove_media(m, &mut db, &config).await;
        }

        let path = Path::new(&config.data_dir).join(format!("{:?}-thumb.jpg", m.uuid));
        if !path.exists() {
            println!("missing thumbnail: {:?}", path);
        }

        let path = Path::new(&config.data_dir).join(format!("{:?}-full.jpg", m.uuid));
        if !path.exists() {
            println!("missing full: {:?}", path);
        }
    }

    println!("--- verification complete, cleaning up data ---");

    let files = std::fs::read_dir(&config.data_dir).unwrap();
    for file in files {
        let file = file.unwrap();
        let path = file.path();
        if path.extension().unwrap_or_default() == "jpg" {
            let name = path.file_stem().unwrap().to_string_lossy();
            let uuid = &name[0..36];
            if !media.iter().any(|m| m.uuid.to_string() == uuid) {
                println!("removing orphaned file: {:?}", path);
                std::fs::remove_file(path).unwrap();
            }
        }
    }
    
    println!("--- cleanup complete ---");


}

async fn remove_media(media: &mut Media, db: &mut SqliteConnection, config: &ScanConfig) {
    media.delete(&mut *db).await.unwrap();
    let thumb = Path::new(&config.data_dir).join(format!("{:?}-thumb.jpg", media.uuid));
    let full = Path::new(&config.data_dir).join(format!("{:?}-full.jpg", media.uuid));
    std::fs::remove_file(thumb);
    std::fs::remove_file(full);
}


async fn scan_dir(path: &str, config: &ScanConfig, db: &mut SqliteConnection) {
    for entry in WalkDir::new(path) {
        if let Ok(entry) = entry {
            if entry.file_type().is_dir() {
                println!("  discovered directory: {:?}", entry.path());
                continue;
            }
            println!("      found file: {:?}", entry.path());
            add_file(&entry, config, db).await;
        } else {
            println!("      unable to access: {:?}", entry.err().unwrap());
        }
    }
}

async fn add_file(entry: &DirEntry, config: &ScanConfig, db: &mut SqliteConnection) {
    // do a cheap check immediately to see if the media already exists
    let file_created_at = system_time_to_naive_datetime(entry.metadata().unwrap().created().unwrap());

    if let Some(mut media) = Media::from_path(&mut *db, entry.path().canonicalize().unwrap().to_string_lossy().as_ref()).await.unwrap() {
        let file_size = entry.metadata().unwrap().len() as u32;
        if media.file_created_at == file_created_at && media.size == file_size {
            println!("          media already exists: {:?}", entry.path());
            return;
        }
        remove_media(&mut media, db, config).await;
    }


    let uuid = Uuid::new_v4();
    let data_dir = Path::new(&config.data_dir);
    let thumb_path = data_dir.join(format!("{:?}-thumb.jpg", uuid));
    let full_path = data_dir.join(format!("{:?}-full.jpg", uuid));

    let (metadata, is_photo) = match get_media_metadata(entry) {
        Ok(Some(data)) => data,
        Ok(None) => {
            println!("          unsupported format: {:?}", entry.path());
            return;
        },
        Err(e) => {
            println!("          error getting metadata: {:?}", e);
            return;
        }
    };
    
    // write metadata to database

    if let Some(mut media) = Media::from_path(&mut *db, entry.path().canonicalize().unwrap().to_string_lossy().as_ref()).await.unwrap() {
        if media.created_at == metadata.created_at && media.size == metadata.size {
            // this shouldn't really happen, but it could if (1) there's a different date in the media metadata as opposed to the file metadata and (2) the file was modified while keeping the file metadata the same (including the size)
            println!("          media already exists (second check): {:?}", entry.path()); 
            return;
        }
        remove_media(&mut media, db, config).await;
    }

    let hash = hash(entry.path());

    // we want to generate a thumbnail while maintaining the aspect ratio, using thumb_size as the max size
    
    let mut twidth = config.thumb_size;
    let mut theight = config.thumb_size;
    
    if metadata.width > metadata.height {
        theight = (metadata.height as f32 / metadata.width as f32 * twidth as f32) as u32;
    } else {
        twidth = (metadata.width as f32 / metadata.height as f32 * theight as f32) as u32;
    }

    let (thumbnail, full) = match generate_media_caches(entry, twidth, theight) {
        Ok(t) => t.unwrap(),
        Err(e) => {
            println!("          error generating thumbnail: {:?}", e);
            return;
        }
    };


    // write thumbnail to disk
    println!("          writing thumbnail: {:?}", thumb_path);
    thumbnail.save(thumb_path).unwrap();

    // write full to disk
    println!("          writing full: {:?}", full_path);
    full.save(full_path).unwrap();

    let mut media = Media {
        id: 0,
        uuid,
        name: metadata.name,
        created_at: metadata.created_at,
        width: metadata.width,
        height: metadata.height,
        size: metadata.size,
        path: entry.path().canonicalize().unwrap().to_string_lossy().to_string(),
        liked: false,
        is_photo,
        added_at: Utc::now().naive_utc(),
        duration: metadata.duration.map(|d| d.as_secs() as u32),
        hash,
        file_created_at,
    };
    
    media.create(&mut *db).await.unwrap();
}


fn hash(path: &Path) -> String {
    let mut hasher = sha1::Sha1::new();
    let mut file = std::fs::File::open(path).unwrap();
    std::io::copy(&mut file, &mut hasher).unwrap();
    format!("{:x}", hasher.finalize())
}



macro_rules! generate_media_caches_formats {
    (
        $entry: expr,
        ($($format: ty),*),
        $twidth: expr,
        $theight: expr
    ) => {
        $(
            if <$format>::is_supported($entry) {
                let thumbnail = <$format>::generate_thumbnail($entry, $twidth, $theight)?;
                let full = <$format>::generate_full($entry)?;
                return Ok(Some((thumbnail, full)));
            }
        )*
    };
}

macro_rules! get_metadata_from_media_formats {
    (
        $entry: expr,
        ($($format: ty),*)
    ) => {
        $(
            if <$format>::is_supported($entry) {
                let metadata = <$format>::get_metadata($entry)?;
                let is_photo = <$format>::is_photo();
                return Ok(Some((metadata, is_photo)))
            }
        )*
    };
}
fn generate_media_caches(entry: &DirEntry, twidth: u32, theight: u32) -> Result<Option<(RgbImage, RgbImage)>, MetadataError> {
    generate_media_caches_formats!(entry, (standard::Standard, heif::Heif, video::Video, raw::Raw), twidth, theight);

    Ok(None)
}

fn get_media_metadata(entry: &DirEntry) -> Result<Option<(MediaMetadata, bool)>, MetadataError> {
    get_metadata_from_media_formats!(entry, (standard::Standard, heif::Heif, video::Video, raw::Raw));

    Ok(None)
}

#[derive(Debug, thiserror::Error)]
enum MetadataError {
    #[error("standard format error: {0}")]
    Standard(#[from] standard::StandardError),
    #[error("heif format error: {0}")]
    Heif(#[from] heif::HeifError),
    #[error("video format error: {0}")]
    Video(#[from] video::VideoError),
    #[error("raw format error: {0}")]
    Raw(#[from] raw::RawError),
}