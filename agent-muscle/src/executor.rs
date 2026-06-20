use anyhow::Result;
use serde::Serialize;
use std::path::Path;
use tokio::process::Command;

#[derive(Debug, Serialize)]
pub struct ExecResult {
    pub job_id: String,
    pub command: String,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
    pub success: bool,
}

pub async fn run_command(cmd: &str, cwd: Option<&Path>) -> Result<ExecResult> {
    let job_id = uuid::Uuid::new_v4().to_string();
    let start = std::time::Instant::now();

    let output = if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args(["/C", cmd])
            .current_dir(cwd.unwrap_or(Path::new(".")))
            .output()
            .await?
    } else {
        Command::new("sh")
            .args(["-c", cmd])
            .current_dir(cwd.unwrap_or(Path::new(".")))
            .output()
            .await?
    };

    let duration_ms = start.elapsed().as_millis() as u64;
    let exit_code = output.status.code().unwrap_or(-1);

    Ok(ExecResult {
        job_id,
        command: cmd.to_string(),
        exit_code,
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        duration_ms,
        success: output.status.success(),
    })
}
