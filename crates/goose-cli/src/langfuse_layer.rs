use chrono::Utc;
use serde_json::{json, Value};
use tracing::{span, Id, Subscriber, Level, Metadata, Event};
use tracing_subscriber::Layer;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::time::Duration;
use uuid::Uuid;
use std::env;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use tracing_subscriber::registry::LookupSpan;
use tracing::field::{Field, Visit};
use reqwest::{Client, StatusCode};
use tracing_subscriber::layer::Context;
use std::future::Future;


const DEFAULT_LANGFUSE_URL: &str = "http://localhost:3000";

#[derive(Debug, Serialize, Deserialize)]
struct IngestionResponse {
    successes: Vec<IngestionSuccess>,
    errors: Vec<IngestionError>,
}

#[derive(Debug, Serialize, Deserialize)]
struct IngestionSuccess {
    id: String,
    status: i32,
}

#[derive(Debug, Serialize, Deserialize)]
struct IngestionError {
    id: String,
    status: i32,
    message: Option<String>,
    error: Option<Value>,
}

#[derive(Debug)]
struct SpanData {
    observation_id: String, // Langfuse requires ids to be UUID v4 strings 
    name: String,
    start_time: String,
    level: String,
    metadata: serde_json::Map<String, Value>,
    parent_span_id: Option<u64>,
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

#[derive(Debug, Clone)]
struct BatchManager {
    batch: Vec<Value>,
    client: Client,
    base_url: String,
    public_key: String,
    secret_key: String,
}

impl BatchManager {
    fn new(public_key: String, secret_key: String, base_url: String) -> Self {
        Self {
            batch: Vec::new(),
            client: Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .expect("Failed to create HTTP client"),
            base_url,
            public_key,
            secret_key,
        }
    }

    // Changed to static method
    fn spawn_sender(manager: Arc<Mutex<Self>>) {
        const BATCH_INTERVAL: Duration = Duration::from_secs(5);
        
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(BATCH_INTERVAL).await;
                if let Err(e) = manager.lock().await.send().await {
                    tracing::error!(
                        error.msg = %e,
                        error.type = %std::any::type_name_of_val(&e),
                        "Failed to send batch to Langfuse"
                    );
                }
            }
        });
    }

    fn flatten_metadata(metadata: serde_json::Map<String, Value>) -> serde_json::Map<String, Value> {
        let mut flattened = serde_json::Map::new();
        for (key, value) in metadata {
            match value {
                Value::String(s) => {
                    flattened.insert(key, json!(s));
                },
                Value::Object(mut obj) => {
                    if let Some(text) = obj.remove("text") {
                        flattened.insert(key, text);
                    } else {
                        flattened.insert(key, json!(obj));
                    }
                },
                _ => {
                    flattened.insert(key, value);
                }
            }
        }
        flattened
    }

    async fn add_event(&mut self, event_type: &str, body: Value) {
        self.batch.push(json!({
            "id": Uuid::new_v4().to_string(),
            "timestamp": Utc::now().to_rfc3339(),
            "type": event_type,
            "body": body
        }));
    }

    async fn send(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if self.batch.is_empty() {
            return Ok(());
        }

        let payload = json!({ "batch": self.batch });
        let url = format!("{}/api/public/ingestion", self.base_url);

        let response = self.client
            .post(&url)
            .basic_auth(&self.public_key, Some(&self.secret_key))
            .json(&payload)
            .send()
            .await?;
        
        match response.status() {
            status if status.is_success() => {
                let response_body: IngestionResponse = response.json().await?;
                
                for error in &response_body.errors {
                    tracing::error!(
                        id = %error.id,
                        status = error.status,
                        message = error.message.as_deref().unwrap_or("No message"),
                        error = ?error.error,
                        "Partial failure in batch ingestion"
                    );
                }
        
                if !response_body.successes.is_empty() {
                    self.batch.clear();
                }
        
                if response_body.successes.is_empty() && !response_body.errors.is_empty() {
                    Err("Langfuse ingestion failed for all items".into())
                } else {
                    Ok(())
                }
            }
            status @ (StatusCode::BAD_REQUEST | StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN | 
                     StatusCode::NOT_FOUND | StatusCode::METHOD_NOT_ALLOWED) => {
                let err_text = response.text().await.unwrap_or_default();
                Err(format!("Langfuse API error: {}: {}", status, err_text).into())
            }
            status => {
                let err_text = response.text().await.unwrap_or_default();
                Err(format!("Unexpected status code: {}: {}", status, err_text).into())
            }
        }
    }
}

