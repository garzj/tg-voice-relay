use std::{error::Error, sync::Arc};

use teloxide::{
    dispatching::DpHandlerDescription,
    dptree::{self, Endpoint},
    prelude::DependencyMap,
    requests::Requester,
    types::Update,
    Bot,
};

use crate::config::AppConfig;

pub fn make_auth_handler(
) -> Endpoint<'static, DependencyMap, Result<(), Box<dyn Error + Send + Sync>>, DpHandlerDescription>
{
    dptree::filter_async(
        |_bot: Bot, update: Update, app_config: Arc<AppConfig>| async move {
            if let Some(chat) = update.chat() {
                // todo: check if user in authed group
                // bot.get_chat_member();

                if app_config
                    .env
                    .admin_users
                    .iter()
                    .any(|admin_id| chat.id.0 == *admin_id)
                {
                    return false;
                }
            }
            true
        },
    )
    .endpoint(|bot: Bot, update: Update| async move {
        if let Some(chat) = update.chat() {
            bot.send_message(
                chat.id,
                "Unauthorized. Please ask an admin to invite you to the group.",
            )
            .await?;
        }
        Ok(())
    })
}
