use itertools::Itertools;
use sqlx::{Pool, Sqlite};
use std::error::Error;
use teloxide::{
    dispatching::DpHandlerDescription,
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup, MediaKind, MessageKind},
};

use crate::{
    callback_handler::CallbackType,
    command::Command,
    dialogues::{self},
};

async fn msg_endpoint(
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
