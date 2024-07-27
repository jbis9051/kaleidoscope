use sqlx::sqlite::SqliteRow;
use sqlx::types::chrono::NaiveDateTime;
use sqlx::{Row, SqliteExecutor};
use std::borrow::Borrow;
use serde::Serialize;
use uuid::{Uuid};
use crate::media_query::MediaQuery;
use crate::models::date;

use crate::types::DbPool;

#[derive(Debug, Serialize, PartialEq)]
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
    pub is_photo: bool,
    #[serde(with = "date")]
    pub added_at: NaiveDateTime,
    pub duration: Option<u32>,
    pub hash: String,
    // in bytes
    pub size: u32,
    #[serde(with = "date")]
    pub file_created_at: NaiveDateTime
}

impl From<&SqliteRow> for Media {
    fn from(row: &SqliteRow) -> Self {
        Media {
            id: row.get("id"),
            uuid: row.get("uuid"),
            name: row.get("name"),
            created_at: row.get("created_at"),
            width: row.get("width"),
            height: row.get("height"),
            size: row.get("size"),
            path: row.get("path"),
            liked: row.get("liked"),
            is_photo: row.get("is_photo"),
            added_at: row.get("added_at"),
            duration: row.get("duration"),
            hash: row.get("hash"),
            file_created_at: row.get("file_created_at")
        }
    }
}

impl Media {
    pub async fn create<'a, T: SqliteExecutor<'a>>(&mut self, db: T) -> Result<(), sqlx::Error> {
        *self = sqlx::query(
            "INSERT INTO media (uuid, name, created_at, width, height, size, path, liked, is_photo, added_at, duration, hash, file_created_at) \
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13) RETURNING *;",
        )
            .bind(self.uuid)
            .bind(&self.name)
            .bind(self.created_at)
            .bind(self.width)
            .bind(self.height)
            .bind(self.size)
            .bind(&self.path)
            .bind(self.liked)
            .bind(self.is_photo)
            .bind(self.added_at)
            .bind(self.duration)
            .bind(&self.hash)
            .bind(self.file_created_at)
            .fetch_one(db)
            .await?
            .borrow()
            .into();
        Ok(())
    }

    pub fn safe_column(name: &str) -> Result<(), sqlx::Error> {
        match name {
            "id" | "uuid" | "name" | "created_at" | "width" | "height" | "size" | "path" | "liked" | "is_photo" | "added_at" | "duration" => Ok(()),
            _ => Err(sqlx::Error::ColumnNotFound(name.to_string()))
        }
    }

    pub async fn get_all(db: &DbPool, media_query: &MediaQuery) -> Result<Vec<Self>, sqlx::Error> {
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

    pub async fn count(db: &DbPool, media_query: &MediaQuery) -> Result<u32, sqlx::Error> {
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

    pub async fn from_path<'a>(db: impl SqliteExecutor<'a>, path: &str) -> Result<Option<Self>, sqlx::Error> {
        Ok(sqlx::query("SELECT * FROM media WHERE path = $1;")
            .bind(path)
            .fetch_optional(db)
            .await?
            .map(|row| row.borrow().into()))
    }

    pub async fn delete<'a>(&self, db: impl SqliteExecutor<'a>) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM media WHERE id = $1")
            .bind(self.id)
            .execute(db)
            .await?;
        Ok(())
    }
}
