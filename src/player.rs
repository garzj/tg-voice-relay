use std::{io, time::Duration};

use tokio::{process::Command, time};

use crate::{ahm::AHMConnection, config::AppConfig};

pub struct Player {
    ahm_endpoint: String,
    player_start_delay: u64,
    player_command: String,
}

impl Player {
    pub fn new(app_config: &AppConfig) -> Self {
        let ahm_endpoint = format!("{}:{}", &app_config.env.ahm_host, &app_config.env.ahm_port);

        Player {
            ahm_endpoint,
            player_start_delay: app_config.env.player_start_delay,
            player_command: app_config.env.player_command.clone(),
        }
    }

    pub async fn set_channel(&self, channel: u16) -> io::Result<()> {
        let mut ahm = AHMConnection::connect(&self.ahm_endpoint).await?;
        ahm.write_preset(channel).await
    }

    pub async fn play_audio_file(
        &self,
        path: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        #[cfg(target_family = "unix")]
        let shell = "sh";
        #[cfg(target_family = "unix")]
        let arg = "-c";
        #[cfg(target_family = "windows")]
        let shell = "cmd";
        #[cfg(target_family = "windows")]
        let arg = "/C";

        let cmd_line = self.player_command.replace("%f", &path);

        time::sleep(Duration::from_millis(self.player_start_delay)).await;

        let output = Command::new(shell).args([arg, &cmd_line]).output().await?;
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

        Ok(())
    }
}
