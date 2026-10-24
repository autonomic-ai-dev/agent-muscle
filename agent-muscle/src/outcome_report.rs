//! Reports finished runs to agent-heart so outcomes feed heart's learning data.
//! Best effort: a failed or slow report never changes the command result.

use std::time::Duration;

use serde::Serialize;

const DEFAULT_HEART_URL: &str = "http://127.0.0.1:3101";
const REPORT_TIMEOUT: Duration = Duration::from_millis(500);

#[derive(Debug, Serialize)]
pub struct OutcomeReport<'a> {
    pub id: &'a str,
    pub organ: &'static str,
    pub kind: &'static str,
    pub success: bool,
    pub exit_code: Option<i32>,
    pub duration_ms: u64,
}

pub async fn report_to(base_url: &str, report: &OutcomeReport<'_>) -> Result<(), reqwest::Error> {
    reqwest::Client::builder()
        .timeout(REPORT_TIMEOUT)
        .build()?
        .post(format!("{base_url}/outcomes/record"))
        .json(report)
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}

pub async fn report(report: &OutcomeReport<'_>) {
    let base_url =
        std::env::var("AUTONOMIC_HEART_URL").unwrap_or_else(|_| DEFAULT_HEART_URL.to_string());
    if let Err(err) = report_to(&base_url, report).await {
        tracing::warn!(error = %err, "outcome not reported to agent-heart");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    fn sample() -> OutcomeReport<'static> {
        OutcomeReport {
            id: "job-1",
            organ: "agent-muscle",
            kind: "bash",
            success: true,
            exit_code: Some(0),
            duration_ms: 3,
        }
    }

    #[tokio::test]
    async fn posts_outcome_to_heart_endpoint() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0u8; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            socket
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                .await
                .unwrap();
            String::from_utf8_lossy(&buf[..n]).into_owned()
        });

        report_to(&base, &sample()).await.unwrap();
        let request = server.await.unwrap();
        assert!(request.starts_with("POST /outcomes/record"));
        assert!(request.contains("\"organ\":\"agent-muscle\""));
    }

    #[tokio::test]
    async fn unreachable_heart_is_an_error_not_a_panic() {
        let port = {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            listener.local_addr().unwrap().port()
        };
        let base = format!("http://127.0.0.1:{port}");
        assert!(report_to(&base, &sample()).await.is_err());
    }

    #[tokio::test]
    async fn heart_error_status_is_reported_as_an_error() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0u8; 4096];
            let _ = socket.read(&mut buf).await.unwrap();
            socket
                .write_all(
                    b"HTTP/1.1 500 Internal Server Error\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                )
                .await
                .unwrap();
        });
        assert!(report_to(&base, &sample()).await.is_err());
    }
}
