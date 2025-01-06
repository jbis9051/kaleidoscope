mod format;

use crate::format::{heif, raw, standard, video, Format, MediaMetadata};
use common::models::media::Media;
use common::models::system_time_to_naive_datetime;
use common::scan_config::AppConfig;
use env_logger::Env;
use image::RgbImage;
use log::{debug, error, info, log, warn};
use serde::Deserialize;
use sha1::Digest;
use sqlx::types::chrono::Utc;
use sqlx::types::Uuid;
use sqlx::{Connection, Error, Executor, SqliteConnection};
use std::env;
use std::hash::Hasher;
use std::path::Path;
use walkdir::{DirEntry, WalkDir};
use common::directory_tree::{DirectoryTree, DIRECTORY_TREE_DB_KEY};
use common::models::kv::Kv;

#[tokio::main]
async fn main() {
    let rust_log = env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());

    let filter = match rust_log.as_str() {
        "trace" => log::LevelFilter::Trace,
        "debug" => log::LevelFilter::Debug,
        "info" => log::LevelFilter::Info,
        "warn" => log::LevelFilter::Warn,
        "error" => log::LevelFilter::Error,
        _ => log::LevelFilter::Info,
    };

    env_logger::Builder::new()
        .filter_module("scan", filter)
        .init();

    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("usage: {} <config file>", args[0]);
        std::process::exit(1);
    }
    let config_file = &args[1];
    let mut config: AppConfig = AppConfig::from_path(config_file);
    let mut db = SqliteConnection::connect(&format!("sqlite:{}", config.db_path))
        .await
        .unwrap();

    config.canonicalize();

    let mut total = 0;
    for path in config.scan_paths.iter() {
        info!("scanning path: {:?}", path);
        let count = scan_dir(path, &config, &mut db).await;
        info!("  found {} new media", count);
        total += count;
    }

    info!("--- scanning complete, found {} new media ---", total);
    info!("--- verifying database ---");

    let mut media = Media::all(&mut db).await.unwrap();

    for m in media.iter_mut() {
        // ensure this is within scope

        let media_path = m.path.clone();
        let path = Path::new(&media_path);

        if !config.path_matches(&path) {
            warn!("media path not in scan paths: {:?}", m.path);
            remove_media(m, &mut db, &config).await;
        }

        if !path.exists() {
            warn!("missing media: {:?}", m.path);
            remove_media(m, &mut db, &config).await;
        }

        let path = Path::new(&config.data_dir).join(format!("{:?}-thumb.jpg", m.uuid));
        if !path.exists() {
            warn!("missing thumbnail: {:?}", path);
        }

        let path = Path::new(&config.data_dir).join(format!("{:?}-full.jpg", m.uuid));
        if !path.exists() {
            warn!("missing full: {:?}", path);
        }
    }

    info!("--- verification complete, cleaning up data ---");

    let files = std::fs::read_dir(&config.data_dir).unwrap();
    for file in files {
        let file = file.unwrap();
        let path = file.path();
        if path.extension().unwrap_or_default() == "jpg" {
            let name = path.file_stem().unwrap().to_string_lossy();
            let uuid = &name[0..36];
            if !media.iter().any(|m| m.uuid.to_string() == uuid) {
                warn!("removing orphaned file: {:?}", path);
                std::fs::remove_file(path).unwrap();
            }
        }
    }

    info!("--- cleanup complete ---");

    info!("--- building directory tree ---");

    let mut tree = DirectoryTree::new();

    // iterate through all media and add them to the tree

    for m in media.iter() {
        // we want to add the path to the tree
        // but we want to remove the filename
        // so we can get the parent directory
        let path = Path::new(&m.path);
        let parent = path.parent().unwrap_or_else(|| Path::new("/"));
        let parent = parent.to_string_lossy();
        tree.add_path(&parent);
    }

    debug!("{:?}", tree);

    let mut kv =
        Kv::from_key(&mut db, DIRECTORY_TREE_DB_KEY).await.expect("error getting directory tree").unwrap_or_else(|| {
            Kv {
                id: 0,
                key: DIRECTORY_TREE_DB_KEY.to_string(),
                value: "{}".to_string(),
                created_at: Default::default(),
                updated_at: Default::default(),
            }
        });

    kv.value = serde_json::to_string(&tree).unwrap();

    // TODO: This is not atomic but it's sqlite and a scan so who cares
    if let Some(mut kv) = Kv::from_key(&mut db, &kv.key).await.unwrap() {
        kv.update_by_key(&mut db).await.unwrap();
    } else {
        kv.create(&mut db).await.unwrap();
    }

    info!("--- directory tree built ---");

    info!("--- scan complete ---");
}

async fn remove_media(media: &mut Media, db: &mut SqliteConnection, config: &AppConfig) {
    media.delete(&mut *db).await.unwrap();
    let thumb = Path::new(&config.data_dir).join(format!("{:?}-thumb.jpg", media.uuid));
    let full = Path::new(&config.data_dir).join(format!("{:?}-full.jpg", media.uuid));
    std::fs::remove_file(thumb);
    std::fs::remove_file(full);
}


async fn scan_dir(path: &str, config: &AppConfig, db: &mut SqliteConnection) -> u32 {
    let mut count = 0;
    for entry in WalkDir::new(path) {
        if let Ok(entry) = entry {
            if !config.path_matches(&entry.path()) {
                debug!("      skipping path (based on config): {:?}", entry.path());
                continue;
            }

            if entry.file_type().is_dir() {
                debug!("  discovered directory: {:?}", entry.path());
                continue;
            }
            if entry.file_type().is_symlink() {
                debug!("      skipping symlink: {:?}", entry.path());
                continue;
            }
            if add_file(&entry, config, db).await {
                info!("      found new file: {:?}", entry.path());
                count += 1;
            }
        } else {
            error!("      unable to access: {:?}", entry.err().unwrap());
        }
    }
    count
}

async fn add_file(entry: &DirEntry, config: &AppConfig, db: &mut SqliteConnection) -> bool {
    // do a cheap check immediately to see if the media already exists
    let file_created_at = system_time_to_naive_datetime(entry.metadata().unwrap().created().unwrap());

    if let Some(mut media) = Media::from_path(&mut *db, entry.path().canonicalize().unwrap().to_string_lossy().as_ref()).await.unwrap() {
        let file_size = entry.metadata().unwrap().len() as u32;
        if media.file_created_at == file_created_at && media.size == file_size {
            debug!("          media already exists: {:?}", entry.path());
            return false;
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
            debug!("          unsupported format: {:?}", entry.path());
            return false;
        }
        Err(e) => {
            error!("          error getting metadata: {:?}", e);
            return false;
        }
    };

    // write metadata to database

    if let Some(mut media) = Media::from_path(&mut *db, entry.path().canonicalize().unwrap().to_string_lossy().as_ref()).await.unwrap() {
        if media.created_at == metadata.created_at && media.size == metadata.size {
            // this shouldn't really happen, but it could if (1) there's a different date in the media metadata as opposed to the file metadata and (2) the file was modified while keeping the file metadata the same (including the size)
            debug!("          media already exists (second check): {:?}", entry.path());
            return false;
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
            error!("          error generating thumbnail: {:?}", e);
            return false;
        }
    };


    // write thumbnail to disk
    debug!("          writing thumbnail: {:?}", thumb_path);
    thumbnail.save(thumb_path).unwrap();

    // write full to disk
    debug!("          writing full: {:?}", full_path);
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
    true
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