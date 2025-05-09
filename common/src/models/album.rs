use sqlx::sqlite::SqliteRow;
use sqlx::types::chrono::NaiveDateTime;
use sqlx::{Row, SqliteExecutor};
use std::borrow::Borrow;
use serde::Serialize;
use uuid::{Uuid};
use crate::media_query::MediaQuery;
use crate::models::media::{Media};
use crate::models::{date, MediaError};


use crate::types::DbPool;

#[derive(Serialize, Debug)]
pub struct Album {
    pub id: i32,
    pub uuid: Uuid,
    pub name: String,
    #[serde(with = "date")]
    pub created_at: NaiveDateTime,
}

impl From<&SqliteRow> for Album {
    fn from(row: &SqliteRow) -> Self {
        Album {
            id: row.get("id"),
            uuid: row.get("uuid"),
            name: row.get("name"),
            created_at: row.get("created_at"),
        }
    }
}

impl Album {
    pub async fn create(&mut self, db: &DbPool) -> Result<(), sqlx::Error> {
        *self = sqlx::query(
            "INSERT INTO album (uuid, name, created_at) \
            VALUES ($1, $2, $3) RETURNING *;",
            )
        .bind(self.uuid)
        .bind(&self.name)
        .bind(self.created_at)
        .fetch_one(db)
        .await?
        .borrow()
        .into();
        Ok(())
    }

    pub async fn from_uuid(db: &DbPool, uuid: &Uuid) -> Result<Self, sqlx::Error> {
        Ok(sqlx::query("SELECT * FROM album WHERE uuid = $1;")
            .bind(uuid)
            .fetch_one(db)
            .await?
            .borrow()
            .into())
    }

    pub async fn delete(&self, db: &DbPool) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM album WHERE id = $1")
            .bind(self.id)
            .execute(db)
            .await?;
        Ok(())
    }

    pub async fn get_all(db: &DbPool, ) -> Result<Vec<Album>, sqlx::Error> {
        Ok(sqlx::query("SELECT * FROM album")
            .fetch_all(db)
            .await?
            .iter()
            .map(|row| row.into())
            .collect())
    }

    pub async fn count(db: &DbPool) -> Result<i32, sqlx::Error> {
        Ok(sqlx::query("SELECT COUNT(*) FROM album;")
            .fetch_one(db)
            .await?
            .get(0))
    }

    pub async fn count_media(&self, db: &DbPool) -> Result<u32, MediaError> {
        Ok(sqlx::query("SELECT COUNT(*) FROM media INNER JOIN album_media ON media.id = album_media.media_id WHERE album_media.album_id = ?")
            .bind(self.id)
            .fetch_one(db)
            .await?
            .get(0))
    }

    pub async fn get_media(&self, db: &DbPool, media_query: &MediaQuery) -> Result<Vec<Media>, MediaError> {

        let mut query = sqlx::QueryBuilder::new("SELECT media.* FROM media \
            INNER JOIN album_media ON media.id = album_media.media_id ");
        
        media_query.add_tables(&mut query);

        query.push(" WHERE album_media.album_id = ");

        query
            .push_bind(self.id);
        
        media_query.add_queries(&mut query)?;
        
        let query = query.build();
        
        Ok(query
            .fetch_all(db)
            .await?
            .iter()
            .map(|row| row.into())
            .collect())
    }

    pub async fn add_media<'a>(&self, db: impl SqliteExecutor<'a>, media_id: i32) -> Result<(), sqlx::Error> {
        sqlx::query("INSERT INTO album_media (album_id, media_id) VALUES ($1, $2);")
            .bind(self.id)
            .bind(media_id)
            .execute(db)
            .await?;
        Ok(())
    }

    pub async fn has_media<'a>(&self, db: impl SqliteExecutor<'a>, media_id: i32) -> Result<bool, sqlx::Error> {
        Ok(sqlx::query("SELECT EXISTS(SELECT 1 FROM album_media WHERE album_id = $1 AND media_id = $2);")
            .bind(self.id)
            .bind(media_id)
            .fetch_one(db)
            .await?
            .get(0))
    }

    pub async fn remove_media<'a>(&self, db: impl SqliteExecutor<'a>, media_id: i32) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM album_media WHERE album_id = $1 AND media_id = $2;")
            .bind(self.id)
            .bind(media_id)
            .execute(db)
            .await?;
        Ok(())
    }
}

