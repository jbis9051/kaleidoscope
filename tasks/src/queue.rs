use std::borrow::Borrow;
use sqlx::{Executor, Row, Sqlite};
use sqlx::SqliteExecutor;
use sqlx::sqlite::SqliteRow;
use common::update_set;
use common::question_marks;
use common::models::date;
use serde::Serialize;
use common::sqlize;
use common::types::SqliteAcquire;

#[derive(Serialize)]
pub struct Queue {
    pub id: i32,
    pub media_id: i32,
    pub task: String,
    #[serde(with = "date")]
    pub created_at: chrono::NaiveDateTime,
}

sqlize!(Queue, "queue", id, [
    media_id,
    task,
    created_at
]);

impl Queue {
    pub async fn get_next(db: impl SqliteAcquire<'_>, task: &str) -> Result<Option<Queue>, sqlx::Error>
    {
        let mut conn = db.acquire().await?;
        let queue = sqlx::query("SELECT * FROM queue WHERE task = ? ORDER BY created_at ASC LIMIT 1")
            .bind(task)
            .fetch_optional(&mut *conn)
            .await?;
        match queue {
            Some(row) => Ok(Some(row.borrow().into())),
            None => Ok(None),
        }
    }

    pub async fn from_media_id(db: impl SqliteAcquire<'_>, task: &str, media_id: i32) -> Result<Option<Queue>, sqlx::Error> {
        let mut conn = db.acquire().await?;
        let queue = sqlx::query("SELECT * FROM queue WHERE media_id = ? AND task = ?")
            .bind(media_id)
            .bind(task)
            .fetch_optional(&mut *conn)
            .await?;
        match queue {
            Some(row) => Ok(Some(row.borrow().into())),
            None => Ok(None),
        }
    }

    pub async fn count(db: impl SqliteAcquire<'_>, task: &str) -> Result<u32, sqlx::Error> {
        let mut conn = db.acquire().await?;
        let count = sqlx::query("SELECT COUNT(*) FROM queue WHERE task = ?")
            .bind(task)
            .fetch_one(&mut *conn)
            .await?;
        Ok(count.get(0))
    }
    
    pub async fn delete(&self, db: impl SqliteAcquire<'_>) -> Result<(), sqlx::Error> {
        let mut conn = db.acquire().await?;
        sqlx::query("DELETE FROM queue WHERE id = ?")
            .bind(self.id)
            .execute(&mut *conn)
            .await?;
        Ok(())
    }
}