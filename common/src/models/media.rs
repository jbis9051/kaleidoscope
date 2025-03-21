use crate::question_marks;
use sqlx::sqlite::SqliteRow;
use sqlx::types::chrono::NaiveDateTime;
use sqlx::{Execute, Row, SqliteExecutor};
use std::borrow::Borrow;
use serde::Serialize;
use uuid::{Uuid};
use crate::media_query::MediaQuery;
use crate::models::{date, MediaError};
use crate::{sqlize, update_set};
use crate::media_processors::format::{FormatType, MediaType};
use crate::types::{DbPool, SqliteAcquire};


#[derive(Debug, Serialize)]
pub struct Metadata {
    pub id: i32,
    pub media_id: i32,
    pub key: String,
    pub value: String,
}

#[derive(Debug, Serialize, PartialEq, Clone)]
pub struct Media {
    pub id: i32,
    pub uuid: Uuid,
    pub name: String,
    #[serde(with = "date")]
    pub created_at: NaiveDateTime,
    pub width: u32,
    pub height: u32,
    pub path: String,
    pub liked: bool,
    pub media_type: MediaType,
    #[serde(with = "date")]
    pub added_at: NaiveDateTime,
    pub duration: Option<u32>,
    pub hash: String,
    // in bytes
    pub size: u32,
    #[serde(with = "date")]
    pub file_created_at: NaiveDateTime,

    pub is_screenshot: bool,

    pub longitude: Option<f64>,
    pub latitude: Option<f64>,

    pub has_thumbnail: bool,

    pub format: FormatType,
    pub metadata_version: i32,
    pub thumbnail_version: i32,
    pub import_id: i32,
}

sqlize!(Media, "media", id, [
    uuid,
    name,
    created_at,
    width,
    height,
    size,
    path,
    liked,
    media_type,
    added_at,
    duration,
    hash,
    file_created_at,
    is_screenshot,
    format,
    longitude,
    latitude,
    metadata_version,
    thumbnail_version,
    import_id,
    has_thumbnail
]);

impl Media {
    pub fn safe_column(name: &str) -> Result<(), sqlx::Error> {
        match name {
            "id" | "uuid" | "name" | "created_at" | "width" | "height" | "size" | "path" | "liked" | "media_type" | "added_at" | "duration" | "import_id" => Ok(()),
            _ => Err(sqlx::Error::ColumnNotFound(name.to_string()))
        }
    }

    pub async fn get_all(db: &DbPool, media_query: &MediaQuery) -> Result<Vec<Self>, MediaError> {
        let mut query = sqlx::QueryBuilder::new("SELECT * FROM media WHERE 1=1");

        media_query.sqlize(&mut query)?;

        let query = query.build();

        Ok(query
            .fetch_all(db)
            .await?
            .iter()
            .map(|row| row.into())
            .collect())
    }

    pub async fn count(db: &DbPool, media_query: &MediaQuery) -> Result<u32, MediaError> {
        let mut query = sqlx::QueryBuilder::new("SELECT COUNT(*) FROM media WHERE 1=1");

        media_query.sqlize(&mut query)?;

        let query = query.build();

        Ok(query
            .fetch_one(db)
            .await?
            .get(0))
    }
    pub async fn all<'a>(db: impl SqliteExecutor<'a>) -> Result<Vec<Self>, sqlx::Error> {
        Ok(sqlx::query("SELECT * FROM media;")
            .fetch_all(db)
            .await?
            .iter()
            .map(|row| row.into())
            .collect())
    }

    pub async fn from_uuid(db: &DbPool, uuid: &Uuid) -> Result<Self, sqlx::Error> {
        Ok(sqlx::query("SELECT * FROM media WHERE uuid = $1;")
            .bind(uuid)
            .fetch_one(db)
            .await?
            .borrow()
            .into())
    }

    pub async fn from_id(db: impl SqliteAcquire<'_>, id: &i32) -> Result<Self, sqlx::Error> {
        let mut conn = db.acquire().await?;
        Ok(sqlx::query("SELECT * FROM media WHERE id = $1;")
            .bind(id)
            .fetch_one(&mut *conn)
            .await?
            .borrow()
            .into())
    }

    pub async fn from_path<'a>(db: impl SqliteExecutor<'a>, path: &str) -> Result<Option<Self>, sqlx::Error> {
        Ok(sqlx::query("SELECT * FROM media WHERE path = $1;")
            .bind(path)
            .fetch_optional(db)
            .await?
            .map(|row| row.borrow().into()))
    }


    pub async fn outdated<'a>(db: impl SqliteExecutor<'a>, format_type: FormatType, metadata_version: i32) -> Result<Vec<Self>, sqlx::Error> {
        Ok(sqlx::query("SELECT * FROM media WHERE format = $1 AND metadata_version < $2")
            .bind(format_type)
            .bind(metadata_version)
            .fetch_all(db)
            .await?
            .iter()
            .map(|row| row.into())
            .collect())
    }

    pub async fn delete<'a>(&self, db: impl SqliteExecutor<'a>) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM media WHERE id = $1")
            .bind(self.id)
            .execute(db)
            .await?;
        Ok(())
    }
}

