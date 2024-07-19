use std::{error::Error, fs::create_dir, io, path::PathBuf};

#[cfg(feature = "dotenvy")]
use dotenvy::dotenv;
use serde::Deserialize;

fn ensure_dir(path: &PathBuf) -> std::io::Result<()> {
    match create_dir(&path) {
        Err(e) => match e.kind() {
            std::io::ErrorKind::AlreadyExists => Ok(()),
            _ => Err(e),
        },
        ok => ok,
    }
}

pub struct AppConfig {
    pub env: EnvConfig,
    pub audio_dir: PathBuf,
    pub db_file: PathBuf,
}

impl AppConfig {
    pub fn init() -> Result<Self, Box<dyn Error>> {
        let env = EnvConfig::from_dotenv()?;
        let data_dir = PathBuf::from(&env.data_dir);
        let audio_dir = data_dir.join("audios");
        let db_file = data_dir.join("bot.db");

        ensure_dir(&data_dir)?;
        ensure_dir(&audio_dir)?;

        Ok(AppConfig {
            env,
            audio_dir,
            db_file,
        })
    }

    pub fn is_admin(&self, id: &i64) -> bool {
        self.env.admin_users.contains(id)
    }
}

#[derive(Deserialize, Debug)]
pub struct EnvConfig {
    pub ahm_host: String,
    #[serde(default = "default_ahm_port")]
    pub ahm_port: u16,
    pub bot_token: String,
    pub admin_users: Vec<i64>,
    pub player_command: String,
    #[serde(default = "default_player_start_delay")]
    pub player_start_delay: u64,
    #[serde(default = "default_data_dir")]
    pub data_dir: String,
}

fn default_ahm_port() -> u16 {
    51325
}

fn default_player_start_delay() -> u64 {
    0
}

fn default_data_dir() -> String {
    "./data".into()
}

impl EnvConfig {
    pub fn from_dotenv() -> Result<Self, Box<dyn Error>> {
        #[cfg(feature = "dotenvy")]
        match dotenv() {
            Err(dotenvy::Error::Io(err)) => match err.kind() {
                io::ErrorKind::NotFound => log::warn!("no .env file found"),
                _ => {
                    log::error!("{}", err)
                }
            },
            Err(err) => {
                log::error!("{}", err)
            }
            Ok(_) => {}
        }

        return Ok(envy::from_env::<EnvConfig>()?);
    }
}
