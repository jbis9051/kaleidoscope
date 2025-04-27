use crate::models::date;
use crate::question_marks;
use sqlx::sqlite::SqliteRow;
use sqlx::{Row};
use std::borrow::Borrow;
use chrono::NaiveDateTime;
use serde::{Serialize};
use crate::{sqlize, update_set};
use crate::types::{SqliteAcquire};


#[derive(Serialize, Debug)]
pub struct CustomTaskMedia {
    pub id: i32,
    pub media_id: i32,
    pub task_name: String,
    pub version: i32,
    #[serde(with = "date")]
    pub created_at: NaiveDateTime,
}


sqlize!(CustomTaskMedia, "custom_task_media", id, [
    media_id,
    version,
    task_name,
    created_at
]);