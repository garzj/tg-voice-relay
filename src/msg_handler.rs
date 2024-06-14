use std::{error::Error, sync::Arc};
use teloxide::{
    net::Download,
    prelude::*,
    types::{MediaKind, MessageKind},
    utils::command::BotCommands,
};
use tokio::fs::File;

use crate::{command::Command, config::AppConfig, player::play_audio_file};

pub async fn handle_message(
    bot: Bot,
    msg: Message,
    app_config: Arc<AppConfig>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let chat_id = msg.chat.id.0;

    // telegram uses negative numbers for groups' `chat_id`
    if chat_id < 0 {
        return Ok(());
    }

    let response = String::from("");
    if let Some(cmd) = msg.text().and_then(|text| Command::parse(text, "").ok()) {
        cmd.execute(&bot, &msg).await?;
    } else {
        match &msg.kind {
            MessageKind::Common(common_msg) => match &common_msg.media_kind {
                MediaKind::Voice(voice) => {
                    let file = bot.get_file(&voice.voice.file.id).await?;
                    let name = file
                        .path
                        .split("/")
                        .last()
                        .ok_or("failed to get voice file name")?;
                    let dst_path = app_config.audio_dir.join(name);
                    let mut dst = File::create(&dst_path).await?;
                    bot.download_file(&file.path, &mut dst).await?;
                    dst.sync_all().await?;

                    play_audio_file(
                        &dst_path
                            .to_str()
                            .ok_or("failed to construct voice file path")?,
                        &app_config.env.player_command,
                    )
                    .await?;
                }
                _ => {
                    bot.send_message(msg.chat.id, "Send me a voice message or use /help.")
                        .await?;
                }
            },
            _ => {}
        }
    }

    if !response.is_empty() {
        bot.send_message(msg.chat.id, response).await?;
    }

    Ok(())
}