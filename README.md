# tg-voice-relay

## Configuration

The app is configured via environment variables listed in [this file](./src/config.rs). Here is a minimal example `.env` file:

```
AHM_HOST=169.42.0.1
BOT_TOKEN=<your_telegram_bot_token>
ADMIN_USERS=<some_user_id>,<some_other_user_id>,...
PLAYER_COMMAND="ffplay -nodisp -autoexit %f"
```

### Player command examples

#### Play audio on speaker (Windows)

- install SoX for playing audios: `winget install --id=ChrisBagwell.SoX -e`
- get the full name of the speaker (i. e. from Windows Device Manager)
- use it like this:
  ```
  PLAYER_COMMAND='"C:\Program Files (x86)\sox-14-4-2\sox.exe" -q %f -t waveaudio "High Definition Audio Device"'
  ```

## Build and run

- `cargo build --release`, saves an executable into the `target/release` dir
- run the executable having the `.env` in the working dir

## Development

### Setup a development db

- Set `DATABASE_URL=sqlite://data/bot.db` variable in a `.env` file.
- `cargo install sqlx-cli`
- `mkdir data`
- `sqlx database create`
- `sqlx database setup`
- When editing SQL queries, don't forget to run `sqlx database prepare` before commiting changes!
- More information [here](https://github.com/launchbadge/sqlx/tree/main/sqlx-cli)
