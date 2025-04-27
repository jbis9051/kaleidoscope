use crate::question_marks;
use sqlx::sqlite::SqliteRow;
use sqlx::types::chrono::NaiveDateTime;
use sqlx::{Execute, Row, SqliteExecutor};
use std::borrow::Borrow;
use serde::Serialize;
use crate::{sqlize, update_set};
use crate::types::{DbPool, SqliteAcquire};


#[derive(Debug, Serialize, Clone)]
pub struct MediaExtra {
    pub id: i32,
    pub media_id: i32,
    pub whisper_version: i32,
    pub whisper_language: Option<String>,
    pub whisper_confidence: Option<f32>,
    pub whisper_transcript: Option<String>,
    pub vision_ocr_version: i32,
    pub vision_ocr_result: Option<String>,
}

impl Default for MediaExtra {
    fn default() -> Self {
        Self {
            id: -1,
            media_id: -1,
            whisper_version: -1,
            whisper_language: None,
            whisper_confidence: None,
            whisper_transcript: None,
            vision_ocr_version: -1,
            vision_ocr_result: None,
        }
    }
}

sqlize!(MediaExtra, "media_extra", id, [
    media_id,
    whisper_version,
    whisper_language,
    whisper_confidence,
    whisper_transcript,
    vision_ocr_version,
    vision_ocr_result
]);

impl MediaExtra {

    // see: https://github.com/launchbadge/sqlx/issues/2093, remove when fixed
    pub async fn create_no_bug(&mut self, db: impl SqliteAcquire<'_>) -> Result<(), sqlx::Error> {
        let mut conn = db.acquire().await?;
        let res = sqlx::query("INSERT INTO media_extra (media_id, whisper_version, whisper_language, whisper_confidence, whisper_transcript, vision_ocr_version, vision_ocr_result) VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING id")
            .bind(&self.media_id)
            .bind(&self.whisper_version)
            .bind(&self.whisper_language)
            .bind(&self.whisper_confidence)
            .bind(&self.whisper_transcript)
            .bind(&self.vision_ocr_version)
            .bind(&self.vision_ocr_result)
            .fetch_one(&mut *conn)
            .await?;
        
        let id = res.get(0);
        
        self.id = id;
        
        Ok(())
    }
    
    pub async fn from_id(db: impl SqliteAcquire<'_>, id: &i32) -> Result<Self, sqlx::Error> {
        let mut conn = db.acquire().await?;
        Ok(sqlx::query("SELECT * FROM media_extra WHERE id = $1;")
            .bind(id)
            .fetch_one(&mut *conn)
            .await?
            .borrow()
            .into())
    }

    pub async fn delete(&self,db: impl SqliteAcquire<'_>) -> Result<(), sqlx::Error> {
        let mut conn = db.acquire().await?;
        sqlx::query("DELETE FROM media_extra WHERE id = $1")
            .bind(self.id)
            .execute(&mut *conn)
            .await?;
        Ok(())
    }
}

