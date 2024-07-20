use itertools::Itertools;
use sqlx::{Pool, Sqlite};
use std::{error::Error, sync::Arc};
use teloxide::{
    dispatching::DpHandlerDescription,
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup, KeyboardRemove, MediaKind, MessageKind},
};

use crate::{
    callback_handler::CallbackType,
    command::Command,
    config::AppConfig,
    dialogues::{self},
};

async fn msg_endpoint(
    app_config: Arc<AppConfig>,
    bot: Bot,
    db: Pool<Sqlite>,
    msg: Message,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    match &msg.kind {
        MessageKind::Common(common_msg) => match &common_msg.media_kind {
            MediaKind::Voice(voice) => {
                let file_id = &voice.voice.file.id;

                let rooms = sqlx::query!("SELECT * FROM rooms").fetch_all(&db).await?;
                let keyboard: Vec<Vec<InlineKeyboardButton>> = rooms
                    .chunks(3)
                    .map(|row| {
                        row.iter()
                            .map(|room| -> serde_json::Result<InlineKeyboardButton> {
                                Ok(InlineKeyboardButton::callback(
                                    &room.name,
                                    serde_json::to_string(&CallbackType::PlayAudio {
                                        room_name: room.name.to_owned(),
                                        voice_file_id: file_id.to_owned(),
                                    })?,
                                ))
                            })
                            .try_collect()
                    })
                    .try_collect()?;

                bot.send_message(msg.chat.id, "Where should I play this?")
                    .reply_markup(InlineKeyboardMarkup::new(keyboard))
                    .await?;
            }
            _ => {
                bot.send_message(msg.chat.id, "Send me a voice message or use /help.")
                    .await?;
            }
        },
        MessageKind::ChatShared(chat_shared_msg) => {
            if !app_config.is_admin(&msg.chat.id.0) {
                bot.send_message(msg.chat.id, "Insufficient permission.")
                    .await?;
                return Ok(());
            }

            let chat_shared = &chat_shared_msg.chat_shared;

            // ? make sure https://core.telegram.org/bots/api#getchatmember works
            // let chat_member = bot
            //     .get_chat_member(chat_shared.chat_id, bot.get_me().await?.id)
            //     .await?;
            // if !chat_member.is_administrator() {
            //     bot.send_message(msg.chat.id, "Please make me a group administrator first.")
            //         .reply_markup(KeyboardRemove::new())
            //         .await?;
            //     return Ok(());
            // }

            let auth_group_ids = sqlx::query!("SELECT id FROM auth_groups")
                .fetch_all(&db)
                .await?;
            if auth_group_ids
                .iter()
                .any(|group_id| group_id.id == chat_shared.chat_id.0)
            {
                bot.send_message(
                    msg.chat.id,
                    "This group is already authorized with its members.",
                )
                .reply_markup(KeyboardRemove::new())
                .await?;
                return Ok(());
            }

            let chat_details = bot.get_chat(chat_shared.chat_id).await?;
            sqlx::query!(
                "INSERT INTO auth_groups (id) VALUES (?)",
                chat_shared.chat_id.0
            )
            .execute(&db)
            .await?;
            bot.send_message(
                msg.chat.id,
                &format!(
                    "Authorized all members of the group{}.",
                    if let Some(title) = chat_details.title() {
                        format!(" \"{}\"", title)
                    } else {
                        "".into()
                    }
                ),
            )
            .reply_markup(KeyboardRemove::new())
            .await?;
        }
        _ => {}
    }

    Ok(())
}

pub fn make_msg_handler(
) -> Handler<'static, DependencyMap, Result<(), Box<dyn Error + Send + Sync>>, DpHandlerDescription>
{
    Update::filter_message()
        .filter(|msg: Message| msg.chat.id.is_user())
        .chain(dialogues::set_room::make_inject_handler())
        .branch(Command::make_handler())
        .branch(dialogues::set_room::make_endpoint_handler())
        .branch(dptree::endpoint(msg_endpoint))
}
