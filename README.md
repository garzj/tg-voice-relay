# tg-voice-relay

## Build this project

- `cargo build --release`

## Setup a development db

- Set `DATABASE_URL=sqlite://data/bot.db` variable in a `.env` file.
- `cargo install sqlx-cli`
- `mkdir data`
- `sqlx database create`
- `sqlx database setup`
- When editing SQL queries, don't forget to run `sqlx database prepare` before commiting changes!
- More information [here](https://github.com/launchbadge/sqlx/tree/main/sqlx-cli)
