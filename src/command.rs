use teloxide::{prelude::*, utils::command::BotCommands};

#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
pub enum Command {
    #[command(description = "show this list.")]
    Help,
}

impl Command {
    pub async fn execute(&self, bot: &Bot, msg: &Message) -> ResponseResult<()> {
        match self {
            Command::Help => {
                bot.send_message(msg.chat.id, Command::descriptions().to_string())
                    .await?;
            }
        };

        Ok(())
    }
}
