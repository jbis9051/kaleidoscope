use sqlx::sqlite::SqliteRow;
use sqlx::types::chrono::NaiveDateTime;
use sqlx::{Row, SqliteExecutor};
use std::borrow::Borrow;
use serde::Serialize;
use uuid::{Uuid};
use crate::media_query::MediaQuery;
use crate::models::media::Media;
use crate::models::date;


use crate::types::DbPool;

#[derive(Serialize, Debug)]
pub struct MediaView {
    pub id: i32,
    pub uuid: Uuid,
    pub name: String,
    pub view_query: String,
    #[serde(with = "date")]
    pub created_at: NaiveDateTime,
}

impl From<&SqliteRow> for MediaView {
    fn from(row: &SqliteRow) -> Self {
        MediaView {
            id: row.get("id"),
            uuid: row.get("uuid"),
            name: row.get("name"),
            view_query: row.get("view_query"),
            created_at: row.get("created_at"),
        }
    }
}

impl MediaView {
    pub async fn create(&mut self, db: &DbPool) -> Result<(), sqlx::Error> {
        *self = sqlx::query(
            "INSERT INTO media_view (uuid, name, view_query, created_at) \
            VALUES ($1, $2, $3, $4) RETURNING *;",
            )
        .bind(self.uuid)
        .bind(&self.name)
        .bind(&self.view_query)
        .bind(self.created_at)
        .fetch_one(db)
        .await?
        .borrow()
        .into();
        Ok(())
    }

    pub async fn from_uuid(db: &DbPool, uuid: &Uuid) -> Result<Self, sqlx::Error> {
        Ok(sqlx::query("SELECT * FROM media_view WHERE uuid = $1;")
            .bind(uuid)
            .fetch_one(db)
            .await?
            .borrow()
            .into())
    }

    pub async fn delete(&self, db: &DbPool) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM media_view WHERE id = $1")
            .bind(self.id)
            .execute(db)
            .await?;
        Ok(())
    }

    pub async fn get_all(db: &DbPool) -> Result<Vec<MediaView>, sqlx::Error> {
        Ok(sqlx::query("SELECT * FROM media_view")
            .fetch_all(db)
            .await?
            .iter()
            .map(|row| row.into())
            .collect())
    }
}
