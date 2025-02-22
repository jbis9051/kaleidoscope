mod ipc;
mod migrations;

use std::io::{BufRead, Read, Write};
use axum::{Extension, Json, Router, routing::get};
use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::{header, HeaderMap, HeaderName, HeaderValue, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use chrono::Utc;
use nix::unistd::Uid;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::sqlite::SqlitePool;
use tower_http::cors::{Any, CorsLayer};
use uuid::Uuid;
use common::models::album::Album;
use common::models::media::Media;
use common::types::DbPool;
use tokio_util::io::ReaderStream;
use common::directory_tree::{DirectoryTree, DIRECTORY_TREE_DB_KEY, LAST_IMPORT_ID_DB_KEY};
use common::env::EnvVar;
use common::media_query::{MediaQuery, MediaQueryType};
use common::models::kv::Kv;
use common::models::media_view::MediaView;
use common::models::timeline::Timeline;
use common::scan_config::AppConfig;
use crate::ipc::request_ipc_file;

static ENV: Lazy<EnvVar> = Lazy::new(|| {
    let env = EnvVar::from_env();
    env
});

static CONFIG: Lazy<AppConfig> = Lazy::new(|| {
    ENV.config.as_ref().expect("No config provided").clone()
});

#[tokio::main]
async fn main() {
    // ensure we aren't running as root
    if Uid::current().is_root() {
        eprintln!("Server must not be run as root!");
        std::process::exit(1);
    }
    
    if ENV.dev_mode {
        println!("Running in dev mode");
    }

    println!("Config: {:?}", &CONFIG);

    let pool = SqlitePool::connect(&format!("sqlite://{}", CONFIG.db_path)).await.unwrap();


    if ENV.db_migrate {
        println!("Migrating database");
        sqlx::migrate!("../db/migrations").run(&pool).await.expect("Failed to migrate database");
        println!("Migration complete");
    }


    println!("Listening on: {}", &CONFIG.listen_addr);


    let cors = CorsLayer::new()
        .allow_methods(vec![Method::GET, Method::POST, Method::DELETE, Method::OPTIONS])
        .allow_origin(Any)
        .allow_headers(vec![header::CONTENT_TYPE]);

    let app = Router::new()
        .route("/media", get(media_index))
        // .route("/media/map", get(media_map))
        .route("/media/timeline", get(media_timeline))
        .route("/media/:uuid", get(media))
        .route("/media/:uuid/raw", get(media_raw))
        .route("/media/:uuid/full", get(media_full))
        .route("/media/:uuid/thumb", get(media_thumb))
        .route("/album", get(album_index).post(album_create))
        .route("/album/:uuid", get(album).delete(album_delete))
        .route("/album/:uuid/media", post(album_add_media).delete(album_delete_media))
        .route("/album/:uuid/timeline", get(album_timeline))
        .route("/media_view", get(media_view_index).post(media_view_create).delete(media_view_delete))
        .route("/directory_tree", get(directory_tree))
        .route("/info", get(info))
        .layer(Extension(pool))
        .layer(cors);

    let listener = tokio::net::TcpListener::bind(&CONFIG.listen_addr).await.unwrap();

    axum::serve(listener, app).await.unwrap();
}

#[derive(Serialize, Deserialize)]
pub struct MediaQueryQuery {
    query: MediaQuery
}

#[derive(Debug, Serialize)]
struct MediaIndexResponse {
    media: Vec<Media>,
    count: u32,
}
async fn media_index(Extension(conn): Extension<DbPool>, query: Query<MediaQueryQuery>) -> Result<Json<MediaIndexResponse>, (StatusCode, String)> {
    let query = &query.query;

    if let Err(err) = query.validate() {
        return Err((StatusCode::BAD_REQUEST, format!("invalid query: {}", err)));
    }

    let media = Media::get_all(&conn, &query).await.unwrap();
    let count = Media::count(&conn, &query.to_count_query()).await.unwrap();
    Ok(Json(MediaIndexResponse { media, count }))
}

#[derive(Debug, serde::Deserialize)]
struct MediaParams {
    uuid: Uuid
}

async fn media(Extension(conn): Extension<DbPool>, path: Path<MediaParams>) -> Result<Json<Media>, (StatusCode, String)> {
    let media = Media::from_uuid(&conn, &path.uuid).await.map_err(|_| (StatusCode::NOT_FOUND, "Media not found".to_string()))?;
    Ok(Json(media))
}

async fn media_raw(Extension(conn): Extension<DbPool>, path: Path<MediaParams>) -> Result<(HeaderMap, Body), (StatusCode, String)> {
    let media = Media::from_uuid(&conn, &path.uuid).await.map_err(|_| (StatusCode::NOT_FOUND, "Media not found".to_string()))?;
    
    let stream = request_ipc_file(&CONFIG, &media).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("ipc error requesting file: {:?}", e)))?;
    let body = Body::from_stream(ReaderStream::new(stream));


    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_str("application/octet-stream").unwrap());
    headers.insert(header::CONTENT_DISPOSITION, HeaderValue::from_str(&format!("attachment; filename=\"{}\"", media.name)).unwrap());
    headers.insert(header::CONTENT_LENGTH, HeaderValue::from_str(&media.size.to_string()).unwrap());

    Ok((headers, body))
}

