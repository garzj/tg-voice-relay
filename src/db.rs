use std::process::exit;

use sqlx::{
    migrate,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    Pool, Sqlite,
};

use crate::config::AppConfig;

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

    migrate!("./migrations").run(&db).await.unwrap_or_else(|e| {
        log::error!("failed to initialize database tables: {}", e);
        exit(1);
    });

    db
}
