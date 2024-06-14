use std::time::Duration;

use tokio::{process::Command, time};

pub async fn play_audio_file(
    path: &str,
    cmd_template: &str,
    player_start_delay: u64,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    #[cfg(target_family = "unix")]
    let shell = "sh";
    #[cfg(target_family = "unix")]
    let arg = "-c";
    #[cfg(target_family = "windows")]
    let shell = "cmd";
    #[cfg(target_family = "windows")]
    let arg = "/C";

    let cmd_line = &cmd_template.replace("%f", &path);

    time::sleep(Duration::from_millis(player_start_delay)).await;

    let output = Command::new(shell).args([arg, cmd_line]).output().await?;
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
