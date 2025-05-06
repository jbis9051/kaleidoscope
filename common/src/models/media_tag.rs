use crate::question_marks;
use sqlx::sqlite::SqliteRow;
use sqlx::{Row};
use std::borrow::Borrow;
use serde::{Deserialize, Serialize};
use crate::{sqlize, update_set};
use crate::types::{SqliteAcquire};


#[derive(Serialize, Debug)]
pub struct MediaTag {
    pub id: i32,
    pub media_id: i32,
    pub tag: String,
    // this is the custom_task name used to add the tag
    pub task: Option<String>, 
}


sqlize!(MediaTag, "media_tag", id, [
    media_id,
    tag,
    task
]);

impl MediaTag {
    pub async fn count_index(db: impl SqliteAcquire<'_>) -> Result<Vec<(MediaTag, u32)>, sqlx::Error> {
        let mut conn = db.acquire().await?;
        Ok(sqlx::query("SELECT *, COUNT(*) as count FROM media_tag GROUP BY tag")
            .fetch_all(&mut *conn)
            .await?
            .into_iter()
            .map(|r| {
                let count = r.get("count");
                (r.borrow().into(), count)
            })
            .collect())
    }
}