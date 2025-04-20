use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use toml::map::Map;
use toml::Table;
use crate::media_processors::format::pdf::PdfConfig;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct AppConfig {
    pub scan_paths: Vec<String>,
    pub exclude_paths: Option<Vec<String>>,
    pub data_dir: String,
    pub db_path: String,

    pub listen_addr: String,
    pub client_user: String,
    pub client_group: String,
    pub socket_path: String,

    pub tasks: Table,
    
    pub formats: FormatConfig,

    pub python_path: String,
    pub ffmpeg_path: String,
    pub scripts_dir: String,

    #[serde(default)]
    pub remote: Table
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct FormatConfig {
    pub pdf: PdfConfig,
}

impl AppConfig {
    pub fn canonicalize(&mut self){
        self.data_dir = std::fs::canonicalize(&self.data_dir).unwrap().to_str().unwrap().to_string();
        self.db_path = std::fs::canonicalize(&self.db_path).unwrap().to_str().unwrap().to_string();
        
        for path in self.scan_paths.iter_mut() {
            *path =  std::fs::canonicalize(path.clone()).unwrap().to_str().unwrap().to_string();
        }

        if let Some(exclude) = self.exclude_paths.as_mut() {
            for path in exclude.iter_mut() {
                *path =  std::fs::canonicalize(path.clone()).unwrap().to_str().unwrap().to_string();
            }
        }
    }

    pub fn from_path<T: AsRef<Path>>(path: T) -> Self {
        let config = std::fs::read_to_string(path).unwrap();
        toml::from_str(&config).unwrap()
    }
    
    pub fn path_matches<T: AsRef<Path>>(&self, path: T) -> bool {
        let path = path.as_ref();
        
        if !self.scan_paths.iter().any(|x| path.starts_with(x)) {
            return false;
        }
        
        if let Some(exclude) = self.exclude_paths.as_ref() {
            if exclude.iter().any(|x| path.starts_with(x)) {
                return false
            }
        }
        
        true
        
    }
}
