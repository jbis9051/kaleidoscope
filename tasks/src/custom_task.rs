use std::borrow::Borrow;
use common::media_query::MediaQuery;
use common::models::media::Media;
use common::scan_config::{AppConfig, CustomConfig};
use common::types::AcquireClone;
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Acquire;
use std::collections::HashMap;
use log::{debug, info};
use sqlx::types::chrono::Utc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use common::models::custom_task_media::CustomTaskMedia;
use crate::tasks::{AnyTask, TaskError};
use crate::tasks::thumbnail::ThumbnailGenerator;

macro_rules! python_func {
    ($($async:tt)? fn $name:tt($db:tt: &mut impl AcquireClone, $media:tt: &Media, $app_config:tt: &AppConfig, $version:tt: i32| $($arg:tt:$typ:ty),*)->$out:ty $code:block) => {
        $($async)? fn $name($db: &mut impl AcquireClone, $media: &Media, $app_config: &AppConfig, $version: i32, args: Vec<Value>) -> Result<String, TaskError> {
            let ($($arg),*,): ($($typ),*,) = serde_json::from_value(serde_json::Value::Array(args)).unwrap();
            let code = async move $code;
            let output: Result<$out, TaskError> = code.await;
            let output = output?;
            return Ok(serde_json::to_string(&output).unwrap());
        }
    };

    // TODO: remove this once we have a nice trait
    (
         @raw
    $($async:tt)? fn $name:tt($db:tt: &mut impl AcquireClone, $media:tt: &Media, $app_config:tt: &AppConfig, $version:tt: i32| $($arg:tt:$typ:ty),*)-> String $code:block) => {
        $($async)? fn $name($db: &mut impl AcquireClone, $media: &Media, $app_config: &AppConfig, $version: i32, args: Vec<Value>) -> Result<String, TaskError> {
            let ($($arg),*,): ($($typ),*,) = serde_json::from_value(serde_json::Value::Array(args)).unwrap();
            let code = async move $code;
            let output: Result<String, TaskError> = code.await;
            let output = output?;
            return Ok(output);
        }
    };
}

python_func!(
    @raw
    async fn execute_task(db: &mut impl AcquireClone, media: &Media, app_config: &AppConfig, version: i32| task_name: String, task_args_str: String) -> String {
        AnyTask::run_custom_anywhere(&task_name, db, &app_config.remote, app_config, &task_args_str).await
    }
);

python_func!(
    async fn add_tag(db: &mut impl AcquireClone, media: &Media, app_config: &AppConfig, version: i32 | tag_name: String) -> bool {
        let tags = media.tags(db.acquire_clone()).await.unwrap();
        if tags.iter().any(|t| t.tag == tag_name) {
            return Ok(false);
        }
        media.add_tag(db, tag_name).await.unwrap();
        Ok(true)
    }
);

python_func!(
    async fn has_tag(db: &mut impl AcquireClone, media: &Media, app_config: &AppConfig, version: i32 | tag_name: String) -> bool {
        let tags = media.tags(db.acquire_clone()).await.unwrap();
        if tags.iter().any(|t| t.tag == tag_name) {
            return Ok(true);
        }
        Ok(false)
    }
);

python_func!(
     async fn remove_tag(db: &mut impl AcquireClone, media: &Media, app_config: &AppConfig, version: i32 | tag_name: String) -> bool {
        Ok(media.remove_tag(db.acquire_clone(), &tag_name).await.unwrap())
    }
);

python_func!(
     async fn add_metadata(db: &mut impl AcquireClone, media: &Media, app_config: &AppConfig, version: i32| key: String, value: String, include_search: bool) -> bool {
        if media.custom(db.acquire_clone(), &key, version).await.unwrap().is_some() {
            return Ok(false);
        }
        media.add_custom(db, key, value, version, include_search).await.unwrap();
        Ok(true)
    }
);

python_func!(
     async fn delete_metadata(db: &mut impl AcquireClone, media: &Media, app_config: &AppConfig, version: i32| key: String) -> bool {
        Ok(media.remove_custom(db.acquire_clone(), &key, version).await.unwrap())
     }
);

python_func!(
     async fn get_metadata(db: &mut impl AcquireClone, media: &Media, app_config: &AppConfig, version: i32| key: String) -> Option<String> {
        Ok(media.latest_custom(db.acquire_clone(), &key).await.unwrap().map(|v| v.value))
     }
);

python_func!(
     async fn log(db: &mut impl AcquireClone, media: &Media, app_config: &AppConfig, version: i32| values: Value) -> () {
        info!("task log: {:?}", values);
        Ok(())
     }
);

python_func!(
     async fn get_thumb(db: &mut impl AcquireClone, media: &Media, app_config: &AppConfig, version: i32|full: bool) -> String {
        let path = if full { ThumbnailGenerator::full_path(&media, app_config) } else {ThumbnailGenerator::thumb_path(&media, app_config)};
        Ok(path.to_str().unwrap().to_string())
     }
);

pub async fn call_fn(
    db: &mut impl AcquireClone,
    media: &Media,
    app_config: &AppConfig,
    version: i32,
    fn_call: FnCall,
) -> Result<String, TaskError> {
    match fn_call.name.as_str() {
        "execute_task" => execute_task(db, media, app_config, version, fn_call.args).await,
        "add_tag" => add_tag(db, media, app_config, version, fn_call.args).await,
        "remove_tag" => remove_tag(db, media, app_config, version, fn_call.args).await,
        "has_tag" => has_tag(db, media, app_config, version, fn_call.args).await,
        "add_metadata" => add_metadata(db, media, app_config, version, fn_call.args).await,
        "delete_metadata" => delete_metadata(db, media, app_config, version, fn_call.args).await,
        "get_metadata" => get_metadata(db, media, app_config, version, fn_call.args).await,
        "log" => log(db, media, app_config, version, fn_call.args).await,
        "get_thumb" => get_thumb(db, media, app_config, version, fn_call.args).await,
        _ => panic!("function '{}' not found", fn_call.name),
    }
}

#[derive(Deserialize, Debug)]
pub(crate) struct FnCall {
    name: String,
    args: Vec<Value>,
    kwargs: HashMap<String, Value>,
}
