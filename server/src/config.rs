use once_cell::sync::Lazy;
use std::env;

#[derive(Debug)]
pub struct Config {
    pub listen_addr: String,
    pub db_path: String,
}

pub static CONFIG: Lazy<Config> = Lazy::new(|| Config {
    listen_addr: env::var("LISTEN_ADDR").unwrap_or_default(),
    db_path: env::var("DATABASE_URL").unwrap(),
});
