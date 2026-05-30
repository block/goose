pub mod server;

#[cfg(test)]
mod tests;

use anyhow::{anyhow, Result};
use rand::{distributions::Alphanumeric, Rng};
use serde::Deserialize;
use std::time::Duration;
use tokio::sync::oneshot;
use tokio::time::timeout;

const APP_SLUG: &str = "aaif-goose-bot";
const CALLBACK_PORT: u16 = 3458;
const INSTALL_TIMEOUT: Duration = Duration::from_secs(600);

#[derive(Debug, Deserialize)]
pub struct InstallCallback {
    pub oauth_code: String,
}

pub type InstallCallbackResult = std::result::Result<InstallCallback, String>;

pub struct GooseBotInstallFlow {
    state: String,
    server_shutdown_tx: Option<oneshot::Sender<()>>,
    oauth_client_id: Option<String>,
}

impl GooseBotInstallFlow {
    pub fn new() -> Self {
        let state: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(64)
            .map(char::from)
            .collect();
        Self {
            state,
            server_shutdown_tx: None,
            oauth_client_id: None,
        }
    }

    pub fn with_oauth_client_id(mut self, oauth_client_id: String) -> Self {
        self.oauth_client_id = Some(oauth_client_id);
        self
    }

    pub fn app_install_url(&self) -> String {
        format!(
            "https://github.com/apps/{}/installations/new?state={}",
            APP_SLUG, self.state
        )
    }

    pub fn open_app_install_url(&self) {
        let _ = webbrowser::open(&self.app_install_url());
    }

    pub fn install_url(&self) -> String {
        if let Some(client_id) = &self.oauth_client_id {
            return format!(
                "https://github.com/login/oauth/authorize?client_id={}&state={}&redirect_uri={}",
                client_id,
                self.state,
                urlencoding::encode(&Self::callback_url())
            );
        }
        self.app_install_url()
    }

    pub fn callback_url() -> String {
        format!("http://localhost:{}/", CALLBACK_PORT)
    }

    pub async fn complete_flow(&mut self) -> Result<InstallCallback> {
        let cb_rx = self.start_callback_server().await?;
        let _ = webbrowser::open(&self.install_url());
        Self::wait_for_callback(cb_rx).await
    }

    pub async fn await_callback(&mut self) -> Result<InstallCallback> {
        let cb_rx = self.start_callback_server().await?;
        Self::wait_for_callback(cb_rx).await
    }

    async fn start_callback_server(&mut self) -> Result<oneshot::Receiver<InstallCallbackResult>> {
        let (cb_tx, cb_rx) = oneshot::channel::<InstallCallbackResult>();
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
        let (ready_tx, ready_rx) = oneshot::channel::<Result<()>>();
        self.server_shutdown_tx = Some(shutdown_tx);

        let expected_state = self.state.clone();
        tokio::spawn(async move {
            if let Err(e) = server::run_callback_server(
                cb_tx,
                shutdown_rx,
                ready_tx,
                expected_state,
                CALLBACK_PORT,
            )
            .await
            {
                tracing::error!("[goose-bot-install] callback server error: {}", e);
            }
        });
        ready_rx
            .await
            .map_err(|_| anyhow!("install callback server failed to start"))??;
        Ok(cb_rx)
    }

    async fn wait_for_callback(
        cb_rx: oneshot::Receiver<InstallCallbackResult>,
    ) -> Result<InstallCallback> {
        match timeout(INSTALL_TIMEOUT, cb_rx).await {
            Ok(Ok(Ok(cb))) => Ok(cb),
            Ok(Ok(Err(e))) => Err(anyhow!(e)),
            Ok(Err(_)) => Err(anyhow!("install callback channel closed unexpectedly")),
            Err(_) => Err(anyhow!("install timeout - please try again")),
        }
    }

    pub fn shutdown(&mut self) {
        if let Some(tx) = self.server_shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}

impl Default for GooseBotInstallFlow {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for GooseBotInstallFlow {
    fn drop(&mut self) {
        self.shutdown();
    }
}
