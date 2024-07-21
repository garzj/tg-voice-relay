use std::{error::Error, sync::Arc};

use itertools::Itertools;
use sqlx::{Pool, Sqlite};
use teloxide::{
    dispatching::DpHandlerDescription,
    prelude::*,
    types::{ButtonRequest, KeyboardButton, KeyboardButtonRequestChat, KeyboardMarkup},
    utils::command::BotCommands,
};

use crate::{
    callback_handler::CallbackType,
    config::AppConfig,
    dialogues,
    handle_voice_message::handle_voice_message,
    inline_data_keyboard::{InlineDataKeyboard, InlineDataKeyboardButton},
    player::Player,
};

#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "snake_case",
    description = "To play an announcement, you can:

- send me a voice message
- send me an audio file
- reply to an older message with /play

Additionally these commands may be used:"
)]
pub enum Command {
    #[command(description = "off")]
    Start,
    #[command(description = "show this list")]
    Help,
    #[command(description = "play the mentioned audio message")]
    Play,
    #[command(description = "stop the currently playing audio")]
    Stop,
    #[command(description = "list all rooms")]
    Rooms,
    #[command(description = "link a room to a preset")]
    RoomSet,
    #[command(description = "delete a room")]
    RoomDel,
    #[command(description = "link a group of authorized users")]
    GroupLink,
}

impl Command {
    pub async fn execute(
        self,
        app_config: Arc<AppConfig>,
        bot: Bot,
        db: Pool<Sqlite>,
        player: Arc<Player>,
        msg: Message,
        set_room_dialogue: dialogues::set_room::DialogueDependency,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        set_room_dialogue.reset().await?;

        match self {
            Command::Start => {
                bot.send_message(msg.chat.id, "Hello there,\n\nSend me a voice message and I'll announce it for you!\n\nUse /help for more information.").await?;
            }
            Command::Play => {
                let reply_msg = match msg.reply_to_message() {
                    None => {
                        bot.send_message(
                            msg.chat.id,
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
            }
            Command::Stop => match player.stop_playing().await {
                Err(err) => match err {
                    crate::player::StopAudioError::AlreadyStopped => {
                        bot.send_message(msg.chat.id, "No audio is being played at this time.")
                            .await?;
                    }
                },
                Ok(()) => {
                    bot.send_message(msg.chat.id, "Stopped playing the current audio.")
                        .await?;
                }
            },
            Command::Help => {
                bot.send_message(msg.chat.id, Command::descriptions().to_string())
                    .await?;
            }
            Command::Rooms => {
                let rooms = sqlx::query!("SELECT * FROM rooms").fetch_all(&db).await?;
                let room_list = if rooms.len() > 0 {
                    "Rooms and presets:\n".to_owned()
                        + &rooms
                            .iter()
                            .map(|room| format!("{} ↦ {}", room.name, room.preset))
                            .join("\n")
                } else {
                    "No rooms defined. Use /room_set to create one.".into()
                };
                bot.send_message(msg.chat.id, room_list).await?;
            }
            Command::RoomSet => {
                if !app_config.is_admin(&msg.chat.id.0) {
                    bot.send_message(msg.chat.id, "Insufficient permission.")
                        .await?;
                    return Ok(());
                }

                bot.send_message(msg.chat.id, "Please send me a name for the room.")
                    .await?;
                set_room_dialogue
                    .update(dialogues::set_room::State::ReceiveRoomName)
                    .await?;
            }
            Command::RoomDel => {
                if !app_config.is_admin(&msg.chat.id.0) {
                    bot.send_message(msg.chat.id, "Insufficient permission.")
                        .await?;
                    return Ok(());
                }

                let rooms = sqlx::query!("SELECT * FROM rooms").fetch_all(&db).await?;
                if rooms.len() <= 0 {
                    bot.send_message(msg.chat.id, "No rooms defined.").await?;
                    return Ok(());
                }

                let keyboard = InlineDataKeyboard::new().buttons(
                    rooms
                        .into_iter()
                        .map(|room| {
                            serde_json::Result::<InlineDataKeyboardButton>::Ok(
                                InlineDataKeyboardButton {
                                    text: room.name.to_owned(),
                                    data: serde_json::to_string(&CallbackType::RoomDel {
                                        name: room.name.to_owned(),
                                    })?,
                                },
                            )
                        })
                        .try_collect()?,
                );
                let keyboard_msg = bot
                    .send_message(msg.chat.id, "Select a room to delete.")
                    .reply_markup(keyboard.build_inline_keyboard_markup())
                    .await?;
                keyboard.insert_into_db(&db, &keyboard_msg.id).await?;
            }
            Command::GroupLink => {
                if !app_config.is_admin(&msg.chat.id.0) {
                    bot.send_message(msg.chat.id, "Insufficient permission.")
                        .await?;
                    return Ok(());
                }

                let button = KeyboardButton::new("Choose a group to authorize users").request(
                    ButtonRequest::RequestChat(
                        KeyboardButtonRequestChat::new(0, false).bot_is_member(true),
                    ),
                );
                bot.send_message(msg.chat.id, "Please pick a group")
                    .reply_markup(
                        KeyboardMarkup::new([[button]])
                            .one_time_keyboard()
                            .resize_keyboard(),
                    )
                    .await?;
            }
        };

        Ok(())
    }

    pub fn make_handler() -> Handler<
        'static,
        DependencyMap,
        Result<(), Box<dyn Error + Send + Sync>>,
        DpHandlerDescription,
    > {
        dptree::entry()
            .filter_map(|msg: Message| msg.text().and_then(|text| Command::parse(text, "").ok()))
            .endpoint(Command::execute)
    }
}
