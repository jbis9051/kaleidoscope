use serde::{Deserialize, Serialize};
use std::path::{Path};
use toml::Table;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RemoteRunnerConfig {
    pub db_path: String,
    pub data_dir: String,

    pub listen_addr: String,

    pub tasks: Table,
    
    pub python_path: String,
    pub ffmpeg_path: String,
    pub scripts_dir: String,

    pub password: Option<String>,

}

impl RemoteRunnerConfig {
    pub fn canonicalize(&mut self) {
        self.db_path = std::fs::canonicalize(&self.db_path).unwrap().to_str().unwrap().to_string();
    }

    pub fn from_path<T: AsRef<Path>>(path: T) -> Self {
        let config = std::fs::read_to_string(path).unwrap();
        toml::from_str(&config).unwrap()
    }
}
