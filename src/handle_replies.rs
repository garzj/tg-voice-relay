use std::error::Error;

use sqlx::{Pool, Sqlite};
use teloxide::{prelude::Requester, types::Message, Bot};

use crate::handle_voice_message::handle_voice_message;

pub async fn handle_replies(
    bot: &Bot,
    db: &Pool<Sqlite>,
    msg: &Message,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let reply_msg = match msg.reply_to_message() {
        None => {
            bot.send_message( msg.chat.id,
                            "This command may only be used in reply to an older voice message. Use /help for more information.",
                        )
                        .await?;
            return Ok(());
        }
        Some(reply_msg) => reply_msg,
    };

    let voice = match reply_msg.voice() {
        Some(voice) => voice,
        None => {
            bot.send_message(
                msg.chat.id,
                "The mentioned message has to be a voice message or an audio file.",
            )
            .await?;
            return Ok(());
        }
    };

    handle_voice_message(&bot, &db, msg.chat.id, &voice.file.id).await?;

    Ok(())
}
