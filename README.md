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

- install ffmpeg: `winget install --id=Gyan.FFmpeg -e`

First, get the id of the Speaker (using `powershell`):

```powershell
$AudioDeviceFilter="" # fill me in to filter output
Get-ItemProperty "HKLM:\SYSTEM\CurrentControlSet\Enum\SWD\MMDEVAPI\*" | Where-Object {($_.FriendlyName -Match $AudioDeviceFilter) -and ($_.PSChildName -Match "0\.0\.0")} | Select-Object -Property FriendlyName,PSChildName
```

Copy the PSChildName from the output and use it like this:

```
PLAYER_COMMAND='"C:\Program Files\VideoLAN\VLC\vlc.exe" -I dummy --dummy-quiet --no-one-instance --mmdevice-audio-device {0.0.0.00000000}.{some-guid} --play-and-exit %f'
```

This GUID may also be visible somwhere in Windows Device Manager, one may double check that the device is correct.

## Build and run

- `cargo build --release`, saves an executable into the `target/release` dir
- run the executable with the `.env` in the same dir

## Development

### Setup a development db

- Set `DATABASE_URL=sqlite://data/bot.db` variable in a `.env` file.
- `cargo install sqlx-cli`
- `mkdir data`
- `sqlx database create`
- `sqlx database setup`
- When editing SQL queries, don't forget to run `sqlx database prepare` before commiting changes!
- More information [here](https://github.com/launchbadge/sqlx/tree/main/sqlx-cli)
