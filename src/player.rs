use std::{io, time::Duration};

use thiserror::Error;
use tokio::{
    process::Command,
    select,
    sync::{oneshot, Mutex, MutexGuard},
    time,
};

use crate::{ahm::AHMConnection, config::EnvConfig};

#[derive(Error, Debug)]
pub enum PlayAudioError {
    #[error("an audio is already being played")]
    AlreadyPlaying,
    #[error("child process returned")]
    ChildProcessError(#[from] io::Error),
    #[error("command parse error")]
    CommandParseError,
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

pub struct PlayerConfig {
    pub ahm_host: String,
    pub ahm_port: u16,
    pub player_start_delay: u64,
    pub player_command: String,
}

impl From<&EnvConfig> for PlayerConfig {
    fn from(env: &EnvConfig) -> Self {
        PlayerConfig {
            player_command: env.player_command.to_owned(),
            player_start_delay: env.player_start_delay,
            ahm_port: env.ahm_port,
            ahm_host: env.ahm_host.to_owned(),
        }
    }
}

impl Player {
    pub fn new(player_config: &PlayerConfig) -> Self {
        let ahm_endpoint = format!("{}:{}", player_config.ahm_host, player_config.ahm_port);
        Player {
            player_lock: Mutex::new(()),
            kill_tx: Mutex::new(None),
            ahm_endpoint,
            player_start_delay: player_config.player_start_delay,
            player_command: player_config.player_command.clone(),
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
    pub async fn play_audio_file(self, path: &str) -> Result<(), PlayAudioError> {
        log::info!("starting to play file: {}", path);
        let (kill_tx, kill_rx) = oneshot::channel::<()>();

        {
            let mut kill_tx_guard = self.player.kill_tx.lock().await;
            if let Some(old_kill_tx) = kill_tx_guard.take() {
                log::error!("the kill channel has already been initialized for this player, will kill and replace");
                let _ = old_kill_tx.send(());
            }
            kill_tx_guard.replace(kill_tx);
        }
        log::debug!("replaced player kill channel");

        // todo: store e in CommandParseError
        let mut args = shell_words::split(&self.player.player_command)
            .map_err(|_| PlayAudioError::CommandParseError)?
            .into_iter();
        let shell = args.next().ok_or(PlayAudioError::CommandParseError)?;
        let args: Vec<String> = args
            .map(|arg| if arg == "%f" { path.to_owned() } else { arg })
            .collect();

        let all_args_iter = std::iter::once(shell.clone()).chain(args.iter().cloned());
        let cmd_line = shell_words::join(all_args_iter);

        let proc = async {
            time::sleep(Duration::from_millis(self.player.player_start_delay)).await;

            let proc = Command::new(shell).args(args).kill_on_drop(true).output();
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
        log::debug!("player done with file: {}", path);
        self.player.kill_tx.lock().await.take();
        log::debug!("player kill channel cleared");

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

        drop(self.guard);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::{future::Future, pin::Pin, sync::Arc, time::Instant};

    use tokio::task::JoinSet;

    use super::*;

    fn make_player() -> Arc<Player> {
        let config = PlayerConfig {
            ahm_port: 51325,
            ahm_host: "127.0.0.1".into(),
            player_start_delay: 250,
            player_command: "sh -c %f".into(),
        };
        Arc::new(Player::new(&config))
    }

    async fn join_all(futures: Vec<Pin<Box<dyn Future<Output = ()> + Send>>>) {
        let mut set = JoinSet::new();
        for future in futures {
            set.spawn(future);
        }
        while let Some(res) = set.join_next().await {
            res.expect("async task failed");
        }
    }

    #[tokio::test]
    async fn player_lock() {
        let player = make_player();

        let shell = "sleep 3";

        let fut1 = Box::pin({
            let player = player.clone();
            async move {
                let lock1 = player.try_lock().expect("lock 1 failed");
                let start = Instant::now();
                lock1
                    .play_audio_file(&shell)
                    .await
                    .expect("lock 1 command failed");
                assert!(
                    start.elapsed().as_millis() >= 3000,
                    "lock 1 command took to little"
                );
            }
        });

        let fut2 = Box::pin({
            let player = player.clone();
            async move {
                tokio::time::sleep(Duration::from_secs(1)).await;

                let lock2 = player.try_lock();
                assert!(
                    matches!(lock2, Err(PlayAudioError::AlreadyPlaying)),
                    "locked a second time"
                );

                tokio::time::sleep(Duration::from_secs(3)).await;

                let lock3 = player.try_lock().expect("still locked after play");
                lock3
                    .play_audio_file(&shell)
                    .await
                    .expect("lock 3 command failed");
            }
        });

        join_all(vec![fut1, fut2]).await;
    }

    async fn player_kill(shell: &'static str) {
        let player = make_player();

        let fut1 = Box::pin({
            let player = player.clone();

            async move {
                let lock1 = player.try_lock().expect("lock 1 failed");
                lock1
                    .play_audio_file(&shell)
                    .await
                    .expect("lock 1 command failed");
            }
        });

        let fut2 = Box::pin({
            let player = player.clone();

            async move {
                tokio::time::sleep(Duration::from_secs(1)).await;

                let stop1 = Box::pin({
                    let player = player.clone();
                    async move {
                        player.stop_playing().await.expect("stop 1 failed");
                    }
                });
                let stop2 = Box::pin({
                    let player = player.clone();
                    async move {
                        player
                            .stop_playing()
                            .await
                            .expect_err("stop 2 should have failed");
                    }
                });
                join_all(vec![stop1, stop2]).await;

                tokio::time::sleep(Duration::from_secs(1)).await;

                let lock2 = player.try_lock().expect("lock 2 failed");
                lock2
                    .play_audio_file(&shell)
                    .await
                    .expect("lock 2 command failed");
            }
        });

        join_all(vec![fut1, fut2]).await;

        player.try_lock().expect("lock 3 failed");
    }

    #[tokio::test]
    async fn player_kill_no_output() {
        player_kill("sleep 3").await;
    }

    #[tokio::test]
    async fn player_kill_with_output() {
        player_kill("echo testoutput && sleep 3 && exit 1").await;
    }
}
