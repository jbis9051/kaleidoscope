use std::env;
use crate::scan_config::AppConfig;

pub struct EnvVar {
    pub config: Option<AppConfig>,
    pub dev_mode: bool,
    pub db_migrate: bool,
    pub migrate: bool,
}

impl EnvVar {
    pub fn from_env() -> Self {

        let config = std::env::var("CONFIG").ok().map(|config| serde_json::from_str(&config).unwrap());
        let dev_mode = std::env::var("dev_mode").ok().map(|dev_mode| dev_mode == "true").unwrap_or(false);
        let migrate = std::env::var("migrate").ok().map(|migrate| migrate == "true").unwrap_or(true);
        let db_migrate = std::env::var("db_migrate").ok().map(|migrate| migrate == "true").unwrap_or(false);

        Self {
            config,
            dev_mode,
            db_migrate,
            migrate,
        }
    }
}

pub fn setup_log(module: &str){
    let rust_log = env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());

    let filter = match rust_log.as_str() {
        "trace" => log::LevelFilter::Trace,
        "debug" => log::LevelFilter::Debug,
        "info" => log::LevelFilter::Info,
        "warn" => log::LevelFilter::Warn,
        "error" => log::LevelFilter::Error,
        _ => log::LevelFilter::Info,
    };

    env_logger::Builder::new()
        .filter_module(module, filter)
        .init();
}