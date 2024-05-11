mod config;


use axum::{Extension, Json, Router, routing::get, serve};
use axum::body::Body;
use axum::extract::{Path, Query};
use axum::http::{header, HeaderMap, HeaderValue, Method, StatusCode};
use axum::response::IntoResponse;
use axum::routing::post;
use serde::Serialize;
use sqlx::sqlite::SqlitePool;
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};
use uuid::Uuid;
use common::models::album::Album;
use common::models::media::Media;
use crate::config::CONFIG;
use common::types::DbPool;
use tokio_util::io::ReaderStream;

#[tokio::main]
async fn main() {
    println!("Listening on: {}", &CONFIG.listen_addr);
    println!("Config: {:?}", &CONFIG);
    let pool = SqlitePool::connect(&format!("sqlite://{}", CONFIG.db_path)).await.unwrap();

    let cors = CorsLayer::new()
        .allow_origin(Any);

    let app = Router::new()
        .route("/media", get(media_index))
        .route("/media/:uuid", get(media))
        .route("/media/:uuid/raw", get(media_raw))
        .route("/media/:uuid/full", get(media_full))
        .route("/media/:uuid/thumb", get(media_thumb))
        .route("/albums", get(album_index))
        .route("/albums/:uuid", get(album))
        .route("/albums/:uuid/media", post(album_add).delete(album_delete))
        .layer(Extension(pool))
        .layer(cors);

    let listener = tokio::net::TcpListener::bind(&CONFIG.listen_addr).await.unwrap();

    axum::serve(listener, app).await.unwrap();
}

#[derive(Debug, serde::Deserialize)]
struct MediaIndexParams {
    page: i32,
    limit: i32,
    asc: bool,
    order_by: String,
}
async fn media_index(Extension(conn): Extension<DbPool>, query: Query<MediaIndexParams>) -> Result<Json<Vec<Media>>, (StatusCode, String)> {
    if Media::safe_column(&query.order_by).is_err() {
        return Err((StatusCode::BAD_REQUEST, format!("Invalid column: {}", &query.order_by)));
    }
    
    let media = Media::get_all(&conn, &query.order_by, query.asc, query.limit, query.page - 1).await.unwrap();
    Ok(Json(media))
}

#[derive(Debug, serde::Deserialize)]
struct MediaParams {
    uuid: Uuid
}

async fn media(Extension(conn): Extension<DbPool>, path: Path<MediaParams>) -> Json<Media> {
    let media = Media::from_uuid(&conn, &path.uuid).await.unwrap();
    Json(media)
}

async fn media_raw(Extension(conn): Extension<DbPool>, path: Path<MediaParams>) -> (HeaderMap, Body) {
    let media = Media::from_uuid(&conn, &path.uuid).await.unwrap();
    serve_file(std::path::Path::new(&media.path), "application/octet-stream".to_string()).await
}

async fn media_full(Extension(conn): Extension<DbPool>, path: Path<MediaParams>) -> (HeaderMap, Body) {
    let media = Media::from_uuid(&conn, &path.uuid).await.unwrap();
    let path = std::path::Path::new(&CONFIG.data_dir).join(format!("{}-full.jpg", media.uuid));
    serve_file(&path, "image/jpeg".to_string()).await
}

async fn media_thumb(Extension(conn): Extension<DbPool>, path: Path<MediaParams>) -> (HeaderMap, Body) {
    let media = Media::from_uuid(&conn, &path.uuid).await.unwrap();
    let path = std::path::Path::new(&CONFIG.data_dir).join(format!("{}-thumb.jpg", media.uuid));
    serve_file(&path, "image/jpeg".to_string()).await
}


async fn serve_file(path: &std::path::Path, content_type: String) -> (HeaderMap, Body) {
    let file = tokio::fs::File::open(path).await.unwrap();
    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_str(&content_type).unwrap());

    (headers, body)
}



#[derive(Debug, serde::Deserialize)]
struct AlbumIndexParams {
    page: i32,
    limit: i32,
    asc: bool,
    order_by: String,
}

async fn album_index(Extension(conn): Extension<DbPool>, query: Query<AlbumIndexParams>) -> Json<Vec<Album>> {
    let albums = Album::get_all(&conn, &query.order_by, query.asc, query.limit, query.page).await.unwrap();
    Json(albums)
}

#[derive(Debug, serde::Deserialize)]
struct AlbumParams {
    uuid: Uuid
}

async fn album(Extension(conn): Extension<DbPool>, path: Path<AlbumParams>) -> Json<Album> {
    let album = Album::from_uuid(&conn, &path.uuid).await.unwrap();
    Json(album)
}

#[derive(Debug, serde::Deserialize)]
struct AlbumAddParam {
    medias: Vec<Uuid>,
}

async fn album_add(Extension(conn): Extension<DbPool>, path: Path<AlbumParams>, query: Query<AlbumAddParam>) -> Json<Album> {
    let album = Album::from_uuid(&conn, &path.uuid).await.unwrap();

    let mut medias = Vec::with_capacity(query.medias.len());

    for media_uuid in query.medias.iter() {
        medias.push(Media::from_uuid(&conn, media_uuid).await.unwrap());
    }

    let mut transaction = conn.begin().await.unwrap();

    for media in medias.iter() {
        album.add_media(&mut transaction, media.id).await.unwrap();
    }

    transaction.commit().await.unwrap();

    Json(album)
}

async fn album_delete(Extension(conn): Extension<DbPool>, path: Path<AlbumParams>) -> Json<()> {
    let album = Album::from_uuid(&conn, &path.uuid).await.unwrap();
    album.delete(&conn).await.unwrap();
    Json(())
}