#[derive(Debug)]
struct SpanTracker {
    active_spans: HashMap<u64, String>, // span_id -> observation_id. span_id in Tracing is u64 whereas Langfuse requires UUID v4 strings
    current_trace_id: Option<String>,
}

impl SpanTracker {
    fn new() -> Self {
        Self {
            active_spans: HashMap::new(),
            current_trace_id: None,
        }
    }

    fn add_span(&mut self, span_id: u64, observation_id: String) {
        self.active_spans.insert(span_id, observation_id);
    }

    fn get_span(&self, span_id: u64) -> Option<&String> {
        self.active_spans.get(&span_id)
    }

    fn remove_span(&mut self, span_id: u64) -> Option<String> {
        self.active_spans.remove(&span_id)
    }
}

#[derive(Debug, Clone)]
pub struct LangfuseLayer {
    batch_manager: Arc<Mutex<BatchManager>>,
    span_tracker: Arc<Mutex<SpanTracker>>,
}

impl LangfuseLayer {
    pub fn new(public_key: String, secret_key: String) -> Self {
        let base_url = env::var("LANGFUSE_URL")
            .unwrap_or_else(|_| DEFAULT_LANGFUSE_URL.to_string());
            
        let batch_manager = Arc::new(Mutex::new(
            BatchManager::new(public_key, secret_key, base_url)
        ));
        
        BatchManager::spawn_sender(batch_manager.clone());

        Self {
            batch_manager,
            span_tracker: Arc::new(Mutex::new(SpanTracker::new())),
        }
    }

    fn map_level(level: &Level) -> &'static str {
        match *level {
            Level::ERROR => "ERROR",
            Level::WARN => "WARNING",
            Level::INFO => "DEFAULT",
            Level::DEBUG => "DEBUG",
            Level::TRACE => "DEBUG",
        }
    }

    fn spawn_task<F, Fut>(&self, f: F)
    where
        F: FnOnce(Self) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let layer = self.clone();
        tokio::spawn(async move { f(layer).await });
    }

    // Core span handling methods
    async fn handle_span(&self, span_id: u64, span_data: SpanData) {
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
        batch.add_event("observation-create", json!({
            "id": observation_id,
            "traceId": trace_id,
            "type": "SPAN",
            "name": span_data.name,
            "startTime": span_data.start_time,
            "parentObservationId": parent_id,
            "metadata": span_data.metadata,
            "level": span_data.level
        })).await;
    }

    async fn handle_span_close(&self, span_id: u64) {
        let observation_id = {
            let mut spans = self.span_tracker.lock().await;
            spans.remove_span(span_id)
        };

        if let Some(observation_id) = observation_id {
            let trace_id = self.ensure_trace_id().await;
            let mut batch = self.batch_manager.lock().await;
            batch.add_event("observation-update", json!({
                "id": observation_id,
                "type": "SPAN",
                "traceId": trace_id,
                "endTime": Utc::now().to_rfc3339()
            })).await;
        }
    }

    async fn ensure_trace_id(&self) -> String {
        let mut spans = self.span_tracker.lock().await;
        if let Some(id) = spans.current_trace_id.clone() {
            return id;
        }

        let trace_id = Uuid::new_v4().to_string();
        spans.current_trace_id = Some(trace_id.clone());

        let mut batch = self.batch_manager.lock().await;
        batch.add_event("trace-create", json!({
            "id": trace_id,
            "name": Utc::now().timestamp().to_string(),
            "timestamp": Utc::now().to_rfc3339(),
            "input": {},
            "metadata": {},
            "tags": [],
            "public": false
        })).await;

        trace_id
    }

    async fn handle_record(&self, span_id: u64, metadata: serde_json::Map<String, Value>) {
        let observation_id = {
            let spans = self.span_tracker.lock().await;
            spans.get_span(span_id).cloned()
        };
    
        if let Some(observation_id) = observation_id {
            let trace_id = self.ensure_trace_id().await;
    
            if let Some(val) = metadata.get("input") {
                let mut batch = self.batch_manager.lock().await;
                batch.add_event("span-update", json!({
                    "id": observation_id,
                    "traceId": trace_id,
                    "type": "SPAN",
                    "input": val
                })).await;
                return;
            }
            
            if let Some(val) = metadata.get("output") {
                let mut batch = self.batch_manager.lock().await;
                batch.add_event("span-update", json!({
                    "id": observation_id,
                    "traceId": trace_id,
                    "type": "SPAN",
                    "output": val
                })).await;
                return;
            }
    
            // Handle other metadata
            let flattened_metadata = BatchManager::flatten_metadata(metadata);
            let mut batch = self.batch_manager.lock().await;
            batch.add_event("observation-update", json!({
                "id": observation_id,
                "traceId": trace_id,
                "metadata": flattened_metadata,
                "type": "SPAN"
            })).await;
        }
    }
}

