use std::future::Future;
use chrono::{NaiveDateTime};
use serde::Serialize;
use uuid::Uuid;
use common::remote_models::job::{Job, JobStatus};
use common::types::{AcquireClone};

pub async fn start_job<Suc, Err, Fut, Db>(task_name: String, media_uuid: Uuid, estimate: Option<NaiveDateTime>, mut db: Db, f: impl FnOnce(Job) -> Fut) -> Result<Job, sqlx::Error>
where
    Db: AcquireClone + Send + 'static,
    Suc: Serialize,
    Err: Serialize,
    Fut: Future<Output = Result<Option<Suc>, Option<Err>>> + Send + 'static,
    Fut::Output: Send + 'static,
{
    let job = Job::new(&mut db, task_name, media_uuid, estimate).await?;
    let ser = serde_json::to_string(&job).unwrap();
    let des: Job = serde_json::from_str(&ser).unwrap();
    let job_uuid = job.uuid.clone();
    let future = f(job.clone());
    tokio::spawn(async move {
        // run the actual job
        let result = future.await;
        // retrieve the job
        let mut job = Job::try_from_uuid(db.acquire_clone(), &job_uuid).await.expect("couldn't get job").expect("couldn't find job");
        match result {
            Ok(suc) => {
                job.status = JobStatus::Success;
                if let Some(suc) = suc {
                    job.success_data = Some(serde_json::to_string(&suc).expect("error serializing success data"));
                }
            }
            Err(err) => {
                job.status = JobStatus::Failed;
                if let Some(err) = err {
                    job.failure_data = Some(serde_json::to_string(&err).expect("error serializing failure data"));
                }
            }
        }
        job.update_by_id(db.acquire_clone()).await.expect("couldn't update job")
    });
    Ok(job)
}