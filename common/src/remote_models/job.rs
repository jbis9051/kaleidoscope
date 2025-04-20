use crate::question_marks;
use sqlx::sqlite::SqliteRow;
use sqlx::types::chrono::NaiveDateTime;
use sqlx::{Row, SqliteExecutor};
use std::borrow::Borrow;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::{Uuid};
use crate::models::{date, option_date};
use crate::{sqlize, update_set};
use crate::types::{AcquireClone, SqliteAcquire};



#[derive(Serialize, Deserialize, sqlx::Type, Debug, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
#[sqlx(rename_all = "lowercase")]
pub enum JobStatus {
    Success,
    Running,
    Failed,
    Cancelled, // indicates the server stopped the the job, reason will be specified in the failure data field as text
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Job {
    pub id: i32,
    pub uuid: Uuid,
    pub media_uuid: Uuid,
    pub task_name: String,
    pub status: JobStatus,
    #[serde(with = "option_date")]
    pub estimated_completion: Option<NaiveDateTime>,
    pub success_data: Option<String>,
    pub failure_data: Option<String>,
    #[serde(with = "date")]
    pub created_at: NaiveDateTime,
    #[serde(with = "date")]
    pub updated_at: NaiveDateTime,
}

sqlize!(Job, "job", id, [
    uuid,
    media_uuid,
    task_name,
    status,
    estimated_completion,
    success_data,
    failure_data,
    created_at,
    updated_at
]);

impl Job {
    
    pub async fn new(db: &mut impl AcquireClone, task_name: String, media_uuid: Uuid, estimated_completion: Option<NaiveDateTime>) -> Result<Self, sqlx::Error> {
        let job_uuid = Uuid::new_v4();
        let mut job = Self {
            id: 0,
            uuid: job_uuid,
            media_uuid,
            task_name,
            status: JobStatus::Running,
            estimated_completion,
            success_data: None,
            failure_data: None,
            created_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
        };
        job.create(db.acquire_clone()).await?;
        Ok(job)
    }

    pub async fn get_by_status(db: impl SqliteAcquire<'_>, status: &JobStatus) -> Result<Vec<Self>, sqlx::Error> {
        let mut conn = db.acquire().await?;
        Ok(sqlx::query("SELECT * FROM job WHERE status = ?")
            .bind(status)
            .fetch_all(&mut *conn)
            .await?
            .iter()
            .map(|row| row.into())
            .collect())
    }

    pub async fn try_from_uuid(db: impl SqliteAcquire<'_>, uuid: &Uuid) -> Result<Option<Self>, sqlx::Error> {
        let mut conn = db.acquire().await?;
        Ok(sqlx::query("SELECT * FROM job WHERE uuid = $1;")
            .bind(uuid)
            .fetch_optional(&mut *conn)
            .await?
            .map(|r| r.borrow().into()))
    }

    pub async fn cancel_all(db: impl SqliteAcquire<'_>, reason: &str) -> Result<u64, sqlx::Error> {
        let mut conn = db.acquire().await?;
        let res = sqlx::query("UPDATE job SET status = ?, failure_data = ?, estimated_completion=NULL, updated_at = ? WHERE status = ?;")
            .bind(JobStatus::Cancelled)
            .bind(reason)
            .bind(Utc::now().naive_utc())
            .bind(JobStatus::Running)
            .execute(&mut *conn)
            .await?;
        Ok(res.rows_affected())
    }

    pub async fn delete<'a>(&self, db: impl SqliteExecutor<'a>) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM job WHERE id = $1")
            .bind(self.id)
            .execute(db)
            .await?;
        Ok(())
    }
}

