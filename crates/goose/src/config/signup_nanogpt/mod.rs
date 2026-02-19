use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::{sleep, timeout};

use crate::config::Config;

/// Default model for NanoGPT configuration
pub const NANOGPT_DEFAULT_MODEL: &str = "openai/gpt-4.1-nano";

const NANOGPT_START_URL: &str = "https://nano-gpt.com/api/cli-login/start";
const NANOGPT_POLL_URL: &str = "https://nano-gpt.com/api/cli-login/poll";
const AUTH_TIMEOUT: Duration = Duration::from_secs(600); // 10 minutes
const POLL_INTERVAL: Duration = Duration::from_secs(2);

#[derive(Debug, Serialize)]
struct StartRequest {
    client_name: String,
}

#[derive(Debug, Deserialize)]
struct StartResponse {
    device_code: String,
    verification_uri_complete: String,
}

#[derive(Debug, Serialize)]
struct PollRequest {
    device_code: String,
}

#[derive(Debug, Deserialize)]
struct PollResponse {
    key: String,
}

async fn poll_for_token(device_code: &str) -> Result<String> {
    let client = Client::new();

    loop {
        sleep(POLL_INTERVAL).await;

        let body = PollRequest {
            device_code: device_code.to_string(),
        };

        let response = client.post(NANOGPT_POLL_URL).json(&body).send().await?;

        match response.status().as_u16() {
            200 => {
                let poll_resp: PollResponse = response.json().await?;
                return Ok(poll_resp.key);
            }
            202 => {
                // Authorization pending, continue polling
                continue;
            }
            410 => {
                return Err(anyhow!("Device code has expired - please try again"));
            }
            409 => {
                return Err(anyhow!("Device code has already been consumed"));
            }
            404 => {
                return Err(anyhow!("Invalid device code"));
            }
            other => {
                let error_text = response.text().await.unwrap_or_default();
                return Err(anyhow!(
                    "Unexpected poll response: {} - {}",
                    other,
                    error_text
                ));
            }
        }
    }
}

pub async fn complete_nanogpt_auth() -> Result<String> {
    let client = Client::new();
    let body = StartRequest {
        client_name: "goose".to_string(),
    };

    let response = client.post(NANOGPT_START_URL).json(&body).send().await?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        return Err(anyhow!(
            "Failed to start NanoGPT device flow: {} - {}",
            status,
            error_text
        ));
    }

    let start_resp: StartResponse = response.json().await?;

    println!("Opening browser for NanoGPT authentication...");

    if let Err(e) = webbrowser::open(&start_resp.verification_uri_complete) {
        eprintln!("Failed to open browser automatically: {}", e);
        println!(
            "Please open this URL manually: {}",
            start_resp.verification_uri_complete
        );
    }

    println!("Waiting for NanoGPT authorization...");

    match timeout(AUTH_TIMEOUT, poll_for_token(&start_resp.device_code)).await {
        Ok(Ok(api_key)) => Ok(api_key),
        Ok(Err(e)) => Err(e),
        Err(_) => Err(anyhow!("Authentication timed out - please try again")),
    }
}

pub fn configure_nanogpt(config: &Config, api_key: String) -> Result<()> {
    config.set_secret("NANOGPT_API_KEY", &api_key)?;
    config.set_goose_provider("nanogpt")?;
    config.set_goose_model(NANOGPT_DEFAULT_MODEL)?;
    Ok(())
}
