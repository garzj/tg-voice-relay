use std::{error::Error, sync::Arc, time::Duration};

use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite};
use teloxide::{
    dispatching::{dialogue::GetChatId, DpHandlerDescription, UpdateFilterExt},
    dptree::Endpoint,
    net::Download,
    payloads::{EditMessageTextInlineSetters, EditMessageTextSetters},
    prelude::DependencyMap,
    requests::{Requester, ResponseResult},
    types::{CallbackQuery, InlineKeyboardButton, InlineKeyboardMarkup, Message, Update},
    Bot,
};
use tokio::{fs::File, sync::Mutex};

use crate::{config::AppConfig, player::Player};

#[derive(Serialize, Deserialize)]
pub enum CallbackType {
    StopAudio {
        id: String,
    },
    PlayAudio {
        room_name: String,
        voice_file_id: String,
    },
    RoomDel {
        name: String,
    },
}

async fn edit_query_message(
    bot: &Bot,
    q: &CallbackQuery,
    text: &str,
    reply_markup: Option<InlineKeyboardMarkup>,
) -> ResponseResult<()> {
    if let Some(Message { id, chat, .. }) = &q.message {
        let mut edit_msg = bot.edit_message_text(chat.id, *id, text);
        if let Some(markup) = reply_markup {
            edit_msg = edit_msg.reply_markup(markup)
        }
        edit_msg.await?;
    } else if let Some(id) = &q.inline_message_id {
        let mut edit_msg = bot.edit_message_text_inline(id, text);
        if let Some(markup) = reply_markup {
            edit_msg = edit_msg.reply_markup(markup);
        }
        edit_msg.await?;
    }

    Ok(())
}

async fn callback_endpoint(
    app_config: Arc<AppConfig>,
    bot: Bot,
    db: Pool<Sqlite>,
    player: Arc<Mutex<Player>>,
    q: CallbackQuery,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    if let Some(ref data) = q.data {
        bot.answer_callback_query(&q.id).await?;

        let chat_id = match q.chat_id() {
            None => {
                log::error!("no chat id on callback query with id {}", q.id);
                return Ok(());
            }
            Some(chat_id) => chat_id,
        };

        let cb_type: CallbackType = match serde_json::from_str(data) {
            Err(err) => {
                edit_query_message(
                    &bot,
                    &q,
                    &format!("Received invalid callback data. Please try this again."),
                    None,
                )
                .await?;
                log::warn!(
                    "invalid callback data for message with id {}: {}",
                    &q.id,
                    err
                );
                return Ok(());
            }
            Ok(cb_type) => cb_type,
        };
        match cb_type {
            CallbackType::RoomDel { name } => {
                if !app_config.is_admin(&chat_id.0) {
                    edit_query_message(&bot, &q, "Insufficient permission.", None).await?;
                    return Ok(());
                }

                sqlx::query!("DELETE FROM rooms WHERE name = ?", name)
                    .execute(&db)
                    .await?;
                edit_query_message(&bot, &q, &format!("Deleted room {}.", name), None).await?;
            }
            CallbackType::PlayAudio {
                room_name,
                voice_file_id,
            } => {
                let player = match player.try_lock() {
                    Err(_) => {
                        edit_query_message(&bot, &q, "Another audio is being played.", None)
                            .await?;
                        return Ok(());
                    }
                    Ok(player) => player,
                };

                let stop_player_keyboard = vec![vec![InlineKeyboardButton::callback(
                    "Stop",
                    serde_json::to_string(&CallbackType::StopAudio { id: "todo".into() })?,
                )]];
                edit_query_message(
                    &bot,
                    &q,
                    &format!("Playing audio in: {}", room_name),
                    Some(InlineKeyboardMarkup::new(stop_player_keyboard)),
                )
                .await?;

                let preset = sqlx::query!("SELECT preset FROM rooms WHERE name = ?", room_name)
                    .fetch_one(&db)
                    .await?
                    .preset as u16;
                player.set_channel(preset).await?;

                let file = bot.get_file(&voice_file_id).await?;
                let name = file
                    .path
                    .split("/")
                    .last()
                    .ok_or("failed to get voice file name")?;
                let dst_path = app_config.audio_dir.join(name);

                match File::create_new(&dst_path).await {
                    Ok(mut dst) => {
                        bot.download_file(&file.path, &mut dst).await?;
                        dst.sync_all().await?;
                    }
                    Err(err) => match err.kind() {
                        std::io::ErrorKind::AlreadyExists => {}
                        _ => return Err(Box::new(err)),
                    },
                };

                let audio_path = dst_path
                    .to_str()
                    .ok_or("failed to construct voice file path")?;

                player.play_audio_file(&audio_path).await?;

                tokio::time::sleep(Duration::from_secs(3)).await;

                edit_query_message(&bot, &q, &format!("Played audio in: {}", room_name), None)
                    .await?;
            }
            CallbackType::StopAudio { id } => {
                todo!()
            }
        }
    }

    Ok(())
}

pub fn make_callback_handler(
) -> Endpoint<'static, DependencyMap, Result<(), Box<dyn Error + Send + Sync>>, DpHandlerDescription>
{
    Update::filter_callback_query().endpoint(callback_endpoint)
}
