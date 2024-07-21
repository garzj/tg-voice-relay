use sqlx::{sqlite::SqliteQueryResult, Pool, QueryBuilder, Sqlite};
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup, MessageId};

pub struct InlineDataKeyboardButton {
    pub text: String,
    pub data: String,
}

pub struct InlineDataKeyboard {
    chunk_size: usize,
    buttons: Vec<InlineDataKeyboardButton>,
}

impl InlineDataKeyboard {
    pub fn new() -> InlineDataKeyboardBuilder {
        InlineDataKeyboardBuilder { chunk_size: 3 }
    }

    pub fn build_inline_keyboard(&self) -> Vec<Vec<InlineKeyboardButton>> {
        self.buttons
            .chunks(self.chunk_size)
            .enumerate()
            .map(|(i, row)| {
                row.iter()
                    .enumerate()
                    .map(|(j, button)| -> InlineKeyboardButton {
                        let button_index = i * self.chunk_size + j;
                        InlineKeyboardButton::callback(&button.text, button_index.to_string())
                    })
                    .collect()
            })
            .collect()
    }

    pub fn build_inline_keyboard_markup(&self) -> InlineKeyboardMarkup {
        InlineKeyboardMarkup::new(self.build_inline_keyboard())
    }

    pub async fn insert_into_db(
        self,
        db: &Pool<Sqlite>,
        message_id: &MessageId,
    ) -> sqlx::Result<SqliteQueryResult> {
        let mut query_builder: QueryBuilder<Sqlite> =
            QueryBuilder::new("INSERT INTO keyboard_buttons (message_id, button_index, data)");
        query_builder.push_values(
            self.buttons.into_iter().enumerate(),
            |mut b, (button_index, button)| {
                b.push_bind(message_id.0)
                    .push_bind(button_index as i64)
                    .push_bind(button.data);
            },
        );
        query_builder.build().execute(db).await
    }

    pub async fn remove_from_db(
        db: &Pool<Sqlite>,
        message_id: &MessageId,
    ) -> sqlx::Result<SqliteQueryResult> {
        sqlx::query!(
            "DELETE FROM keyboard_buttons WHERE message_id = ?",
            message_id.0
        )
        .execute(db)
        .await
    }
}

pub struct InlineDataKeyboardBuilder {
    chunk_size: usize,
}

impl InlineDataKeyboardBuilder {
    #[allow(dead_code)]
    pub fn chunk_size(self, chunk_size: usize) -> Self {
        Self { chunk_size, ..self }
    }

    pub fn buttons(self, buttons: Vec<InlineDataKeyboardButton>) -> InlineDataKeyboard {
        InlineDataKeyboard {
            chunk_size: self.chunk_size,
            buttons,
        }
    }
}
