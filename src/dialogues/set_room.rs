use std::{error::Error, sync::Arc};

use sqlx::{Pool, Sqlite};
use teloxide::{
    dispatching::{
        dialogue::{self, InMemStorage},
        DpHandlerDescription, HandlerExt,
    },
    dptree::{self, Handler},
    prelude::{DependencyMap, Dialogue},
    requests::Requester,
    types::Message,
    Bot,
};

use crate::config::AppConfig;

#[derive(Clone, Default)]
pub enum State {
    #[default]
    Inactive,
    ReceiveRoomName,
    ReceivePresetNumber {
        name: String,
    },
}

pub type DialogueDependency = Dialogue<State, InMemStorage<State>>;

pub fn make_inject_handler(
) -> Handler<'static, DependencyMap, Result<(), Box<dyn Error + Send + Sync>>, DpHandlerDescription>
{
    dptree::map({
        let storage = InMemStorage::<State>::new();
        move || storage.clone()
    })
    .enter_dialogue::<Message, InMemStorage<State>, State>() // todo: change to sqlite storage
}

pub fn make_endpoint_handler(
) -> Handler<'static, DependencyMap, Result<(), Box<dyn Error + Send + Sync>>, DpHandlerDescription>
{
    dptree::entry()
        .filter_async(
            |app_config: Arc<AppConfig>, msg: Message, dialogue: DialogueDependency| async move {
                if app_config.is_admin(&msg.chat.id.0) {
                    true
                } else {
                    if let Err(err) = dialogue.reset().await {
                        log::error!("failed to reset set room dialogue: {}", err);
                    }
                    false
                }
            },
        )
        .branch(dptree::case![State::ReceiveRoomName].endpoint(
            |bot: Bot, msg: Message, dialogue: DialogueDependency| async move {
                let text = match msg.text() {
                    Some(text) => text,
                    None => {
                        bot.send_message(msg.chat.id, "Please send me a valid room name.")
                            .await?;
                        return Ok(());
                    }
                };

                let text = text.trim();
                if text.is_empty() {
                    bot.send_message(msg.chat.id, "The name cannot be empty.")
                        .await?;
                    return Ok(());
                }

                bot.send_message(msg.chat.id, "Now please enter a preset number.")
                    .await?;
                dialogue
                    .update(State::ReceivePresetNumber {
                        name: text.to_owned(),
                    })
                    .await?;
                Ok(())
            },
        ))
        .branch(dptree::case![State::ReceivePresetNumber { name }].endpoint(
            |bot: Bot,
             msg: Message,
             db: Pool<Sqlite>,
             dialogue: DialogueDependency,
             name: String| async move {
                let text = match msg.text() {
                    Some(text) => text,
                    None => {
                        bot.send_message(msg.chat.id, "Please send me a valid preset number.")
                            .await?;
                        return Ok(());
                    }
                };

                let preset = match text.parse::<i64>() {
                    Ok(text) => text,
                    Err(_) => {
                        bot.send_message(msg.chat.id, "The preset has to be a number.")
                            .await?;
                        return Ok(());
                    }
                };

                if preset < 1 || preset > 500 {
                    bot.send_message(
                        msg.chat.id,
                        "The preset should be a number between 1 and 500.",
                    )
                    .await?;
                    return Ok(());
                }

                let res = sqlx::query!(
                    "INSERT INTO rooms VALUES($1, $2)
                        ON CONFLICT(name) DO UPDATE SET preset=$2",
                    name,
                    preset
                )
                .execute(&db)
                .await;
                if let Err(e) = res {
                    bot.send_message(msg.chat.id, format!("Failed to set room preset: {}", e))
                        .await?;
                    return Ok(());
                }

                bot.send_message(
                    msg.chat.id,
                    format!("Linked room {} to preset {}.", name, preset),
                )
                .await?;

                dialogue.reset().await?;
                Ok(())
            },
        ))
}
