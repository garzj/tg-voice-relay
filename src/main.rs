#![forbid(unsafe_code)]

mod ahm;
mod auth_handler;
mod callback_handler;
mod command;
mod config;
mod db;
mod dialogues;
mod msg_handler;
mod player;

use std::{process::exit, sync::Arc};

use auth_handler::make_auth_handler;
use callback_handler::make_callback_handler;
use config::AppConfig;
use msg_handler::make_msg_handler;
use player::Player;
use teloxide::prelude::*;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    log::info!("starting telegram voice relay bot.");

    let app_config = AppConfig::init().unwrap_or_else(|e| {
        log::error!("initialization phase failed: {}", e);
        exit(1)
    });

    let bot = Bot::new(&app_config.env.bot_token);

    let player = Player::new(&app_config);

    let db = db::init(&app_config).await;

    Dispatcher::builder(
        bot,
        dptree::entry()
            .branch(make_auth_handler())
            .branch(make_msg_handler())
            .branch(make_callback_handler()),
    )
    .dependencies(dptree::deps![
        Arc::new(app_config),
        Arc::new(Mutex::new(player)),
        db.clone()
    ])
    .enable_ctrlc_handler()
    .build()
    .dispatch()
    .await;

    db.close().await;
}
