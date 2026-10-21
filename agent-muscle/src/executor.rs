use anyhow::Result;
use serde::Serialize;
use std::path::Path;
use std::time::Duration;
use tokio::process::Command;

use crate::ast_guard::validate_bash;
use crate::outcome_report::{self, OutcomeReport};
use crate::output_slicer::{compress_output, DEFAULT_MAX_OUTPUT_LINES};

const DEFAULT_TIMEOUT_SECS: u64 = 60;

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
    let started = std::time::Instant::now();
    if let Err(err) = validate_bash(cmd) {
        let job_id = uuid::Uuid::new_v4().to_string();
        report_outcome(&job_id, "bash_guard", false, None, started).await;
        return Err(anyhow::anyhow!("{err}"));
    }
    let outcome = exec_shell(cmd, cwd, Duration::from_secs(DEFAULT_TIMEOUT_SECS)).await;
    match &outcome {
        Ok(res) => {
            report_outcome(
                &res.job_id,
                "bash",
                res.success,
                Some(res.exit_code),
                started,
            )
            .await
        }
        Err(_) => {
            let job_id = uuid::Uuid::new_v4().to_string();
            report_outcome(&job_id, "bash", false, None, started).await;
        }
    }
    outcome
}

async fn report_outcome(
    job_id: &str,
    kind: &'static str,
    success: bool,
    exit_code: Option<i32>,
    started: std::time::Instant,
) {
    outcome_report::report(&OutcomeReport {
        id: job_id,
        organ: "agent-muscle",
        kind,
        success,
        exit_code,
        duration_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
    })
    .await;
}

pub async fn run_python(code: &str) -> Result<ExecResult> {
    let job_id = uuid::Uuid::new_v4().to_string();
    let start = std::time::Instant::now();

    let output = Command::new("python3").args(["-c", code]).output().await?;

    let duration_ms = start.elapsed().as_millis() as u64;
    let exit_code = output.status.code().unwrap_or(-1);

    let raw_stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let raw_stderr = String::from_utf8_lossy(&output.stderr).to_string();

    let stdout = compress_output(&raw_stdout, exit_code, DEFAULT_MAX_OUTPUT_LINES);
    let stderr = compress_output(&raw_stderr, exit_code, DEFAULT_MAX_OUTPUT_LINES);

    Ok(ExecResult {
        job_id,
        command: format!("python3 -c {}", code.len()),
        exit_code,
        stdout,
        stderr,
        duration_ms,
        success: output.status.success(),
    })
}

async fn exec_shell(cmd: &str, cwd: Option<&Path>, timeout: Duration) -> Result<ExecResult> {
    let job_id = uuid::Uuid::new_v4().to_string();
    let start = std::time::Instant::now();

    let mut command = if cfg!(target_os = "windows") {
        let mut command = Command::new("cmd");
        command.args(["/C", cmd]);
        command
    } else {
        let mut command = Command::new("sh");
        command.args(["-c", cmd]);
        command
    };
    command
        .current_dir(cwd.unwrap_or(Path::new(".")))
        .kill_on_drop(true);

    let output = tokio::time::timeout(timeout, command.output())
        .await
        .map_err(|_| anyhow::anyhow!("command timed out after {}s", timeout.as_secs()))??;

    let duration_ms = start.elapsed().as_millis() as u64;
    let exit_code = output.status.code().unwrap_or(-1);

    let raw_stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let raw_stderr = String::from_utf8_lossy(&output.stderr).to_string();

    let stdout = compress_output(&raw_stdout, exit_code, DEFAULT_MAX_OUTPUT_LINES);
    let stderr = compress_output(&raw_stderr, exit_code, DEFAULT_MAX_OUTPUT_LINES);

    Ok(ExecResult {
        job_id,
        command: cmd.to_string(),
        exit_code,
        stdout,
        stderr,
        duration_ms,
        success: output.status.success(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn run_command_executes_valid_echo() {
        let res = run_command("echo hello_muscle", None).await.unwrap();
        assert_eq!(res.exit_code, 0);
        assert!(res.stdout.contains("hello_muscle"));
        assert!(res.success);
    }

    #[tokio::test]
    async fn run_command_rejects_ast_syntax_error() {
        let res = run_command("echo \"unclosed quote", None).await;
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("AST syntax error"));
    }

    #[tokio::test]
    async fn exec_shell_kills_command_that_exceeds_timeout() {
        let res = exec_shell("sleep 5", None, Duration::from_millis(200)).await;
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("timed out"));
    }

    #[tokio::test]
    async fn run_command_rejects_destructive_root_rm() {
        let res = run_command("rm -rf /", None).await;
        assert!(res.is_err());
        assert!(res
            .unwrap_err()
            .to_string()
            .contains("Destructive command blocked"));
    }
}
