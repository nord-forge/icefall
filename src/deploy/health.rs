use std::time::Duration;

use crate::deploy::DeployError;

pub async fn wait_for_healthy(
    host_port: u16,
    max_attempts: u32,
    interval_ms: u64,
) -> Result<(), DeployError> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .expect("reqwest client builder with default TLS");

    let url = format!("http://127.0.0.1:{host_port}/");

    for attempt in 1..=max_attempts {
        match client.get(&url).send().await {
            Ok(resp) if resp.status().is_success() || resp.status().is_redirection() => {
                tracing::info!("Health check passed on attempt {attempt}");
                return Ok(());
            }
            Ok(resp) => {
                tracing::debug!(
                    "Health check attempt {attempt}/{max_attempts}: status {}",
                    resp.status()
                );
            }
            Err(e) => {
                tracing::debug!("Health check attempt {attempt}/{max_attempts}: {e}");
            }
        }

        if attempt < max_attempts {
            tokio::time::sleep(Duration::from_millis(interval_ms)).await;
        }
    }

    Err(DeployError::HealthCheckFailed(max_attempts))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn fails_on_unreachable_port() {
        let result = wait_for_healthy(19999, 3, 100).await;
        assert!(result.is_err());
        match result {
            Err(DeployError::HealthCheckFailed(attempts)) => assert_eq!(attempts, 3),
            other => panic!("expected HealthCheckFailed, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn succeeds_on_responsive_server() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let response = "HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nOK";
            tokio::io::AsyncWriteExt::write_all(&mut stream, response.as_bytes())
                .await
                .unwrap();
        });

        let result = wait_for_healthy(port, 5, 100).await;
        assert!(result.is_ok());
    }
}
