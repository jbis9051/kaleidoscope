use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppConfig {
    pub scan_paths: Vec<String>,
    pub data_dir: String,
    pub thumb_size: u32,
    pub db_path: String,

    pub listen_addr: String,
    pub client_user: String,
    pub client_group: String,
    pub socket_path: String,
}

impl AppConfig {
    pub fn from_path<T: AsRef<Path>>(path: T) -> Self {
        let config = std::fs::read_to_string(path).unwrap();
        toml::from_str(&config).unwrap()
    }
}
