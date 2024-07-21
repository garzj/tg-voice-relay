use std::{error::Error, sync::Arc};

use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite};
use teloxide::{
    dispatching::{DpHandlerDescription, UpdateFilterExt},
    dptree::Endpoint,
    net::Download,
    payloads::EditMessageTextSetters,
    prelude::DependencyMap,
    requests::Requester,
    types::{CallbackQuery, InlineKeyboardMarkup, Update},
    Bot,
};
use tokio::{fs::File, sync::Mutex};

use crate::{
    config::AppConfig,
    inline_data_keyboard::{InlineDataKeyboard, InlineDataKeyboardButton},
    player::Player,
};

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

async fn callback_endpoint(
    app_config: Arc<AppConfig>,
    bot: Bot,
    db: Pool<Sqlite>,
    player: Arc<Mutex<Player>>,
    q: CallbackQuery,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let message = match q.message {
        None => {
            log::warn!("no message associated with callback query with id {}", q.id);
            return Ok(());
        }
        Some(chat_id) => chat_id,
    };
    let chat_id = message.chat.id;

    let query_data = match q.data {
        Some(ref query_data) => query_data,
        None => {
            log::warn!("no data on callback query with id {}", q.id);
            return Ok(());
        }
    };
    let button_index = match query_data.parse::<i64>() {
        Ok(button_index) => button_index,
        Err(_) => {
            log::warn!(
                "invalid data on callback query with id {}: {}",
                q.id,
                query_data
            );
            return Ok(());
        }
    };

    let button_data = sqlx::query!(
        "SELECT data FROM keyboard_buttons WHERE message_id = ? AND button_index = ?",
        message.id.0,
        button_index
    )
    .fetch_one(&db)
    .await?;

    let edit_query_message = |text: String, reply_markup: Option<InlineKeyboardMarkup>| async {
        let mut edit_msg = bot.edit_message_text(chat_id, message.id, text);
        if let Some(markup) = reply_markup {
            edit_msg = edit_msg.reply_markup(markup)
        }
        edit_msg.await?;

        InlineDataKeyboard::remove_from_db(&db, &message.id).await?;

        Result::<(), Box<dyn Error + Send + Sync>>::Ok(())
    };

    let cb_type: CallbackType = match serde_json::from_str(&button_data.data) {
        Err(err) => {
            edit_query_message(
                format!("Received invalid callback data. Please try this again."),
                None,
            )
            .await?;
            log::error!(
                "invalid callback data for button {} on message with id {}: {}",
                button_index,
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
                edit_query_message("Insufficient permission.".into(), None).await?;
                return Ok(());
            }

            sqlx::query!("DELETE FROM rooms WHERE name = ?", name)
                .execute(&db)
                .await?;
            edit_query_message(format!("Deleted room {}.", name), None).await?;
        }
        CallbackType::PlayAudio {
            room_name,
            voice_file_id,
        } => {
            let player = match player.try_lock() {
                Err(_) => {
                    edit_query_message("Another audio is being played.".into(), None).await?;
                    return Ok(());
                }
                Ok(player) => player,
            };

            let stop_keyboard = InlineDataKeyboard::new().buttons(vec![InlineDataKeyboardButton {
                text: "Stop".into(),
                data: serde_json::to_string(&CallbackType::StopAudio { id: "todo".into() })?,
            }]);
            edit_query_message(
                format!("Playing audio in: {}", room_name),
                Some(stop_keyboard.build_inline_keyboard_markup()),
            )
            .await?;
            stop_keyboard.insert_into_db(&db, &message.id).await?;

            let preset = sqlx::query!("SELECT preset FROM rooms WHERE name = ?", room_name)
                .fetch_one(&db)
                .await?
                .preset as u16;
            if let Err(err) = player.set_channel(preset).await {
                log::error!("failed to switch channels: {}", err);
                edit_query_message(
                    "Failed to switch channels, the mixer ain't responding :/ Please try this again later.".into(),
                    None
                )
                .await?;
                return Ok(());
            }

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

            edit_query_message(format!("Played audio in: {}", room_name), None).await?;
            InlineDataKeyboard::remove_from_db(&db, &message.id).await?;
        }
        CallbackType::StopAudio { id: _ } => {
            todo!()
        }
    }

    Ok(())
}

pub fn make_callback_handler(
) -> Endpoint<'static, DependencyMap, Result<(), Box<dyn Error + Send + Sync>>, DpHandlerDescription>
{
    Update::filter_callback_query().endpoint(callback_endpoint)
}
