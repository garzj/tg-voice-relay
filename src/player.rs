use std::{io, time::Duration};

use thiserror::Error;
use tokio::{
    process::Command,
    select,
    sync::{oneshot, Mutex, MutexGuard},
    time,
};

use crate::{ahm::AHMConnection, config::AppConfig};

#[derive(Error, Debug)]
pub enum PlayAudioError {
    #[error("an audio is already being played")]
    AlreadyPlaying,
    #[error("child process returned")]
    ChildProcessError(#[from] io::Error),
}

#[derive(Error, Debug)]
pub enum StopAudioError {
    #[error("no audio is being played")]
    AlreadyStopped,
}

pub struct Player {
    player_lock: Mutex<()>,
    kill_tx: Mutex<Option<oneshot::Sender<()>>>,
    ahm_endpoint: String,
    player_start_delay: u64,
    player_command: String,
}

pub struct PlayerLock<'a> {
    player: &'a Player,
    guard: MutexGuard<'a, ()>,
}

impl Player {
    pub fn new(app_config: &AppConfig) -> Self {
        let ahm_endpoint = format!("{}:{}", &app_config.env.ahm_host, &app_config.env.ahm_port);
        Player {
            player_lock: Mutex::new(()),
            kill_tx: Mutex::new(None),
            ahm_endpoint,
            player_start_delay: app_config.env.player_start_delay,
            player_command: app_config.env.player_command.clone(),
        }
    }

    pub async fn set_channel(&self, channel: u16) -> io::Result<()> {
        let mut ahm = AHMConnection::connect(&self.ahm_endpoint).await?;
        ahm.write_preset(channel).await
    }

    pub fn try_lock(&self) -> Result<PlayerLock, PlayAudioError> {
        let guard = self
            .player_lock
            .try_lock()
            .map_err(|_| PlayAudioError::AlreadyPlaying)?;

        Ok(PlayerLock {
            player: self,
            guard,
        })
    }

    pub async fn stop_playing(&self) -> Result<(), StopAudioError> {
        let mut kill_tx = self.kill_tx.lock().await;
        let _ = kill_tx
            .take()
            .ok_or(StopAudioError::AlreadyStopped)?
            .send(());
        Ok(())
    }
}

impl<'a> PlayerLock<'a> {
    pub async fn play_audio_file(self, path: &str) -> std::io::Result<()> {
        let (kill_tx, kill_rx) = oneshot::channel::<()>();

        {
            let mut kill_tx_guard = self.player.kill_tx.lock().await;
            if let Some(old_kill_tx) = kill_tx_guard.take() {
                log::error!("the kill channel has already been initialized for this player, will kill and replace");
                let _ = old_kill_tx.send(());
            }
            kill_tx_guard.replace(kill_tx);
        }

        #[cfg(target_family = "unix")]
        let shell = "sh";
        #[cfg(target_family = "unix")]
        let arg = "-c";
        #[cfg(target_family = "windows")]
        let shell = "cmd";
        #[cfg(target_family = "windows")]
        let arg = "/C";

        let cmd_line = self.player.player_command.replace("%f", &path);

        let proc = async {
            time::sleep(Duration::from_millis(self.player.player_start_delay)).await;

            let proc = Command::new(shell)
                .args([arg, &cmd_line])
                .kill_on_drop(true)
                .output();

            proc.await
        };

        let finished = select! {
            result = proc => Some(result),
            result = kill_rx => {
                if let Err(err) = result {
                    log::error!("recv error for player's kill receiver: {}", err);
                }
                None
            }
        };
        let Some(result) = finished else {
            return Ok(());
        };
        let output = result?;

        if let Some(code) = output.status.code() {
            if code != 0 {
                if let Ok(stdout_str) = std::str::from_utf8(&output.stdout) {
                    log::warn!("stdout: {}", stdout_str);
                }
                if let Ok(stderr_str) = std::str::from_utf8(&output.stderr) {
                    log::warn!("stderr: {}", stderr_str);
                }
                log::warn!("command line: {}", cmd_line);
                log::warn!(
                    "this player process terminated with a non-zero exit code: {}",
                    code
                );
            }
        }

        self.player.kill_tx.lock().await.take();

        drop(self.guard);

        Ok(())
    }
}
