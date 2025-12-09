//! PostHog telemetry - fires once per session creation.

use crate::config::Config;
use once_cell::sync::Lazy;
use std::sync::atomic::{AtomicBool, Ordering};

const POSTHOG_API_KEY: &str = "phc_RyX5CaY01VtZJCQyhSR5KFh6qimUy81YwxsEpotAftT";

static TELEMETRY_DISABLED: Lazy<AtomicBool> = Lazy::new(|| {
    std::env::var("GOOSE_TELEMETRY_OFF")
        .map(|v| v == "1" || v.to_lowercase() == "true")
        .unwrap_or(false)
        .into()
});

pub fn emit_session_started() {
    if TELEMETRY_DISABLED.load(Ordering::Relaxed) {
        return;
    }

    tokio::spawn(async {
        let _ = send_session_event().await;
    });
}

async fn send_session_event() -> Result<(), String> {
    let client = posthog_rs::client(POSTHOG_API_KEY).await;
    let mut event = posthog_rs::Event::new("session_started", "goose_user");

    event.insert_prop("os", std::env::consts::OS).ok();
    event.insert_prop("arch", std::env::consts::ARCH).ok();
    event.insert_prop("version", env!("CARGO_PKG_VERSION")).ok();

    let config = Config::global();
    if let Ok(provider) = config.get_param::<String>("GOOSE_PROVIDER") {
        event.insert_prop("provider", provider).ok();
    }
    if let Ok(model) = config.get_param::<String>("GOOSE_MODEL") {
        event.insert_prop("model", model).ok();
    }

    client.capture(event).await.map_err(|e| format!("{:?}", e))
}
