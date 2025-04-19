use std::path::PathBuf;
use axum::extract::Request;
use axum::{Json, RequestExt};
use axum::response::{IntoResponse, Response};
use common::media_processors::format::{AnyFormat, MetadataError};
use common::media_processors::RgbImage;
use common::models::media::Media;
use common::scan_config::AppConfig;
use common::types::{AcquireClone};
use log::debug;
use common::runner_config::RemoteRunnerConfig;
use crate::tasks::{BackgroundTask, RemoteBackgroundTask, TaskError};


pub struct RemoteTest;

impl BackgroundTask for RemoteTest {
    type Error = TaskError;
    const NAME: &'static str = "remote_test";
    type Data = String;
    type Config = ();

    async fn new(db: &mut impl AcquireClone, config: &Self::Config, app_config: &AppConfig) -> Result<Self, Self::Error> {
       Ok(Self)
    }


    async fn compatible(media: &Media) -> bool {
        true
    }

    async fn outdated(&self, db: &mut impl AcquireClone, media: &Media) -> Result<bool, Self::Error> {
       Ok(true)
    }

    async fn run(&self, db: &mut impl AcquireClone, media: &Media) -> Result<Self::Data, Self::Error> {
        unimplemented!()
    }

    async fn run_and_store(&self, db: &mut impl AcquireClone, media: &mut Media) -> Result<(), Self::Error> {
       unimplemented!()
    }

    async fn remove_data(&self, db: &mut impl AcquireClone, media: &mut Media) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl RemoteBackgroundTask for RemoteTest {
    type RemoteClientConfig = String;
    type RunnerConfig = String;

    async fn new_remote(db: &mut impl AcquireClone, runner_config: &Self::RunnerConfig, remote_server_config: &RemoteRunnerConfig) -> Result<Self, Self::Error> {
        Ok(Self)
    }

    async fn remote_handler(&self, request: Request, db: &mut impl AcquireClone, runner_config: &Self::RunnerConfig, remote_server_config: &RemoteRunnerConfig) -> Result<Response, Self::Error> {
        let data: Json<String> = request.extract().await.expect("couldn't extract");
        Ok(format!("Hello {} my name is {}", data.0, runner_config).into_response())
    }

    async fn run_remote(&self, db: &mut impl AcquireClone, media: &Media, server: &Self::RemoteClientConfig) -> Result<Self::Data, Self::Error> {
        let client = reqwest::Client::new();
        let res = client.post(format!("{}/task/{}", server, Self::NAME))
            .json(&media.name)
            .send()
            .await
            .expect("couldn't send request");
        let result = res.text().await.expect("couldn't read response body");
        Ok(result)
    }

    async fn run_remote_and_store(&self, db: &mut impl AcquireClone, media: &mut Media, remote_config: &Self::RemoteClientConfig) -> Result<(), Self::Error> {
        todo!()
    }
}