impl<S> Layer<S> for LangfuseLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn enabled(&self, metadata: &Metadata<'_>, _ctx: Context<'_, S>) -> bool {
        metadata.target().starts_with("goose::")
    }

    fn on_new_span(
        &self,
        attrs: &span::Attributes<'_>,
        id: &span::Id,
        ctx: Context<'_, S>,
    ) {
        let span_id = id.into_u64();
        
        let parent_span_id = ctx.span_scope(id)
            .and_then(|scope| scope.skip(1).next())
            .map(|parent| parent.id().into_u64());
    
        let mut visitor = JsonVisitor::new();
        attrs.record(&mut visitor);
    
        let span_data = SpanData {
            observation_id: Uuid::new_v4().to_string(),
            name: attrs.metadata().name().to_string(),
            start_time: Utc::now().to_rfc3339(),
            level: Self::map_level(attrs.metadata().level()).to_owned(),
            metadata: visitor.recorded_fields,
            parent_span_id,
        };
    
        self.spawn_task(move |layer| async move {
            layer.handle_span(span_id, span_data).await
        });
    }

    fn on_close(&self, id: Id, _ctx: Context<'_, S>) {
        let span_id = id.into_u64();
        self.spawn_task(move |layer| async move {
            layer.handle_span_close(span_id).await
        });
    }

    fn on_record(&self, span: &Id, values: &span::Record<'_>, _ctx: Context<'_, S>) {
        let span_id = span.into_u64();
        let mut visitor = JsonVisitor::new();
        values.record(&mut visitor);
        let metadata = visitor.recorded_fields;

        if !metadata.is_empty() {
            self.spawn_task(move |layer| async move {
                layer.handle_record(span_id, metadata).await
            });
        }
    }

    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        let mut visitor = JsonVisitor::new();
        event.record(&mut visitor);
        let metadata = visitor.recorded_fields;
        
        if let Some(span_id) = ctx.lookup_current().map(|span| span.id().into_u64()) {
            self.spawn_task(move |layer| async move {
                layer.handle_record(span_id, metadata).await
            });
        }
    }

    fn on_enter(&self, span: &Id, _ctx: Context<'_, S>) {
        let span_id = span.into_u64();
        
        self.spawn_task(move |layer| async move {
            if let Some(observation_id) = layer.span_tracker.lock().await.get_span(span_id).cloned() {
                let mut batch = layer.batch_manager.lock().await;
                batch.add_event("span-enter", json!({
                    "id": observation_id
                })).await;
            }
        });
    }

    fn on_exit(&self, span: &Id, _ctx: Context<'_, S>) {
        let span_id = span.into_u64();
        
        self.spawn_task(move |layer| async move {
            if let Some(observation_id) = layer.span_tracker.lock().await.get_span(span_id).cloned() {
                let mut batch = layer.batch_manager.lock().await;
                batch.add_event("span-exit", json!({
                    "id": observation_id
                })).await;
            }
        });
    }
}

pub fn create_langfuse_layer() -> Option<LangfuseLayer> {
    let public_key = env::var("LANGFUSE_PUBLIC_KEY")
        .or_else(|_| env::var("LANGFUSE_INIT_PROJECT_PUBLIC_KEY"))
        .unwrap_or_else(|_| "publickey-local".to_string());
    
    let secret_key = env::var("LANGFUSE_SECRET_KEY")
        .or_else(|_| env::var("LANGFUSE_INIT_PROJECT_SECRET_KEY"))
        .unwrap_or_else(|_| "secretkey-local".to_string());

    Some(LangfuseLayer::new(public_key, secret_key))
}