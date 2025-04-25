use std::process::exit;
use common::media_query::MediaQuery;
use common::models::media::Media;
use common::scan_config::AppConfig;
use common::types::AcquireClone;
use custom_task::run_custom;
use sqlx::{Connection, SqliteConnection};

#[tokio::main]
async fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() < 2 {
        eprintln!("Usage: ./custom_task <config>");
        exit(1);
    }
    let config_path = &args[1];
    let app_config = AppConfig::from_path(&config_path);
    let mut db = SqliteConnection::connect(&format!("sqlite:{}", app_config.db_path))
        .await
        .unwrap();
    for (custom_task, config) in &app_config.custom {
        println!("[[Task {}]]", custom_task);
        println!("{:#?}", config);
        let query: MediaQuery = config.query.parse().unwrap();
        let matching = Media::get_all(db.acquire_clone(), &query).await.unwrap();
        println!("matched {} media", matching.len());

        for (i, media) in matching.iter().enumerate() {
            println!(
                "[({}/{}): {}-{}]",
                i,
                matching.len(),
                &media.uuid,
                &media.path
            );
            run_custom(&mut db, &media, &app_config, config, true)
                .await
                .unwrap();
        }
    }
}
