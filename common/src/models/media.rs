use crate::question_marks;
use sqlx::sqlite::SqliteRow;
use sqlx::types::chrono::NaiveDateTime;
use sqlx::{Acquire, Execute, Row, SqliteExecutor};
use std::borrow::Borrow;
use chrono::Utc;
use serde::Serialize;
use uuid::{Uuid};
use crate::media_query::MediaQuery;
use crate::models::{date, MediaError};
use crate::{sqlize, update_set};
use crate::media_processors::format::{FormatType, MediaType};
use crate::models::custom_metadata::CustomMetadata;
use crate::models::media_extra::MediaExtra;
use crate::models::media_tag::MediaTag;
use crate::types::{AcquireClone, DbPool, SqliteAcquire};


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

    pub async fn get_all(db: impl SqliteAcquire<'_>, media_query: &MediaQuery) -> Result<Vec<Self>, MediaError> {
        let mut conn = db.acquire().await?;
        let mut query = sqlx::QueryBuilder::new("SELECT DISTINCT media.* FROM media ");

        media_query.sqlize(&mut query)?;
        let query = query.build();

        Ok(query
            .fetch_all(&mut *conn)
            .await?
            .iter()
            .map(|row| row.into())
            .collect())
    }

    pub async fn count(db: &DbPool, media_query: &MediaQuery) -> Result<u32, MediaError> {
        let mut query = sqlx::QueryBuilder::new("SELECT COUNT(DISTINCT media.id) FROM media ");

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
    
    pub async fn extra(&self, db: impl SqliteAcquire<'_>) -> Result<Option<MediaExtra>, sqlx::Error>{
        let mut conn = db.acquire().await?;
        Ok(sqlx::query("SELECT * FROM media_extra WHERE media_id = $1;")
            .bind(self.id)
            .fetch_optional(&mut *conn)
            .await?
            .map(|row| row.borrow().into()))
    }

    pub async fn add_tag(&self, db: &mut impl AcquireClone, tag: String) -> Result<MediaTag, sqlx::Error> {
        let mut tag = MediaTag {
            id: 0,
            media_id: self.id,
            tag,
        };
        tag.create(db.acquire_clone()).await?;
        Ok(tag)
    }

    pub async fn tags(&self, db: impl SqliteAcquire<'_>) -> Result<Vec<MediaTag>, sqlx::Error> {
        let mut conn = db.acquire().await?;
        Ok(sqlx::query("SELECT * FROM media_tag WHERE media_id = $1 ORDER BY media_tag.id")
            .bind(self.id)
            .fetch_all(&mut *conn)
            .await?
            .into_iter()
            .map(|row| row.borrow().into())
            .collect())
    }

    pub async fn remove_tag(&self, db: impl SqliteAcquire<'_>, tag: &str) -> Result<bool, sqlx::Error> {
        let mut conn = db.acquire().await?;
        let res = sqlx::query("DELETE FROM media_tag WHERE media_id = $1 AND tag = $2;")
            .bind(self.id)
            .bind(tag)
            .execute(&mut *conn)
            .await?;
        Ok(res.rows_affected() > 0)
    }


    pub async fn add_custom(&self, db: &mut impl AcquireClone, key: String, value: String, version: i32, include_search: bool) -> Result<CustomMetadata, sqlx::Error> {
        let mut custom = CustomMetadata {
            id: 0,
            media_id: self.id,
            version,
            key,
            value,
            created_at: Utc::now().naive_utc(),
            include_search,
        };
        custom.create(db.acquire_clone()).await?;
        Ok(custom)
    }

    pub async fn customs(&self, db: impl SqliteAcquire<'_>) -> Result<Vec<CustomMetadata>, sqlx::Error> {
        let mut conn = db.acquire().await?;
        Ok(sqlx::query("SELECT * FROM custom_metadata WHERE media_id = $1 ORDER BY custom_metadata.id")
            .bind(self.id)
            .fetch_all(&mut *conn)
            .await?
            .into_iter()
            .map(|row| row.borrow().into())
            .collect())
    }

    pub async fn latest_custom(&self, db: impl SqliteAcquire<'_>, key: &str) -> Result<Option<CustomMetadata>, sqlx::Error> {
        let mut conn = db.acquire().await?;
        Ok(sqlx::query("SELECT * FROM custom_metadata WHERE media_id = $1 AND custom_metadata.key = $2 ORDER BY custom_metadata.version DESC")
            .bind(self.id)
            .bind(key)
            .fetch_optional(&mut *conn)
            .await?
            .map(|row| row.borrow().into()))
    }


    pub async fn custom(&self, db: impl SqliteAcquire<'_>, key: &str, version: i32) -> Result<Option<CustomMetadata>, sqlx::Error> {
        let mut conn = db.acquire().await?;
        Ok(sqlx::query("SELECT * FROM custom_metadata WHERE media_id = $1 AND custom_metadata.key = $2 AND custom_metadata.version = $3")
            .bind(self.id)
            .bind(key)
            .bind(version)
            .fetch_optional(&mut *conn)
            .await?
            .map(|row| row.borrow().into()))
    }

    pub async fn remove_custom(&self, db: impl SqliteAcquire<'_>, key: &str, version: i32) -> Result<bool, sqlx::Error> {
        let mut conn = db.acquire().await?;
        let res = sqlx::query("DELETE FROM custom_metadata WHERE media_id = $1 AND key = $2 AND version = $3;")
            .bind(self.id)
            .bind(key)
            .bind(version)
            .execute(&mut *conn)
            .await?;
        Ok(res.rows_affected() > 0)
    }

    pub async fn remove_outdated_custom(&self, db: impl SqliteAcquire<'_>, key: &str, latest_version: i32) -> Result<u64, sqlx::Error> {
        let mut conn = db.acquire().await?;
        let res = sqlx::query("DELETE FROM custom_metadata WHERE media_id = $1 AND key = $2 AND version < $3;")
            .bind(self.id)
            .bind(key)
            .bind(latest_version)
            .execute(&mut *conn)
            .await?;
        Ok(res.rows_affected())
    }

    pub async fn remove_custom_for_version(&self, db: impl SqliteAcquire<'_>, version: i32) -> Result<u64, sqlx::Error> {
        let mut conn = db.acquire().await?;
        let res = sqlx::query("DELETE FROM custom_metadata WHERE media_id = $1 AND version = $3;")
            .bind(self.id)
            .bind(version)
            .execute(&mut *conn)
            .await?;
        Ok(res.rows_affected())
    }

    pub async fn remove_custom_task_media(&self, db: impl SqliteAcquire<'_>, task_name: &str) -> Result<u64, sqlx::Error> {
        let mut conn = db.acquire().await?;
        let res = sqlx::query("DELETE FROM custom_task_media WHERE media_id = $1 AND task_name = $2;")
            .bind(self.id)
            .bind(task_name)
            .execute(&mut *conn)
            .await?;
        Ok(res.rows_affected())
    }
}

