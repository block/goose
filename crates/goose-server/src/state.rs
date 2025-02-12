use anyhow::Result;
use goose::agents::Agent;
use std::sync::Arc;
use tokio::sync::Mutex;
use serde_yaml::Value;
use std::collections::HashMap;

/// Shared application state
#[allow(dead_code)]
#[derive(Clone)]
pub struct AppState {
    pub agent: Arc<Mutex<Option<Box<dyn Agent>>>>,
    pub secret_key: String,
    pub config: Arc<Mutex<HashMap<String, Value>>>,
}

impl AppState {
    pub async fn new(secret_key: String) -> Result<Self> {
        // Initialize the config as an empty HashMap
        let config = Arc::new(Mutex::new(HashMap::<String, Value>::new()));

        Ok(Self {
            agent: Arc::new(Mutex::new(None)),
            secret_key,
            config,
        })
    }
}
