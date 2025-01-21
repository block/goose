use chrono::Utc;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::field::{Field, Visit};
use tracing::{span, Event, Id, Level, Metadata, Subscriber};
use tracing_subscriber::layer::Context;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Layer;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct SpanData {
    pub observation_id: String, // Langfuse requires ids to be UUID v4 strings
    pub name: String,
    pub start_time: String,
    pub level: String,
    pub metadata: serde_json::Map<String, Value>,
    pub parent_span_id: Option<u64>,
}

pub fn map_level(level: &Level) -> &'static str {
    match *level {
        Level::ERROR => "ERROR",
        Level::WARN => "WARNING",
        Level::INFO => "DEFAULT",
        Level::DEBUG => "DEBUG",
        Level::TRACE => "DEBUG",
    }
}

pub fn flatten_metadata(
    metadata: serde_json::Map<String, Value>,
) -> serde_json::Map<String, Value> {
    let mut flattened = serde_json::Map::new();
    for (key, value) in metadata {
        match value {
            Value::String(s) => {
                flattened.insert(key, json!(s));
            }
            Value::Object(mut obj) => {
                if let Some(text) = obj.remove("text") {
                    flattened.insert(key, text);
                } else {
                    flattened.insert(key, json!(obj));
                }
            }
            _ => {
                flattened.insert(key, value);
            }
        }
    }
    flattened
}

pub trait BatchManager: Send + Sync + 'static {
    fn add_event(&mut self, event_type: &str, body: Value);
    fn send(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    fn is_empty(&self) -> bool;
}

#[derive(Debug)]
pub struct SpanTracker {
    active_spans: HashMap<u64, String>, // span_id -> observation_id. span_id in Tracing is u64 whereas Langfuse requires UUID v4 strings
    current_trace_id: Option<String>,
}

impl Default for SpanTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl SpanTracker {
    pub fn new() -> Self {
        Self {
            active_spans: HashMap::new(),
            current_trace_id: None,
        }
    }

    pub fn add_span(&mut self, span_id: u64, observation_id: String) {
        self.active_spans.insert(span_id, observation_id);
    }

    pub fn get_span(&self, span_id: u64) -> Option<&String> {
        self.active_spans.get(&span_id)
    }

    pub fn remove_span(&mut self, span_id: u64) -> Option<String> {
        self.active_spans.remove(&span_id)
    }
}

#[derive(Clone)]
pub struct ObservationLayer {
    pub batch_manager: Arc<Mutex<dyn BatchManager>>,
    pub span_tracker: Arc<Mutex<SpanTracker>>,
}

impl ObservationLayer {
    pub async fn handle_span(&self, span_id: u64, span_data: SpanData) {
        let observation_id = span_data.observation_id.clone();

        {
            let mut spans = self.span_tracker.lock().await;
            spans.add_span(span_id, observation_id.clone());
        }

        // Get parent ID if it exists
        let parent_id = if let Some(parent_span_id) = span_data.parent_span_id {
            let spans = self.span_tracker.lock().await;
            spans.get_span(parent_span_id).cloned()
        } else {
            None
        };

        let trace_id = self.ensure_trace_id().await;

        // Create the span observation
        let mut batch = self.batch_manager.lock().await;
        batch.add_event(
            "observation-create",
            json!({
                "id": observation_id,
                "traceId": trace_id,
                "type": "SPAN",
                "name": span_data.name,
                "startTime": span_data.start_time,
                "parentObservationId": parent_id,
                "metadata": span_data.metadata,
                "level": span_data.level
            }),
        );
    }

    pub async fn handle_span_close(&self, span_id: u64) {
        let observation_id = {
            let mut spans = self.span_tracker.lock().await;
            spans.remove_span(span_id)
        };

        if let Some(observation_id) = observation_id {
            let trace_id = self.ensure_trace_id().await;
            let mut batch = self.batch_manager.lock().await;
            batch.add_event(
                "observation-update",
                json!({
                    "id": observation_id,
                    "type": "SPAN",
                    "traceId": trace_id,
                    "endTime": Utc::now().to_rfc3339()
                }),
            );
        }
    }

    pub async fn ensure_trace_id(&self) -> String {
        let mut spans = self.span_tracker.lock().await;
        if let Some(id) = spans.current_trace_id.clone() {
            return id;
        }

        let trace_id = Uuid::new_v4().to_string();
        spans.current_trace_id = Some(trace_id.clone());

        let mut batch = self.batch_manager.lock().await;
        batch.add_event(
            "trace-create",
            json!({
                "id": trace_id,
                "name": Utc::now().timestamp().to_string(),
                "timestamp": Utc::now().to_rfc3339(),
                "input": {},
                "metadata": {},
                "tags": [],
                "public": false
            }),
        );

        trace_id
    }

