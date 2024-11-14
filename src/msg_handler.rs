use sqlx::{Pool, Sqlite};
use std::{error::Error, sync::Arc};
use teloxide::{
    dispatching::DpHandlerDescription,
    prelude::*,
    types::{KeyboardRemove, MediaKind, MessageKind},
};

use crate::{
    command::Command,
    config::AppConfig,
    dialogues::{self},
    handle_replies::handle_replies,
    handle_voice_message::handle_voice_message,
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
                handle_voice_message(&bot, &db, msg.chat.id, &voice.voice.file.id).await?;
            }
            MediaKind::Audio(audio) => {
                handle_voice_message(&bot, &db, msg.chat.id, &audio.audio.file.id).await?;
            }
            MediaKind::Document(doc) => {
                handle_voice_message(&bot, &db, msg.chat.id, &doc.document.file.id).await?;
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
                    "Authorized all members of the group{}.\nRemove me from the group at anytime to revoke access.",
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

pub fn make_dot_reply_handler(
) -> Handler<'static, DependencyMap, Result<(), Box<dyn Error + Send + Sync>>, DpHandlerDescription>
{
    dptree::entry()
        .filter(|msg: Message| matches!(msg.text(), Some(".")))
        .endpoint(|bot: Bot, db: Pool<Sqlite>, msg: Message| async move {
            handle_replies(&bot, &db, &msg).await
        })
}

pub async fn make_msg_handler(
    app_config: &AppConfig,
) -> Handler<'static, DependencyMap, Result<(), Box<dyn Error + Send + Sync>>, DpHandlerDescription>
{
    Update::filter_message()
        .filter(|msg: Message| msg.chat.id.is_user())
        .chain(dialogues::set_room::make_inject_handler(app_config).await)
        .branch(make_dot_reply_handler())
        .branch(Command::make_handler())
        .branch(dialogues::set_room::make_endpoint_handler())
        .branch(dptree::endpoint(msg_endpoint))
}
