use axum::http::{header, Method, StatusCode};
use axum::{Extension, Router};
use axum::extract::{Path, Request};
use axum::response::{Response};
use axum::routing::{get, post};
use once_cell::sync::Lazy;
use sqlx::SqlitePool;
use common::env::EnvVar;
use common::types::DbPool;
use common::runner_config::RemoteRunnerConfig;
use remote_runner::RemoteTask;

static ENV: Lazy<EnvVar> = Lazy::new(|| {
    let env = EnvVar::from_env();
    env
});
static CONFIG: Lazy<RemoteRunnerConfig> = Lazy::new(|| {
    let config_path = std::env::args().nth(1).expect("No config file provided");

    let path = std::path::Path::new(&config_path);
    if !path.exists() {
        panic!("config file does not exist: {}", config_path);
    }
   
    let mut config = RemoteRunnerConfig::from_path(&config_path);
    config.canonicalize();
    config
});

#[tokio::main]
async fn main() {
    if ENV.dev_mode {
        println!("Running in dev mode");
    }

    println!("Config: {:?}", &CONFIG);

    let pool = SqlitePool::connect(&format!("sqlite://{}", CONFIG.db_path)).await.unwrap();

    if ENV.db_migrate {
        println!("Migrating database");
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("Failed to migrate database");
        println!("Migration complete");
    }

    println!("Remote Runner Listening on: {}", &CONFIG.listen_addr);
    
    let app = Router::new()
        .route("/task/{task_name}", get(task_run).post(task_run))
        .layer(Extension(pool));

    let listener = tokio::net::TcpListener::bind(&CONFIG.listen_addr).await.unwrap();

    axum::serve(listener, app).await.unwrap();
}

async fn task_run(
    Extension(pool): Extension<DbPool>,
    Path(task_name): Path<String>,
    request: Request,
) -> Result<Response, (axum::http::StatusCode, String)> {
    let _ = CONFIG.tasks.get(&task_name).ok_or((StatusCode::BAD_REQUEST, "task unsupported".to_string()))?;
    
    let remote_task = RemoteTask::new(&task_name, &mut &pool, &CONFIG.tasks, &CONFIG).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(remote_task.remote_handler(request, &mut &pool, &CONFIG.tasks, &CONFIG).await.map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?)
}