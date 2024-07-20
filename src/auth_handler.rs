use std::{error::Error, sync::Arc};

use sqlx::{Pool, Sqlite};
use teloxide::{
    dispatching::DpHandlerDescription,
    dptree::{self, Endpoint},
    prelude::DependencyMap,
    requests::Requester,
    types::{ChatId, MessageKind, Update, UpdateKind, UserId},
    Bot,
};

use crate::config::AppConfig;

pub fn make_auth_handler(
) -> Endpoint<'static, DependencyMap, Result<(), Box<dyn Error + Send + Sync>>, DpHandlerDescription>
{
    dptree::filter_async(
        |bot: Bot, update: Update, app_config: Arc<AppConfig>, db: Pool<Sqlite>| async move {
            let chat = match update.chat() {
                Some(chat) => chat,
                None => return false,
            };

            let user_id = match chat.id.is_user() {
                true => UserId(chat.id.0 as u64),
                false => return true,
            };

            if app_config.is_admin(&chat.id.0) {
                return false;
            }

            let res = sqlx::query!("SELECT id FROM auth_groups")
                .fetch_all(&db)
                .await;
            let auth_group_ids = match res {
                Ok(ids) => ids,
                Err(err) => {
                    log::error!("failed to read auth group ids: {}", err);
                    return true;
                }
            };
            let mut is_in_group = false;
            for auth_group_id in auth_group_ids {
                let res = bot.get_chat_member(ChatId(auth_group_id.id), user_id).await;
                let member = match res {
                    Ok(member) => member,
                    Err(err) => {
                        log::error!("failed to check chat member for auth: {}", err);
                        continue;
                    }
                };
                if member.is_present() {
                    is_in_group = true;
                }
            }
            return !is_in_group;
        },
    )
    .endpoint(|bot: Bot, update: Update| async move {
        match update.kind {
            UpdateKind::Message(msg) => match msg.kind {
                MessageKind::Common(_) => {
                    if msg.chat.id.is_user() {
                        bot.send_message(
                            msg.chat.id,
                            "Unauthorized. Please ask an admin to invite you to the group.",
                        )
                        .await?;
                    }
                    Ok(())
                }
                _ => Ok(()),
            },
            _ => Ok(()),
        }
    })
}