    pub async fn handle_record(&self, span_id: u64, metadata: serde_json::Map<String, Value>) {
        let observation_id = {
            let spans = self.span_tracker.lock().await;
            spans.get_span(span_id).cloned()
        };

        if let Some(observation_id) = observation_id {
            let trace_id = self.ensure_trace_id().await;

            let mut update = json!({
                "id": observation_id,
                "traceId": trace_id,
                "type": "SPAN"
            });

            // Handle special fields
            if let Some(val) = metadata.get("input") {
                update["input"] = val.clone();
            }

            if let Some(val) = metadata.get("output") {
                update["output"] = val.clone();
            }

            if let Some(val) = metadata.get("model_config") {
                update["metadata"] = json!({ "model_config": val });
            }

            // Handle any remaining metadata
            let remaining_metadata: serde_json::Map<String, Value> = metadata
                .iter()
                .filter(|(k, _)| !["input", "output", "model_config"].contains(&k.as_str()))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();

            if !remaining_metadata.is_empty() {
                let flattened = flatten_metadata(remaining_metadata);
                if update.get("metadata").is_some() {
                    // If metadata exists (from model_config), merge with it
                    if let Some(obj) = update["metadata"].as_object_mut() {
                        for (k, v) in flattened {
                            obj.insert(k, v);
                        }
                    }
                } else {
                    // Otherwise set it directly
                    update["metadata"] = json!(flattened);
                }
            }

            let mut batch = self.batch_manager.lock().await;
            batch.add_event("span-update", update);
        }
    }
}

impl<S> Layer<S> for ObservationLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn enabled(&self, metadata: &Metadata<'_>, _ctx: Context<'_, S>) -> bool {
        metadata.target().starts_with("goose::")
    }

    fn on_new_span(&self, attrs: &span::Attributes<'_>, id: &span::Id, ctx: Context<'_, S>) {
        let span_id = id.into_u64();

        let parent_span_id = ctx
            .span_scope(id)
            .and_then(|mut scope| scope.nth(1))
            .map(|parent| parent.id().into_u64());

        let mut visitor = JsonVisitor::new();
        attrs.record(&mut visitor);

        let span_data = SpanData {
            observation_id: Uuid::new_v4().to_string(),
            name: attrs.metadata().name().to_string(),
            start_time: Utc::now().to_rfc3339(),
            level: map_level(attrs.metadata().level()).to_owned(),
            metadata: visitor.recorded_fields,
            parent_span_id,
        };

        let layer = self.clone();
        tokio::spawn(async move { layer.handle_span(span_id, span_data).await });
    }

    fn on_close(&self, id: Id, _ctx: Context<'_, S>) {
        let span_id = id.into_u64();
        let layer = self.clone();
        tokio::spawn(async move { layer.handle_span_close(span_id).await });
    }

    fn on_record(&self, span: &Id, values: &span::Record<'_>, _ctx: Context<'_, S>) {
        let span_id = span.into_u64();
        let mut visitor = JsonVisitor::new();
        values.record(&mut visitor);
        let metadata = visitor.recorded_fields;

        if !metadata.is_empty() {
            let layer = self.clone();
            tokio::spawn(async move { layer.handle_record(span_id, metadata).await });
        }
    }

    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        let mut visitor = JsonVisitor::new();
        event.record(&mut visitor);
        let metadata = visitor.recorded_fields;

        if let Some(span_id) = ctx.lookup_current().map(|span| span.id().into_u64()) {
            let layer = self.clone();
            tokio::spawn(async move { layer.handle_record(span_id, metadata).await });
        }
    }
}

#[derive(Debug)]
struct JsonVisitor {
    recorded_fields: serde_json::Map<String, Value>,
}

impl JsonVisitor {
    fn new() -> Self {
        Self {
            recorded_fields: serde_json::Map::new(),
        }
    }

    fn insert_value(&mut self, field: &Field, value: Value) {
        self.recorded_fields.insert(field.name().to_string(), value);
    }
}

macro_rules! record_field {
    ($fn_name:ident, $type:ty) => {
        fn $fn_name(&mut self, field: &Field, value: $type) {
            self.insert_value(field, Value::from(value));
        }
    };
}

