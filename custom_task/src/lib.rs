use std::collections::HashMap;
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use common::models::media::Media;
use common::scan_config::{AppConfig, CustomConfig};
use common::types::AcquireClone;
use tasks::run_python::run_python;
use tasks::tasks::{AnyTask, TaskError};
use tokio::process::Command;

macro_rules! python_func {
    ($($async:tt)? fn $name:tt($db:tt: &mut impl AcquireClone, $media:tt: &Media, $app_config:tt: &AppConfig, $version:tt: i32| $($arg:tt:$typ:ty),*)->$out:ty $code:block) => {
        $($async)? fn $name($db: &mut impl AcquireClone, $media: &Media, $app_config: &AppConfig, $version: i32, args: Vec<Value>) -> Result<String, TaskError> {
            let ($($arg),*): ($($typ),*) = serde_json::from_value(serde_json::Value::Array(args)).unwrap();
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
            let ($($arg),*): ($($typ),*) = serde_json::from_value(serde_json::Value::Array(args)).unwrap();
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
        AnyTask::run_custom(&task_name, db, &app_config.tasks, app_config, &task_args_str).await
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
     async fn remove_tag(db: &mut impl AcquireClone, media: &Media, app_config: &AppConfig, version: i32 | tag_name: String) -> bool {
        Ok(media.remove_tag(db.acquire_clone(), &tag_name).await.unwrap())
    }
);

python_func!(
     async fn add_metadata(db: &mut impl AcquireClone, media: &Media, app_config: &AppConfig, version: i32| key: String, value: String) -> bool {
        if media.custom(db.acquire_clone(), &key, version).await.unwrap().is_some() {
            return Ok(false);
        }
        media.add_custom(db, key, value, version).await.unwrap();
        Ok(true)
    }
);

python_func!(
     async fn delete_metadata(db: &mut impl AcquireClone, media: &Media, app_config: &AppConfig, version: i32| key: String, value: String) -> bool {
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
        println!("logged: {:?}", values);
        Ok(())
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
        "add_metadata" => add_metadata(db, media, app_config, version, fn_call.args).await,
        "delete_metadata" => delete_metadata(db, media, app_config, version, fn_call.args).await,
        "get_metadata" => get_metadata(db, media, app_config, version, fn_call.args).await,
        "log" => log(db, media, app_config, version, fn_call.args).await,
        _ => panic!("function '{}' not found", fn_call.name),
    }
}


#[derive(Deserialize)]
struct FnCall {
    name: String,
    args: Vec<Value>,
    kwargs: HashMap<String, Value>,
}

pub async fn run_custom(
    db: &mut impl AcquireClone,
    media: &Media,
    app_config: &AppConfig,
    custom: &CustomConfig,
    debug: bool
) -> Result<(), TaskError> {
    let mut child = Command::new(&app_config.python_path)
        .arg(&custom.path)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("unable to spawn custom command");
    
    let mut stdin = child.stdin.take().expect("Failed to open stdin");
    
    stdin.write_all(&serde_json::to_vec(&json!({
        "media": media,
        "version": custom.version,
    })).unwrap()).await.expect("unable to write initial input");
    stdin.write_all(b"\n").await.expect("unable to write initial input");
    
    let stdout = child.stdout.take().expect("Failed to open stdout");
    let mut reader = BufReader::new(stdout).lines();
    while let Some(line) = reader.next_line().await.unwrap() {
        if debug {
            println!("run_custom script outputted: {}", line);
        }
        let fn_call: FnCall = serde_json::from_str(&line).expect("unable to parse input");
        let res = call_fn(db, media, app_config, custom.version, fn_call).await?;
        if debug {
            println!("run_custom responded: {}", res);
        }
        if res != "null" {
            stdin.write_all(res.as_bytes()).await;
            stdin.write_all(b"\n").await;
        }
    }
    
    let out = child.wait_with_output().await.expect("unable to wait for child process");
    
    if !out.status.success() {
        return Err(TaskError::CustomTaskError((out.status,out.stderr)))
    }
    
    Ok(())
}
