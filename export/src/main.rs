use chrono::Local;
use clap::Parser;
use common::env::setup_log;
use common::media_query::MediaQuery;
use common::models::media::Media;
use common::scan_config::AppConfig;
use log::info;
use sqlx::migrate::Migrate;
use sqlx::{Connection, Row, SqliteConnection};
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use common::types::AcquireClone;
use tasks::tasks::thumbnail::ThumbnailGenerator;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct CliArgs {
    #[arg(index = 1)]
    config: String,
    #[arg(index = 2)]
    out_path: String,
}

#[tokio::main]
async fn main() {
    setup_log("export");

    let args = CliArgs::parse();
    let mut config: AppConfig = AppConfig::from_path(args.config);
    config.canonicalize();

    let stdin = std::io::stdin().lines();

    let mut queries: Vec<MediaQuery> = Vec::new();

    for line in stdin {
        let line = line.unwrap();
        queries.push(line.parse().unwrap());
    }

    let db_name = Path::new(&config.db_path)
        .file_stem()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    let now = Local::now().format("%Y%m%d_%H%M%S").to_string();

    let out_dir = Path::new(&args.out_path).join(format!("kaleidoscope_export_{}", now));

    fs::create_dir(&out_dir).unwrap();

    let data_dir = out_dir.join("data");

    fs::create_dir(&data_dir).unwrap();

    let media_dir = out_dir.join("media");

    fs::create_dir(&media_dir).unwrap();

    let export_db_path = Path::new(&out_dir).join(format!("{}_{}_export.db", db_name, now));

    // 1. copy the entire DB
    fs::copy(&config.db_path, &export_db_path).unwrap();

    let mut export_db = SqliteConnection::connect(&format!("sqlite:{}", export_db_path.display()))
        .await
        .unwrap();

    let mut included_media_set = HashSet::new();

    let mut included_media = Vec::new();

    // 2. get all matching media
    for query in &queries {
        let medias = Media::get_all(&mut export_db, &query)
            .await
            .expect("could not get medias");
        for media in &medias {
            included_media_set.insert(media.uuid);
        }
        included_media.extend(medias);
    }

    let all_media = Media::all(&mut export_db)
        .await
        .expect("could not get all medias");

    info!(
        "Including {} media of {}",
        included_media.len(),
        all_media.len()
    );

    // 3. delete non matching media from the db
    for media in all_media {
        if !included_media_set.contains(&media.uuid) {
            media
                .delete(&mut export_db)
                .await
                .expect("could not delete medias");
        }
    }

    // 4. create our csv

    let out_csv_path = Path::new(&out_dir).join("out.csv");
    let mut out_csv = csv::WriterBuilder::new()
        .has_headers(false)
        .from_path(out_csv_path)
        .unwrap();
    
    let headers = get_headers(&mut export_db).await;
    out_csv.write_record(&headers.0).unwrap();


    let mut new_config = config.clone();
    new_config.db_path = "".to_string();
    new_config.data_dir = data_dir.to_str().unwrap().to_string();

    for media in included_media {
        // 5. copy thumbnails
        if media.has_thumbnail {
            let thumb_path = ThumbnailGenerator::thumb_path(&media, &config);
            let full_path = ThumbnailGenerator::full_path(&media, &config);

            let to_thumb = ThumbnailGenerator::thumb_path(&media, &new_config);
            let to_full = ThumbnailGenerator::full_path(&media, &new_config);

            if thumb_path.exists() || full_path.exists() {
                fs::create_dir_all(to_full.parent().unwrap()).unwrap();
            }

            if thumb_path.exists() {
                fs::copy(&thumb_path, &to_thumb).unwrap();
            }

            if full_path.exists() {
                fs::copy(&full_path, &to_full).unwrap();
            }
        }
        // 6. copy actual media

        let new_media_path = media_dir.join(&media.name);
        fs::copy(&media.path, &new_media_path).unwrap();

        // 7. write metadata to csv
        
        let row = create_row(&headers, &media, &mut export_db).await;
        out_csv.write_record(row).unwrap();
    }
}

pub async fn get_headers(db: &mut SqliteConnection) -> (Vec<String>, usize) {
    let mut headers: Vec<String> = Vec::new();

    for h in [
        "id",
        "uuid",
        "name",
        "width",
        "height",
        "media_type",
        "duration",
        "hash",
        "is_screenshot",
        "longitude",
        "latitude",
        "has_thumbnail",
        "whisper_transcript",
        "vision_ocr_result",
        "tags",
        "albums",
    ] {
        headers.push(h.to_string());
    }
    
    let custom_starts = headers.len();

    let keys: Vec<String> = sqlx::query("SELECT DISTINCT key FROM custom_metadata;")
        .fetch_all(db)
        .await
        .unwrap()
        .into_iter()
        .map(|row| row.get(0))
        .collect();
    headers.extend(keys);
    (headers, custom_starts)
}

pub async fn create_row(headers: &(Vec<String>, usize), media: &Media, db: &mut SqliteConnection) -> Vec<String> {
    let (headers, custom_starts) = headers;
     
    let mut out: Vec<String> = Vec::new();
    
    
    out.push(media.id.to_string());
    out.push(media.uuid.to_string());
    out.push(media.name.to_string());
    out.push(media.width.to_string());
    out.push(media.height.to_string());
    out.push(media.media_type.to_string());
    out.push(media.duration.map(|d| d.to_string()).unwrap_or_default());
    out.push(media.hash.to_string());
    out.push(media.is_screenshot.to_string());
    out.push(media.longitude.map(|l| l.to_string()).unwrap_or_default());
    out.push(media.latitude.map(|l| l.to_string()).unwrap_or_default());
    out.push(media.has_thumbnail.to_string());

    let media_extra = media.extra(db.acquire_clone()).await.unwrap();

    if let Some(media_extra) = media_extra {
        out.push(media_extra.whisper_transcript.map(|w| w.to_string()).unwrap_or_default());
        out.push(media_extra.vision_ocr_result.map(|w| w.to_string()).unwrap_or_default());
    } else {
        out.push("".to_string());
        out.push("".to_string());
    }
    
    let tags = media.tags(db.acquire_clone()).await.unwrap();
    let tags: Vec<String> = tags.into_iter().map(|t| t.tag).collect();
    out.push(tags.join(","));
    
    let albums: Vec<String> = sqlx::query("SELECT album.name FROM album INNER JOIN album_media on album.id = album_media.album_id WHERE media_id = ?")
        .bind(media.id)
        .fetch_all(&mut *db)
        .await
        .unwrap()
        .into_iter()
        .map(|r| r.get(0))
        .collect();
    
    out.push(albums.join(","));
    
    // now we do custom
    
    let customs = &headers[(*custom_starts)..];
    
    for c in customs {
        let custom = media.latest_custom(db.acquire_clone(), c).await.unwrap();
        out.push(custom.map(|c| c.value).unwrap_or_default())
    }
    
    out
}
