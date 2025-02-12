use anyhow::Result;
use goose::agents::Agent;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Shared application state
#[allow(dead_code)]
#[derive(Clone)]
pub struct AppState {
    pub agent: Arc<Mutex<Option<Box<dyn Agent>>>>,
    pub secret_key: String,
}

impl AppState {
    pub async fn new(secret_key: String) -> Result<Self> {
        Ok(Self {
            agent: Arc::new(Mutex::new(None)),
            secret_key,
        })
    }
}