async fn media_full(Extension(conn): Extension<DbPool>, path: Path<MediaParams>) -> Result<(HeaderMap, Body), (StatusCode, String)> {
    let media = Media::from_uuid(&conn, &path.uuid).await.map_err(|_| (StatusCode::NOT_FOUND, "Media not found".to_string()))?;
    let path = std::path::Path::new(&CONFIG.data_dir).join(format!("{}-full.jpg", media.uuid));
    Ok(serve_file(&path, "image/jpeg".to_string()).await)
}

async fn media_thumb(Extension(conn): Extension<DbPool>, path: Path<MediaParams>) -> Result<(HeaderMap, Body), (StatusCode, String)> {
    let media = Media::from_uuid(&conn, &path.uuid).await.map_err(|_| (StatusCode::NOT_FOUND, "Media not found".to_string()))?;
    let path = std::path::Path::new(&CONFIG.data_dir).join(format!("{}-thumb.jpg", media.uuid));
    Ok(serve_file(&path, "image/jpeg".to_string()).await)
}


async fn serve_file(path: &std::path::Path, content_type: String) -> (HeaderMap, Body) {
    let file = tokio::fs::File::open(path).await.unwrap();
    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_str(&content_type).unwrap());

    (headers, body)
}
async fn album_index(Extension(conn): Extension<DbPool>) -> Json<Vec<(Album, u32)>> {
    let albums = Album::get_all(&conn).await.unwrap();
    let mut out = Vec::with_capacity(albums.len());
    for album in albums.into_iter() {
        let count = album.count_media(&conn, &MediaQuery::new()).await.unwrap();
        out.push((album, count));
    }
    Json(out)
}

#[derive(Debug, serde::Deserialize)]
struct AlbumParams {
    uuid: Uuid
}

#[derive(Debug, Serialize)]
struct AlbumResponse {
    album: Album,
    media: MediaIndexResponse
}

async fn album(Extension(conn): Extension<DbPool>, path: Path<AlbumParams>, query: Query<MediaQueryQuery>) -> Result<Json<AlbumResponse>, (StatusCode, String)> {
    let query = &query.query;

    if let Err(err) = query.validate() {
        return Err((StatusCode::BAD_REQUEST, format!("invalid query: {}", err)));
    }

    let album = Album::from_uuid(&conn, &path.uuid).await.map_err(|_| (StatusCode::NOT_FOUND, "Album not found".to_string()))?;
    let media = album.get_media(&conn, &query).await.unwrap();
    let count = album.count_media(&conn, &query.to_count_query()).await.unwrap();
    Ok(Json(AlbumResponse { album, media: MediaIndexResponse { media, count } }))
}

