use sqlx::sqlite::SqliteRow;
use sqlx::types::chrono::NaiveDateTime;
use sqlx::{Row, SqliteExecutor};
use std::borrow::Borrow;
use serde::Serialize;
use crate::models::date;


use crate::types::DbPool;

#[derive(Serialize, Debug)]
pub struct Kv {
    pub id: i32,
    pub key: String,
    pub value: String,
    #[serde(with = "date")]
    pub created_at: NaiveDateTime,
    #[serde(with = "date")]
    pub updated_at: NaiveDateTime,
}

impl From<&SqliteRow> for Kv {
    fn from(row: &SqliteRow) -> Self {
        Self {
            id: row.get("id"),
            key: row.get("key"),
            value: row.get("value"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    }
}

impl Kv {

    pub async fn create<'a>(&mut self, db: impl SqliteExecutor<'a>) -> Result<(), sqlx::Error> {
        self.created_at = chrono::Utc::now().naive_utc();
        self.updated_at = self.created_at;

        *self = sqlx::query(
            "INSERT INTO kv (key, value, created_at, updated_at) \
            VALUES ($1, $2, $3, $4) RETURNING *;",
            )
        .bind(&self.key)
        .bind(&self.value)
        .bind(self.created_at)
        .bind(self.updated_at)
        .fetch_one(db)
        .await?
        .borrow()
        .into();
        Ok(())
    }

    pub async fn from_key<'a>(db: impl SqliteExecutor<'a>, key: &str) -> Result<Option<Self>, sqlx::Error> {
        Ok(sqlx::query("SELECT * FROM kv WHERE key = $1;")
            .bind(key)
            .fetch_optional(db)
            .await?
            .map(|row| row.borrow().into()))
    }

    pub async fn update_by_key<'a>(&mut self, db: impl SqliteExecutor<'a>) -> Result<(), sqlx::Error> {
        self.updated_at = chrono::Utc::now().naive_utc();
        *self = sqlx::query(
            "UPDATE kv SET value = $1, updated_at = $2 WHERE key = $3 RETURNING *;",
            )
        .bind(&self.value)
        .bind(self.updated_at)
        .bind(&self.key)
        .fetch_one(db)
        .await?
        .borrow()
        .into();
        Ok(())
    }

    pub async fn delete(&self, db: &DbPool) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM kv WHERE id = $1")
            .bind(self.id)
            .execute(db)
            .await?;
        Ok(())
    }
}
