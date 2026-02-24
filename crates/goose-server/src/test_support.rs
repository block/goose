use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Result;
use axum::middleware;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

use crate::auth::check_token;
use crate::routes;
use crate::state::AppState;

pub struct TestServerHandle {
    pub base_url: String,
    pub secret_key: String,
    pub state: Arc<AppState>,
    shutdown_tx: Option<oneshot::Sender<()>>,
    join_handle: Option<JoinHandle<()>>,
}

impl TestServerHandle {
    pub async fn shutdown(mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        if let Some(handle) = self.join_handle.take() {
            let _ = handle.await;
        }
    }
}

impl Drop for TestServerHandle {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        if let Some(handle) = self.join_handle.take() {
            handle.abort();
        }
    }
}

/// Spawn an in-process goosed server bound to an ephemeral port.
///
/// This is intended for integration tests that want a hermetic goosed instance
/// without spawning a separate process.
pub async fn spawn_test_server() -> Result<TestServerHandle> {
    let secret_key = "test-secret".to_string();
    let state = AppState::new().await?;

    let app = routes::configure(Arc::clone(&state), secret_key.clone()).layer(
        middleware::from_fn_with_state(secret_key.clone(), check_token),
    );

    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr: SocketAddr = listener.local_addr()?;

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    let join_handle = tokio::spawn(async move {
        let serve = axum::serve(listener, app).with_graceful_shutdown(async move {
            let _ = shutdown_rx.await;
        });
        let _ = serve.await;
    });

    Ok(TestServerHandle {
        base_url: format!("http://{}", addr),
        secret_key,
        state,
        shutdown_tx: Some(shutdown_tx),
        join_handle: Some(join_handle),
    })
}
