mod media_operations;

use std::collections::HashSet;
use crate::media_operations::{add_media, remove_media, update_media, AddMediaError};
use common::directory_tree::{DirectoryTree, DIRECTORY_TREE_DB_KEY, LAST_IMPORT_ID_DB_KEY};
use common::models::kv::Kv;
use common::models::media::Media;
use common::scan_config::AppConfig;
use log::{debug, error, info, log, warn};
use sha1::Digest;
use sqlx::{Connection, SqliteConnection};
use std::env;
use std::path::Path;
use walkdir::WalkDir;
use common::{debug_sql, question_marks, update_set};
use common::media_processors::format::{match_format, FormatType};
use tasks::ops::{add_outdated_queues, add_to_compatible_queues};
use tasks::tasks::Task;

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

    info!("--- starting scan ---");

    if Kv::from_key(&mut db, LAST_IMPORT_ID_DB_KEY).await.unwrap().is_none() {
        Kv {
            id: 0,
            key: LAST_IMPORT_ID_DB_KEY.to_string(),
            value: "0".to_string(),
            created_at: Default::default(),
            updated_at: Default::default(),
        }.create(&mut db).await.unwrap();
    }

    let mut import_id_kv = Kv::from_key(&mut db, LAST_IMPORT_ID_DB_KEY)
        .await
        .expect("error getting last import id")
        .expect("last import id not found");

    let mut import_id = import_id_kv.value.parse::<i32>().unwrap();

    import_id += 1;

    import_id_kv.value = import_id.to_string();

    import_id_kv.update_by_key(&mut db).await.unwrap();

    debug!("beginning import id: {}", import_id);

    let mut total = 0;
    for path in config.scan_paths.iter() {
        info!("scanning path: {:?}", path);
        let count = scan_dir(path, &config, import_id, &mut db).await;
        info!("  found {} new media", count);
        total += count;
    }

    info!("--- scanning complete, found {} new media, import_id: {} ---", total, import_id);

    info!("--- updating database ---");
    info!("--- updating database: metadata ---");
    
    let formats =  FormatType::all();
    let mut updated = vec![0; formats.len()];
    for (i, format) in formats.iter().enumerate() {
       let metadata_version = match format {
            _ => match_format!(format, |ActualFormat| { <ActualFormat as Format>::METADATA_VERSION })
       };
        
        // TODO: i don't think format changes are actually being detected
        
        let mut outdated = Media::outdated(&mut db, *format, metadata_version).await.unwrap();
        
        for media in outdated.iter_mut() {
            match update_media(media, &config, &mut db).await {
                Ok(_) => {
                    updated[i] += 1;
                }
                Err(e) => {
                    error!("  error updating media: {:?} - {:?}", media, e);
                }
            }
        }
        
    }
    
    let report = formats.iter().zip(updated.iter()).map(|(f, u)| format!("{:?}[{}]", f, u)).collect::<Vec<String>>().join("|");
    
    info!("--- updating metadata complete report: {} ---", report);

    info!("--- updating database: tasks ---");

    let media = Media::all(&mut db).await.unwrap();

    let report = add_outdated_queues(&mut db, &media, &Task::TASK_NAMES, &config.tasks, &config).await.unwrap();
    
    let report = report.iter().map(|(t, c)| format!("{}[{}]", t, c)).collect::<Vec<String>>().join("|");
    
    info!("--- updating database: tasks complete queuing report: {} ---", report);

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
    }

    info!("--- verification complete, cleaning up data ---");

    let files = std::fs::read_dir(&config.data_dir).unwrap();
    let uuids: HashSet<String> = media.iter().map(|m| m.uuid.to_string()).collect();
    
    for file in files {
        let file = file.unwrap();
        let path = file.path();
        if path.extension().unwrap_or_default() == "jpg" {
            let name = path.file_stem().unwrap().to_string_lossy();
            let uuid = &name[0..36];
            if !uuids.contains(uuid) {
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
    if Kv::from_key(&mut db, &kv.key).await.unwrap().is_some() {
        kv.update_by_key(&mut db).await.unwrap();
    } else {
        kv.create(&mut db).await.unwrap();
    }

    info!("--- directory tree built ---");

    info!("--- scan complete ---");
}


async fn scan_dir(path: &str, config: &AppConfig, import_id: i32, db: &mut SqliteConnection) -> u32 {
    let mut count = 0;
    for entry in WalkDir::new(path) {
        if let Ok(entry) = entry {
            if !config.path_matches(entry.path()) {
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
            match add_media(entry.path(), config, import_id, db).await {
                Ok(_) => {
                    info!("      found new file: {:?}", entry.path());
                    count += 1;
                }
                Err(AddMediaError::AlreadyExists(_)) => {
                    debug!("      file already exists: {:?}", entry.path());
                }
                Err(e) => {
                    error!("      error adding file: {} - {:?}", e, entry.path());
                }
            }
        } else {
            error!("      unable to access: {:?}", entry.err().unwrap());
        }
    }
    count
}