#[derive(Debug, serde::Deserialize)]
struct AlbumCreateParams {
    name: String,
}

async fn album_create(Extension(conn): Extension<DbPool>, payload: Json<AlbumCreateParams>) -> Json<Album> {
    let mut album = Album {
        uuid: Uuid::new_v4(),
        name: payload.name.clone(),
        created_at: Utc::now().naive_utc(),
        id: 0,
    };
    album.create(&conn).await.unwrap();
    Json(album)
}

async fn album_delete(Extension(conn): Extension<DbPool>, path: Path<AlbumParams>) -> Result<(), (StatusCode, String)> {
    let album = Album::from_uuid(&conn, &path.uuid).await.map_err(|_| (StatusCode::NOT_FOUND, "Album not found".to_string()))?;
    album.delete(&conn).await.unwrap();
    Ok(())
}



#[derive(Debug, serde::Deserialize)]
struct AlbumMediaParam {
    medias: Vec<Uuid>,
}

async fn album_add_media(Extension(conn): Extension<DbPool>, path: Path<AlbumParams>, payload: Json<AlbumMediaParam>) -> Result<Json<Album>, (StatusCode, String)> {
    let album = Album::from_uuid(&conn, &path.uuid).await.map_err(|_| (StatusCode::NOT_FOUND, "Album not found".to_string()))?;

    let mut medias = Vec::with_capacity(payload.medias.len());

    for media_uuid in payload.medias.iter() {
        medias.push(Media::from_uuid(&conn, media_uuid).await.map_err(|_| (StatusCode::NOT_FOUND, format!("Media not found: {}", media_uuid)))?);
    }

    let mut transaction = conn.begin().await.unwrap();

    for media in medias.iter() {
        if !album.has_media(&mut transaction, media.id).await.unwrap() {
            album.add_media(&mut transaction, media.id).await.unwrap();
        }
    }

    transaction.commit().await.unwrap();

    Ok(Json(album))
}

async fn album_delete_media(Extension(conn): Extension<DbPool>, path: Path<AlbumParams>, payload: Json<AlbumMediaParam>) -> Result<Json<Album>, (StatusCode, String)> {
    let album = Album::from_uuid(&conn, &path.uuid).await.map_err(|_| (StatusCode::NOT_FOUND, "Album not found".to_string()))?;

    let mut medias = Vec::with_capacity(payload.medias.len());

    for media_uuid in payload.medias.iter() {
        medias.push(Media::from_uuid(&conn, media_uuid).await.map_err(|_| (StatusCode::NOT_FOUND, format!("Media not found: {}", media_uuid)))?);
    }

    let mut transaction = conn.begin().await.unwrap();

    for media in medias.iter() {
        album.remove_media(&mut transaction, media.id).await.unwrap();
    }

    transaction.commit().await.unwrap();

    Ok(Json(album))
}



#[derive(Debug, Serialize)]
struct MediaViewIndexResponse {
    media_views: Vec<MediaView>,
    last_import_id: i32,
}

async fn media_view_index(Extension(conn): Extension<DbPool>) -> Json<MediaViewIndexResponse> {
    let media = MediaView::get_all(&conn).await.unwrap();
    let last_import_id = Kv::from_key(&conn, LAST_IMPORT_ID_DB_KEY).await.unwrap().map(|kv| kv.value.parse().unwrap()).unwrap_or(-1);

    Json(MediaViewIndexResponse { media_views: media, last_import_id })
}

#[derive(Debug, serde::Deserialize)]
struct MediaViewCreateParams {
    name: String,
    view_query: String,
}

async fn media_view_create(Extension(conn): Extension<DbPool>, payload: Json<MediaViewCreateParams>) -> Json<MediaView> {
    let mut media_view = MediaView {
        uuid: Uuid::new_v4(),
        name: payload.name.clone(),
        view_query: payload.view_query.clone(),
        created_at: Utc::now().naive_utc(),
        id: 0,
    };
    media_view.create(&conn).await.unwrap();
    Json(media_view)
}

