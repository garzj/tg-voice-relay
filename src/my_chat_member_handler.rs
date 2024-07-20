use std::error::Error;

use sqlx::{Pool, Sqlite};
use teloxide::{
    dispatching::{DpHandlerDescription, UpdateFilterExt},
    dptree::Handler,
    prelude::DependencyMap,
    types::{ChatMemberUpdated, Update},
};

pub fn make_my_chat_member_handler(
) -> Handler<'static, DependencyMap, Result<(), Box<dyn Error + Send + Sync>>, DpHandlerDescription>
{
    Update::filter_my_chat_member().endpoint(
        |member_update: ChatMemberUpdated, db: Pool<Sqlite>| async move {
            if member_update.old_chat_member.is_present()
                && !member_update.new_chat_member.is_present()
            {
                sqlx::query!(
                    "DELETE FROM auth_groups WHERE id=?",
                    member_update.chat.id.0
                )
                .execute(&db)
                .await?;
            }
            Ok(())
        },
    )
}
