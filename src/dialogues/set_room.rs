use std::{error::Error, process::exit, sync::Arc};

use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite};
use teloxide::{
    dispatching::{
        dialogue::{serializer::Json, ErasedStorage, SqliteStorage, Storage},
        DpHandlerDescription, HandlerExt,
    },
    dptree::{self, Handler},
    prelude::{DependencyMap, Dialogue},
    requests::Requester,
    types::Message,
    Bot,
};

use crate::config::AppConfig;

#[derive(Clone, Default, Serialize, Deserialize)]
pub enum State {
    #[default]
    Inactive,
    ReceiveRoomName,
    ReceivePresetNumber {
        name: String,
    },
}

type MyStorage = std::sync::Arc<ErasedStorage<State>>;

pub type DialogueDependency = Dialogue<State, ErasedStorage<State>>;

pub async fn make_inject_handler(
    app_config: &AppConfig,
) -> Handler<'static, DependencyMap, Result<(), Box<dyn Error + Send + Sync>>, DpHandlerDescription>
{
    let db_file = &app_config.db_file.to_str().unwrap_or_else(|| {
        log::error!("invalid db file path: {:?}", app_config.db_file);
        exit(1)
    });
    let storage: MyStorage = SqliteStorage::open(db_file, Json)
        .await
        .unwrap_or_else(|e| {
            log::error!("db connection for storage failed: {}", e);
            exit(1)
        })
        .erase();

    dptree::map(move || storage.clone()).enter_dialogue::<Message, ErasedStorage<State>, State>()
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
