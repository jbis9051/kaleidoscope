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
pub struct CustomMetadata {
    pub id: i32,
    pub media_id: i32,
    pub version: i32,
    pub key: String,
    pub value: String,
    #[serde(with = "date")]
    pub created_at: NaiveDateTime,
}


sqlize!(CustomMetadata, "custom_metadata", id, [
    media_id,
    version,
    key,
    value,
    created_at
]);