use chrono::{NaiveDateTime, TimeDelta, Utc};
use common::remote_models::job::{Job, JobStatus};
use reqwest::multipart::Form;
use reqwest::{Client, RequestBuilder, Response, StatusCode};
use std::ops::{Add, Sub};
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;
use tokio::time::sleep;
use uuid::Uuid;

pub struct RemoteRequester {
    task_name: String,
    url: String,
    password: Option<String>,
    background: bool,
}

impl RemoteRequester {
    pub fn new(task_name: String, url: String, password: Option<String>, background: bool) -> Self {
        if let Some(password) = password {
            Self {
                task_name,
                url,
                password: Some(format!("Bearer {}", password).to_string()),
                background,
            }
        } else {
            Self {
                task_name,
                url,
                password: None,
                background,
            }
        }
    }

    pub fn post(&self, path: &str) -> RequestBuilder {
        let mut req = Client::new().post(format!("{}{}", self.url, path));
        if let Some(password) = &self.password {
            req = req.header("Authorization", password.clone());
        }
        req
    }

    pub fn get(&self, path: &str) -> RequestBuilder {
        let mut req = Client::new().get(format!("{}{}", self.url, path));
        if let Some(password) = &self.password {
            req = req.header("Authorization", password.clone());
        }
        req
    }

    pub async fn request_multipart(&self, form: Form) -> Result<Response, RequestError> {
        let path = if self.background { "background" } else { "custom" };
        let res = self
            .post(&format!("/task/{}/{}", &self.task_name, path))
            .multipart(form)
            .send()
            .await?;

        if res.status() == StatusCode::CONFLICT {
            return Err(RequestError::ServerBusy(res.text().await?));
        }

        Ok(res)
    }

    pub async fn wait_for_completion(&self, job_uuid: &Uuid) -> Result<Job, RequestError> {
        loop {
            let request = self.get(&format!("/job/{}", &job_uuid)).send().await?;
            if request.status() == StatusCode::NOT_FOUND {
                return Err(RequestError::JobNotFound);
            }
            let job = request.json::<Job>().await?;
            if job.status == JobStatus::Running {
                if let Some(estimate) = job.estimated_completion {
                    let diff = estimate.sub(Utc::now().naive_utc());
                    if diff.num_seconds() > 0 {
                        sleep(Duration::from_secs(diff.num_seconds() as u64)).await;
                        continue;
                    }
                }
                // if there's no estimate or the estimate is outdated then we wait 10 seconds
                sleep(Duration::from_secs(10)).await;
                continue;
            }
            return Ok(job);
        }
    }

    pub async fn one_shot_file<T: AsRef<Path>>(&self, key: String, path: T, media_uuid: Option<Uuid>) -> Result<OneShotResponse, RequestError> {
        let mut form = Form::new();
        if let Some(uuid) = media_uuid {
            form = form.text("media_uuid", uuid.to_string());
        }
        form = form.file(key, path).await?;
        let res = self.request_multipart(form).await?;
        match res.status() {
            StatusCode::OK => Ok(OneShotResponse::Response(res)),
            StatusCode::CREATED => {
                // job has been created, let's wait
                let job_uuid = Uuid::from_str(&res.text().await?).expect("invalid uuid from server");
                let job = self.wait_for_completion(&job_uuid).await?;
                Ok(OneShotResponse::Job(job))
            }
            _ => Err(RequestError::Response(res))
        }
    }
}

#[derive(Debug)]
pub enum OneShotResponse {
    Job(Job),
    Response(Response),
}

#[derive(thiserror::Error, Debug)]
pub enum RequestError {
    #[error("reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("server busy: {0}")]
    ServerBusy(String),
    #[error("job not found")]
    JobNotFound,
    #[error("iO error {0}")]
    Io(#[from] std::io::Error),
    #[error("error response {0:?}")]
    Response(Response),
}
