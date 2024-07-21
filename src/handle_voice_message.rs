use std::error::Error;

use itertools::Itertools;
use sqlx::{Pool, Sqlite};
use teloxide::{payloads::SendMessageSetters, requests::Requester, types::ChatId, Bot};

use crate::{
    callback_handler::CallbackType,
    inline_data_keyboard::{InlineDataKeyboard, InlineDataKeyboardButton},
};

pub async fn handle_voice_message(
    bot: &Bot,
    db: &Pool<Sqlite>,
    chat_id: ChatId,
    voice_file_id: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let rooms = sqlx::query!("SELECT * FROM rooms").fetch_all(db).await?;
    if rooms.len() <= 0 {
        bot.send_message(chat_id, "No rooms were defined yet to play this in.")
            .await?;
        return Ok(());
    }

    let keyboard = InlineDataKeyboard::new().buttons(
        rooms
            .iter()
            .map(
                move |room| -> serde_json::Result<InlineDataKeyboardButton> {
                    Ok(InlineDataKeyboardButton {
                        text: room.name.to_owned(),
                        data: serde_json::to_string(&CallbackType::PlayAudio {
                            room_name: room.name.to_owned(),
                            voice_file_id: voice_file_id.to_owned(),
                        })?,
                    })
                },
            )
            .try_collect()?,
    );
    let keyboard_msg = bot
        .send_message(chat_id, "Where should I play this?")
        .reply_markup(keyboard.build_inline_keyboard_markup())
        .await?;
    keyboard.insert_into_db(&db, &keyboard_msg.id).await?;

    Ok(())
}
