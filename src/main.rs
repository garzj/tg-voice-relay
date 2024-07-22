#![forbid(unsafe_code)]

mod ahm;
mod auth_handler;
mod backoff;
mod callback_handler;
mod command;
mod config;
mod db;
mod dialogues;
mod handle_voice_message;
mod heartbeat;
mod inline_data_keyboard;
mod msg_handler;
mod my_chat_member_handler;
mod player;

use std::{process::exit, sync::Arc, time::Duration};

use auth_handler::make_auth_handler;
use callback_handler::make_callback_handler;
use config::AppConfig;
use heartbeat::Heartbeat;
use msg_handler::make_msg_handler;
use my_chat_member_handler::make_my_chat_member_handler;
use player::Player;
use teloxide::prelude::*;

const ENV_LOGGER_VAR: &str = "TG_VOICE_RELAY_LOG";

#[tokio::main]
async fn main() {
    if std::env::var(ENV_LOGGER_VAR).is_err() {
        std::env::set_var(ENV_LOGGER_VAR, "info");
    }
    pretty_env_logger::init_custom_env(ENV_LOGGER_VAR);
    log::info!("starting telegram voice relay bot.");

    let app_config = AppConfig::init().unwrap_or_else(|e| {
        log::error!("initialization phase failed: {}", e);
        exit(1)
    });

    if let Some(endpoint) = &app_config.env.heartbeat_endpoint {
        let heartbeat = Heartbeat::new(
            endpoint.to_owned(),
            Duration::from_millis(app_config.env.heartbeat_interval),
        );
        tokio::spawn(heartbeat.task());
    }

    let bot = Bot::new(&app_config.env.bot_token);

    let player = Player::new(&app_config);

    let db = db::init(&app_config).await;

    Dispatcher::builder(
        bot,
        dptree::entry()
            .branch(make_my_chat_member_handler())
            .branch(make_auth_handler())
            .branch(make_msg_handler(&app_config).await)
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