#[derive(Debug, serde::Deserialize)]
struct MediaViewParams {
    uuid: Uuid
}

async fn media_view_delete(Extension(conn): Extension<DbPool>, payload: Json<MediaViewParams>) {
    let media_view = MediaView::from_uuid(&conn, &payload.uuid).await.unwrap();
    media_view.delete(&conn).await.unwrap();
}


async fn directory_tree(Extension(conn): Extension<DbPool>) -> Result<Json<DirectoryTree>, (StatusCode, String)> {
    let kv = Kv::from_key(&conn, DIRECTORY_TREE_DB_KEY).await.unwrap().ok_or_else(|| (StatusCode::NOT_FOUND, "Directory tree not found".to_string()))?;
    let tree: DirectoryTree = serde_json::from_str(kv.value.as_str()).unwrap();
    Ok(Json(tree))
}

async fn info() -> Response {
    let info = format!(
        r#"{{
            "media_query": {}
        }}"#,
        MediaQueryType::describe()
    );

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        info,
    )
        .into_response()
}

#[derive(Serialize, Deserialize)]
struct MapQuery {
    query: MediaQuery,
    zoom: u32,
}

async fn media_map(Extension(conn): Extension<DbPool>, query: Query<MapQuery>) -> Result<Json<Vec<()>>, (StatusCode, String)> {
    Ok(Json(vec![]))
}

#[derive(Serialize, Deserialize)]
struct TimelineQuery {
    query: MediaQuery,
    interval: String,
}

async fn media_timeline(Extension(conn): Extension<DbPool>, query: Query<TimelineQuery>) -> Result<Json<Vec<Value>>, (StatusCode, String)> {
    let media_query = &query.query;
    let interval = &query.interval;
    
    if let Err(err) = media_query.validate() {
        return Err((StatusCode::BAD_REQUEST, format!("invalid query: {}", err)));
    }
    
    let media_query = media_query.to_count_query();

    timeline(&conn, &media_query, interval, None).await
}

async fn album_timeline(Extension(conn): Extension<DbPool>, query: Query<TimelineQuery>, path: Path<AlbumParams>) -> Result<Json<Vec<Value>>, (StatusCode, String)> {
    let media_query = &query.query;
    let interval = &query.interval;

    if let Err(err) = media_query.validate() {
        return Err((StatusCode::BAD_REQUEST, format!("invalid query: {}", err)));
    }

    let media_query = media_query.to_count_query();

    let album = Album::from_uuid(&conn, &path.uuid).await.map_err(|_| (StatusCode::NOT_FOUND, "Album not found".to_string()))?;

    timeline(&conn, &media_query, interval, Some(album.id)).await
}

async fn timeline(conn: &DbPool, media_query: &MediaQuery, interval: &str, album_id: Option<i32>) -> Result<Json<Vec<Value>>, (StatusCode, String)> {
    let media_query = media_query.to_count_query();

    match interval {
        "month" => {
            let timeline = Timeline::timeline_months(&conn, &media_query, album_id).await.unwrap();
            Ok(Json(timeline.into_iter().map(|t| serde_json::to_value(t).unwrap()).collect()))
        }
        "day" => {
            let timeline = Timeline::timeline_days(&conn, &media_query, album_id).await.unwrap();
            Ok(Json(timeline.into_iter().map(|t| serde_json::to_value(t).unwrap()).collect()))
        }
        "hour" => {
            let timeline = Timeline::timeline_hours(&conn, &media_query, album_id).await.unwrap();
            Ok(Json(timeline.into_iter().map(|t| serde_json::to_value(t).unwrap()).collect()))
        }
        _ => {
            Err((StatusCode::BAD_REQUEST, "invalid interval, options: 'month|day|hour'".to_string()))
        }
    }
}