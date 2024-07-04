mod ahm;
mod command;
mod config;
mod msg_handler;
mod player;

use std::{process::exit, sync::Arc};

use config::AppConfig;
use msg_handler::handle_message;
use player::Player;
use teloxide::prelude::*;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    log::info!("starting telegram voice relay bot.");

    let app_config = Arc::new(AppConfig::init().unwrap_or_else(|e| {
        log::error!("initialization phase failed: {}", e);
        exit(1)
    }));

    let bot = Bot::new(&app_config.env.bot_token);

    let player = Player::new(&app_config);

    Dispatcher::builder(
        bot,
        dptree::entry().branch(Update::filter_message().endpoint(handle_message)),
    )
    .dependencies(dptree::deps![app_config, Arc::new(Mutex::new(player))])
    .enable_ctrlc_handler()
    .build()
    .dispatch()
    .await;
}
