use serde_json::Value;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::{BatchManager, ObservationLayer};

pub(crate) type CapturedEvents = Arc<Mutex<Vec<(String, Value)>>>;

struct CapturingBatchManager {
    events: CapturedEvents,
}

impl BatchManager for CapturingBatchManager {
    fn add_event(&mut self, event_type: &str, body: Value) {
        self.events
            .lock()
            .unwrap()
            .push((event_type.to_string(), body));
    }

    fn send(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }

    fn is_empty(&self) -> bool {
        self.events.lock().unwrap().is_empty()
    }
}

pub(crate) fn capturing_layer() -> (ObservationLayer, CapturedEvents) {
    let events: CapturedEvents = Arc::new(Mutex::new(Vec::new()));
    let layer = ObservationLayer::new(Arc::new(tokio::sync::Mutex::new(CapturingBatchManager {
        events: events.clone(),
    })));
    (layer, events)
}

pub(crate) async fn wait_for_closed_generation(events: &CapturedEvents) -> Vec<(String, Value)> {
    tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            let captured = events.lock().unwrap().clone();
            if captured.iter().any(|(event_type, body)| {
                event_type == "generation-update" && body.get("endTime").is_some()
            }) {
                return captured;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("generation should close")
}
