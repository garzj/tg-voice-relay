#![forbid(unsafe_code)]

mod ahm;
mod auth_handler;
mod callback_handler;
mod command;
mod config;
mod db;
mod dialogues;
mod handle_voice_message;
mod inline_data_keyboard;
mod msg_handler;
mod my_chat_member_handler;
mod player;

use std::{process::exit, sync::Arc};

use auth_handler::make_auth_handler;
use callback_handler::make_callback_handler;
use config::AppConfig;
use msg_handler::make_msg_handler;
use my_chat_member_handler::make_my_chat_member_handler;
use player::Player;
use teloxide::prelude::*;

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
            .branch(make_my_chat_member_handler())
            .branch(make_auth_handler())
            .branch(make_msg_handler())
            .branch(make_callback_handler()),
    )
    .dependencies(dptree::deps![
        Arc::new(app_config),
        Arc::new(player),
        db.clone()
    ])
    .distribution_function(|_| None::<()>)
    .enable_ctrlc_handler()
    .build()
    .dispatch()
    .await;

    db.close().await;
}
