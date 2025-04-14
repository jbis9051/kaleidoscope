use std::error::Error;
use sqlx::SqliteExecutor;

const MIGRATION_VERSION_DB_KEY: &str = "migration_version";

pub trait Migration {
    const VERSION: u32;
    async fn up(&self) -> Result<(), Box<dyn Error>>;

}

pub struct MigrationManager;

impl MigrationManager {
    pub async fn current_version<'a>(db: impl SqliteExecutor<'a>) -> Result<u32, sqlx::Error> {
        let version = sqlx::query_scalar("SELECT value FROM kv WHERE key = ?")
            .bind(MIGRATION_VERSION_DB_KEY)
            .fetch_one(db)
            .await?;
        Ok(version)
    }
}