impl Visit for JsonVisitor {
    record_field!(record_i64, i64);
    record_field!(record_u64, u64);
    record_field!(record_bool, bool);
    record_field!(record_str, &str);

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.insert_value(field, Value::String(format!("{:?}", value)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::sync::mpsc;
    use tracing::dispatcher;

    struct TestFixture {
        original_subscriber: Option<dispatcher::Dispatch>,
        events: Option<Arc<Mutex<Vec<(String, Value)>>>>,
    }

    impl TestFixture {
        fn new() -> Self {
            Self {
                original_subscriber: Some(dispatcher::get_default(dispatcher::Dispatch::clone)),
                events: None,
            }
        }

        fn with_test_layer(mut self) -> (Self, ObservationLayer) {
            let events = Arc::new(Mutex::new(Vec::new()));
            let mock_manager = MockBatchManager::new(events.clone());

            let layer = ObservationLayer {
                batch_manager: Arc::new(Mutex::new(mock_manager)),
                span_tracker: Arc::new(Mutex::new(SpanTracker::new())),
            };

            self.events = Some(events);
            (self, layer)
        }

        async fn get_events(&self) -> Vec<(String, Value)> {
            self.events
                .as_ref()
                .expect("Events not initialized")
                .lock()
                .await
                .clone()
        }
    }

    impl Drop for TestFixture {
        fn drop(&mut self) {
            if let Some(subscriber) = &self.original_subscriber {
                let _ = dispatcher::set_global_default(subscriber.clone());
            }
        }
    }

    struct MockBatchManager {
        events: Arc<Mutex<Vec<(String, Value)>>>,
        sender: mpsc::UnboundedSender<(String, Value)>,
    }

    impl MockBatchManager {
        fn new(events: Arc<Mutex<Vec<(String, Value)>>>) -> Self {
            let (sender, mut receiver) = mpsc::unbounded_channel();
            let events_clone = events.clone();

            tokio::spawn(async move {
                while let Some((event_type, body)) = receiver.recv().await {
                    events_clone.lock().await.push((event_type, body));
                }
            });

            Self { events, sender }
        }
    }

    impl BatchManager for MockBatchManager {
        fn add_event(&mut self, event_type: &str, body: Value) {
            self.sender
                .send((event_type.to_string(), body))
                .expect("Failed to send event");
        }

        fn send(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            Ok(())
        }

        fn is_empty(&self) -> bool {
            futures::executor::block_on(async { self.events.lock().await.is_empty() })
        }
    }

    fn create_test_span_data() -> SpanData {
        SpanData {
            observation_id: Uuid::new_v4().to_string(),
            name: "test_span".to_string(),
            start_time: Utc::now().to_rfc3339(),
            level: "DEFAULT".to_string(),
            metadata: serde_json::Map::new(),
            parent_span_id: None,
        }
    }

    const TEST_WAIT_DURATION: Duration = Duration::from_secs(6);

    #[tokio::test]
    async fn test_span_creation() {
        let (fixture, layer) = TestFixture::new().with_test_layer();
        let span_id = 1u64;
        let span_data = create_test_span_data();

        layer.handle_span(span_id, span_data.clone()).await;
        tokio::time::sleep(TEST_WAIT_DURATION).await;

        let events = fixture.get_events().await;
        assert_eq!(events.len(), 2); // trace-create and observation-create

        let (event_type, body) = &events[1];
        assert_eq!(event_type, "observation-create");
        assert_eq!(body["id"], span_data.observation_id);
        assert_eq!(body["name"], "test_span");
        assert_eq!(body["type"], "SPAN");
    }

    #[tokio::test]
    async fn test_span_close() {
        let (fixture, layer) = TestFixture::new().with_test_layer();
        let span_id = 1u64;
        let span_data = create_test_span_data();

        layer.handle_span(span_id, span_data.clone()).await;
        layer.handle_span_close(span_id).await;
        tokio::time::sleep(TEST_WAIT_DURATION).await;

        let events = fixture.get_events().await;
        assert_eq!(events.len(), 3); // trace-create, observation-create, observation-update

        let (event_type, body) = &events[2];
        assert_eq!(event_type, "observation-update");
        assert_eq!(body["id"], span_data.observation_id);
        assert!(body["endTime"].as_str().is_some());
    }

    #[tokio::test]
    async fn test_record_handling() {
        let (fixture, layer) = TestFixture::new().with_test_layer();
        let span_id = 1u64;
        let span_data = create_test_span_data();

        layer.handle_span(span_id, span_data.clone()).await;

        let mut metadata = serde_json::Map::new();
        metadata.insert("input".to_string(), json!("test input"));
        metadata.insert("output".to_string(), json!("test output"));
        metadata.insert("custom_field".to_string(), json!("custom value"));

        layer.handle_record(span_id, metadata).await;
        tokio::time::sleep(TEST_WAIT_DURATION).await;

        let events = fixture.get_events().await;
        assert_eq!(events.len(), 3); // trace-create, observation-create, span-update

        let (event_type, body) = &events[2];
        assert_eq!(event_type, "span-update");
        assert_eq!(body["input"], "test input");
        assert_eq!(body["output"], "test output");
        assert_eq!(body["metadata"]["custom_field"], "custom value");
    }

    #[test]
    fn test_flatten_metadata() {
        let _fixture = TestFixture::new();
        let mut metadata = serde_json::Map::new();
        metadata.insert("simple".to_string(), json!("value"));
        metadata.insert(
            "complex".to_string(),
            json!({
                "text": "inner value"
            }),
        );

        let flattened = flatten_metadata(metadata);
        assert_eq!(flattened["simple"], "value");
        assert_eq!(flattened["complex"], "inner value");
    }
}
