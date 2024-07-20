use std::process::exit;

use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    Pool, Sqlite,
};

use crate::config::AppConfig;

async fn init_tables(db: &Pool<Sqlite>) -> sqlx::Result<()> {
    sqlx::query!(
        "
CREATE TABLE IF NOT EXISTS rooms (
    name VARCHAR(20) NOT NULL PRIMARY KEY,
    preset INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS auth_groups (
    id INTEGER NOT NULL PRIMARY KEY
);
        "
    )
    .execute(db)
    .await?;

    Ok(())
}

pub async fn init(app_config: &AppConfig) -> Pool<Sqlite> {
    let db_file = &app_config.db_file.to_str().unwrap_or_else(|| {
        log::error!("invalid db file path: {:?}", app_config.db_file);
        exit(1)
    });
    let options = SqliteConnectOptions::new()
        .filename(db_file)
        .create_if_missing(true);
    let db = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .unwrap_or_else(|e| {
            log::error!("db connection failed: {}", e);
            exit(1)
        });

    init_tables(&db)
        .await
        .unwrap_or_else(|e| log::error!("failed to initialize database tables: {}", e));

    db
}
