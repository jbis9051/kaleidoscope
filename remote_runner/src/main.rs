use axum::http::{header, HeaderMap, Method, StatusCode};
use axum::{middleware, Extension, Json, Router};
use axum::extract::{Path, Request};
use axum::middleware::Next;
use axum::response::{ErrorResponse, IntoResponse, Response};
use axum::routing::{get, post};
use once_cell::sync::Lazy;
use sqlx::{SqlitePool};
use sqlx::types::Uuid;
use subtle::ConstantTimeEq;
use common::env::EnvVar;
use common::remote_models::job::{Job, JobStatus};
use common::types::DbPool;
use common::runner_config::RemoteRunnerConfig;
use tasks::remote_utils::RemoteTaskStatus;
use tasks::tasks::Task;

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

static PASS: Lazy<Option<Vec<u8>>> = Lazy::new(|| {
    if let Some(pass) = CONFIG.password.as_ref() {
        let correct = format!("Bearer {}", pass);
        // we could use a proper password hash but there's no real reason to
        return Some(blake3::hash(correct.as_bytes()).as_bytes().to_vec());
    }
    return None
});

#[tokio::main]
async fn main() {
    if ENV.dev_mode {
        println!("Running in dev mode");
    }

    println!("Config: {:?}", &CONFIG);

    // execute the lazy function
    let _ = *PASS;

    let pool = SqlitePool::connect(&format!("sqlite://{}", CONFIG.db_path)).await.unwrap();

    if ENV.db_migrate {
        println!("Migrating database");
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("Failed to migrate database");
        println!("Migration complete");
    }
    // we need to cancel all jobs

    let cancelled = Job::cancel_all(&pool, "server restarted").await.expect("Failed to cancel all jobs");

    if cancelled > 0 {
        println!("Cancelled {} jobs", cancelled);
    }

    println!("Remote Runner Listening on: {}", &CONFIG.listen_addr);

    let app = Router::new()
        .route("/status", get(status))
        .route("/task/{task_name}", post(task_run))
        .route("/job/{job_uuid}", get(job_status))
        .layer(Extension(pool))
        .layer(middleware::from_fn(auth_middleware));

    let listener = tokio::net::TcpListener::bind(&CONFIG.listen_addr).await.unwrap();

    axum::serve(listener, app).await.unwrap();
}

async fn auth_middleware(headers: HeaderMap, request: Request, next: Next) -> Result<Response, (StatusCode, String)> {
    if let Some(password) = &*PASS {
        let authorization = headers.get("authorization").ok_or((StatusCode::FORBIDDEN, "missing 'authorization' header".to_string()))?;
        let authorization = blake3::hash(authorization.as_bytes());
        let attempt = authorization.as_bytes();
        // timing safe comparison
        let res = password.ct_eq(attempt);

        if res.unwrap_u8() != 1 {
            return Err((StatusCode::FORBIDDEN, "bad authentication (invalid password)".to_string()))
        }
    }
    Ok(next.run(request).await.into_response())
}


async fn status(Extension(pool): Extension<DbPool>) -> Json<RemoteTaskStatus> {
    let running = Job::get_by_status(&pool, &JobStatus::Running)
        .await
        .unwrap();

    if running.len() > 1 {
        panic!("only one job should be running {:?}", running);
    }

    if running.len() == 0 {
        return Json(RemoteTaskStatus::Ready)
    }

    Json(RemoteTaskStatus::Busy(running[0].clone()))
}

async fn job_status(Extension(pool): Extension<DbPool>, Path(job_uuid): Path<Uuid>) -> Result<Json<Job>, (StatusCode, String)> {
    let job = Job::try_from_uuid(&pool, &job_uuid).await.unwrap().ok_or((StatusCode::NOT_FOUND, "job not found with that uuid".to_string()))?;

    // once the client has checked the job status, we delete the job from the database
    if job.status != JobStatus::Running {
        job.delete(&pool).await.unwrap();
    }

    Ok(Json(job))
}

async fn task_run(
    Extension(pool): Extension<DbPool>,
    Path(task_name): Path<String>,
    request: Request,
) -> Result<Response, ErrorResponse> {
    let running = Job::get_by_status(&pool, &JobStatus::Running)
        .await
        .unwrap();
    if running.len() > 0 {
        return Err((StatusCode::CONFLICT, format!("runner is busy with job(s): {:?}", running)).into());
    }

    let _ = CONFIG.tasks.get(&task_name).ok_or((StatusCode::BAD_REQUEST, "task unsupported".to_string()))?;
    
    if !Task::remotable(&task_name) {
        panic!("task is enabled for remote but isn't remotable");
    }
    
    let remote_task = Task::new_remote(&task_name, &mut &pool, &CONFIG.tasks, &CONFIG).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    remote_task.remote_handler(request, pool, &CONFIG.tasks, &CONFIG).await
}