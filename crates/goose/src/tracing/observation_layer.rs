use chrono::Utc;
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex as StdMutex};
use tokio::sync::{mpsc, Mutex};
use tracing::field::{Field, Visit};
use tracing::{span, Event, Id, Level, Metadata, Subscriber};
use tracing_subscriber::layer::Context;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Layer;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct SpanData {
    pub observation_id: String,
    pub name: String,
    pub start_time: String,
    pub level: String,
    pub metadata: Map<String, Value>,
    pub parent_span_id: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ObservationType {
    Span,
    Generation,
}

impl ObservationType {
    fn from_value(value: Option<Value>) -> Self {
        match value.as_ref().and_then(Value::as_str) {
            Some(value) if value.eq_ignore_ascii_case("GENERATION") => Self::Generation,
            _ => Self::Span,
        }
    }

    fn create_event(self) -> &'static str {
        match self {
            Self::Span => "span-create",
            Self::Generation => "generation-create",
        }
    }

    fn update_event(self) -> &'static str {
        match self {
            Self::Span => "span-update",
            Self::Generation => "generation-update",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TraceFieldTarget {
    Trace,
    Observation,
}

#[derive(Debug, Clone)]
struct ObservationContext {
    observation_id: String,
    trace_id: String,
    observation_type: ObservationType,
    trace_field_target: TraceFieldTarget,
}

#[derive(Debug)]
struct NewObservation {
    context: ObservationContext,
    parent_observation_id: Option<String>,
    name: String,
    start_time: String,
    level: String,
    fields: Map<String, Value>,
    creates_trace: bool,
}

pub fn map_level(level: &Level) -> &'static str {
    match *level {
        Level::ERROR => "ERROR",
        Level::WARN => "WARNING",
        Level::INFO => "DEFAULT",
        Level::DEBUG | Level::TRACE => "DEBUG",
    }
}

pub fn flatten_metadata(metadata: Map<String, Value>) -> Map<String, Value> {
    let mut flattened = Map::new();
    for (key, value) in metadata {
        match value {
            Value::Object(mut object) => {
                if let Some(text) = object.remove("text") {
                    flattened.insert(key, text);
                } else {
                    flattened.insert(key, Value::Object(object));
                }
            }
            value => {
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
    active_spans: HashMap<u64, String>,
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

type QueuedEvent = (&'static str, Value);
static ACTIVE_OBSERVATION_LAYERS: AtomicUsize = AtomicUsize::new(0);

struct ActiveObservationLayer;

impl Drop for ActiveObservationLayer {
    fn drop(&mut self) {
        ACTIVE_OBSERVATION_LAYERS.fetch_sub(1, Ordering::Relaxed);
    }
}

pub(crate) fn is_observation_layer_active() -> bool {
    ACTIVE_OBSERVATION_LAYERS.load(Ordering::Relaxed) > 0
}

#[derive(Clone)]
pub struct ObservationLayer {
    pub batch_manager: Arc<Mutex<dyn BatchManager>>,
    pub span_tracker: Arc<Mutex<SpanTracker>>,
    event_sender: mpsc::UnboundedSender<QueuedEvent>,
    idle_receiver: Arc<StdMutex<Option<mpsc::UnboundedReceiver<QueuedEvent>>>>,
    _activity: Arc<ActiveObservationLayer>,
}

impl ObservationLayer {
    pub fn new(batch_manager: Arc<Mutex<dyn BatchManager>>) -> Self {
        let (event_sender, event_receiver) = mpsc::unbounded_channel::<QueuedEvent>();
        ACTIVE_OBSERVATION_LAYERS.fetch_add(1, Ordering::Relaxed);

        let layer = Self {
            batch_manager,
            span_tracker: Arc::new(Mutex::new(SpanTracker::new())),
            event_sender,
            idle_receiver: Arc::new(StdMutex::new(Some(event_receiver))),
            _activity: Arc::new(ActiveObservationLayer),
        };
        layer.start_worker();
        layer
    }

    /// Callers such as `create_langfuse_observer` build the layer from
    /// synchronous logging setup, so there may be no runtime to spawn on yet.
    /// The channel is unbounded, so events queued before the worker starts are
    /// delivered once tracing reaches the layer from inside a runtime.
    fn start_worker(&self) {
        let mut idle_receiver = self.idle_receiver.lock().unwrap();
        let Some(mut event_receiver) = idle_receiver.take() else {
            return;
        };
        let Ok(handle) = tokio::runtime::Handle::try_current() else {
            *idle_receiver = Some(event_receiver);
            return;
        };

        let batch_manager = self.batch_manager.clone();
        handle.spawn(async move {
            while let Some((event_type, body)) = event_receiver.recv().await {
                batch_manager.lock().await.add_event(event_type, body);
            }
        });
    }

    fn enqueue(&self, event_type: &'static str, body: Value) {
        self.start_worker();
        let _ = self.event_sender.send((event_type, body));
    }

    fn enqueue_new_observation(&self, mut observation: NewObservation) {
        let trace_input = take_value(&mut observation.fields, &["trace_input"]);
        let trace_output = take_value(&mut observation.fields, &["trace_output"]);

        if observation.context.trace_field_target == TraceFieldTarget::Observation {
            if let Some(input) = trace_input {
                observation.fields.insert("input".to_string(), input);
            }
            if let Some(output) = trace_output {
                observation.fields.insert("output".to_string(), output);
            }
        } else if observation.creates_trace {
            let mut trace_body = json!({
                "id": observation.context.trace_id,
                "name": observation.name,
                "timestamp": observation.start_time,
                "metadata": {},
                "tags": [],
                "public": false
            });

            if let Some(session_id) = observation
                .fields
                .get("session.id")
                .and_then(value_as_string)
            {
                trace_body["sessionId"] = Value::String(session_id);
            }
            if let Some(input) = trace_input {
                trace_body["input"] = input;
            }
            if let Some(output) = trace_output {
                trace_body["output"] = output;
            }

            self.enqueue("trace-create", trace_body);
        } else if trace_input.is_some() || trace_output.is_some() {
            let mut trace_update = json!({ "id": observation.context.trace_id });
            if let Some(input) = trace_input {
                trace_update["input"] = input;
            }
            if let Some(output) = trace_output {
                trace_update["output"] = output;
            }
            self.enqueue("trace-create", trace_update);
        }

        let body = observation_body(
            &observation.context,
            observation.parent_observation_id,
            observation.name,
            observation.start_time,
            observation.level,
            &mut observation.fields,
        );
        self.enqueue(observation.context.observation_type.create_event(), body);
    }

    fn enqueue_close(&self, context: ObservationContext) {
        self.enqueue(
            context.observation_type.update_event(),
            json!({
                "id": context.observation_id,
                "traceId": context.trace_id,
                "endTime": Utc::now().to_rfc3339()
            }),
        );
    }

    fn enqueue_record(&self, context: ObservationContext, mut fields: Map<String, Value>) {
        match context.trace_field_target {
            TraceFieldTarget::Trace => {
                let mut trace_update = json!({ "id": context.trace_id });
                let mut has_trace_update = false;

                if let Some(input) = take_value(&mut fields, &["trace_input"]) {
                    trace_update["input"] = input;
                    has_trace_update = true;
                }
                if let Some(output) = take_value(&mut fields, &["trace_output"]) {
                    trace_update["output"] = output;
                    has_trace_update = true;
                }
                if has_trace_update {
                    self.enqueue("trace-create", trace_update);
                }
            }
            TraceFieldTarget::Observation => {
                if let Some(input) = take_value(&mut fields, &["trace_input"]) {
                    fields.insert("input".to_string(), input);
                }
                if let Some(output) = take_value(&mut fields, &["trace_output"]) {
                    fields.insert("output".to_string(), output);
                }
            }
        }

        remove_internal_fields(&mut fields);
        if fields.is_empty() {
            return;
        }

        let mut update = json!({
            "id": context.observation_id,
            "traceId": context.trace_id
        });
        apply_observation_fields(&mut update, &mut fields, context.observation_type);
        if !fields.is_empty() {
            update["metadata"] = Value::Object(flatten_metadata(fields));
        }
        self.enqueue(context.observation_type.update_event(), update);
    }

    pub async fn handle_span(&self, span_id: u64, mut span_data: SpanData) {
        let observation_type =
            ObservationType::from_value(take_value(&mut span_data.metadata, &["observation_type"]));
        let parent_observation_id = if let Some(parent_span_id) = span_data.parent_span_id {
            self.span_tracker
                .lock()
                .await
                .get_span(parent_span_id)
                .cloned()
        } else {
            None
        };
        self.span_tracker
            .lock()
            .await
            .add_span(span_id, span_data.observation_id.clone());
        let trace_id = self.ensure_trace_id().await;
        let context = ObservationContext {
            observation_id: span_data.observation_id,
            trace_id,
            observation_type,
            trace_field_target: TraceFieldTarget::Trace,
        };
        let body = observation_body(
            &context,
            parent_observation_id,
            span_data.name,
            span_data.start_time,
            span_data.level,
            &mut span_data.metadata,
        );
        self.enqueue(observation_type.create_event(), body);
    }

    pub async fn handle_span_close(&self, span_id: u64) {
        let observation_id = self.span_tracker.lock().await.remove_span(span_id);
        if let Some(observation_id) = observation_id {
            self.enqueue_close(ObservationContext {
                observation_id,
                trace_id: self.ensure_trace_id().await,
                observation_type: ObservationType::Span,
                trace_field_target: TraceFieldTarget::Trace,
            });
        }
    }

    pub async fn ensure_trace_id(&self) -> String {
        let mut spans = self.span_tracker.lock().await;
        if let Some(trace_id) = &spans.current_trace_id {
            return trace_id.clone();
        }

        let trace_id = Uuid::new_v4().to_string();
        spans.current_trace_id = Some(trace_id.clone());
        self.enqueue(
            "trace-create",
            json!({
                "id": trace_id,
                "name": Utc::now().timestamp().to_string(),
                "timestamp": Utc::now().to_rfc3339(),
                "metadata": {},
                "tags": [],
                "public": false
            }),
        );
        trace_id
    }
}

fn observation_body(
    context: &ObservationContext,
    parent_observation_id: Option<String>,
    name: String,
    start_time: String,
    level: String,
    fields: &mut Map<String, Value>,
) -> Value {
    remove_internal_fields(fields);
    let mut body = json!({
        "id": context.observation_id,
        "traceId": context.trace_id,
        "name": name,
        "startTime": start_time,
        "level": level
    });
    if let Some(parent_observation_id) = parent_observation_id {
        body["parentObservationId"] = Value::String(parent_observation_id);
    }
    apply_observation_fields(&mut body, fields, context.observation_type);
    if !fields.is_empty() {
        body["metadata"] = Value::Object(flatten_metadata(std::mem::take(fields)));
    }
    body
}

fn apply_observation_fields(
    body: &mut Value,
    fields: &mut Map<String, Value>,
    observation_type: ObservationType,
) {
    for (source_names, target_name) in [
        (&["input"][..], "input"),
        (&["output"][..], "output"),
        (&["status_message", "statusMessage"][..], "statusMessage"),
    ] {
        if let Some(value) = take_value(fields, source_names) {
            body[target_name] = value;
        }
    }

    for (source_names, target_name) in [
        (&["input_json"][..], "input"),
        (&["output_json"][..], "output"),
    ] {
        if let Some(value) = take_value(fields, source_names) {
            body[target_name] = parse_recorded_json(value);
        }
    }

    if observation_type != ObservationType::Generation {
        return;
    }

    for (source_names, target_name) in [
        (&["model"][..], "model"),
        (
            &["completion_start_time", "completionStartTime"][..],
            "completionStartTime",
        ),
    ] {
        if let Some(value) = take_value(fields, source_names) {
            body[target_name] = value;
        }
    }

    for (source_names, target_name) in [
        (
            &[
                "model_parameters_json",
                "model_parameters",
                "modelParameters",
                "model_config",
            ][..],
            "modelParameters",
        ),
        (
            &[
                "usage_details_json",
                "usage_details",
                "usageDetails",
                "usage",
            ][..],
            "usageDetails",
        ),
        (
            &["cost_details_json", "cost_details", "costDetails"][..],
            "costDetails",
        ),
    ] {
        if let Some(value) = take_value(fields, source_names) {
            body[target_name] = parse_recorded_json(value);
        }
    }
}

fn remove_internal_fields(fields: &mut Map<String, Value>) {
    for name in [
        "observation_type",
        "observation_id",
        "trace_id",
        "parent_observation_id",
        "trace_boundary",
        "trace_input",
        "trace_output",
    ] {
        fields.remove(name);
    }
}

fn take_value(fields: &mut Map<String, Value>, names: &[&str]) -> Option<Value> {
    names.iter().find_map(|name| fields.remove(*name))
}

fn value_as_string(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => Some(value.clone()),
        _ => None,
    }
}

fn parse_recorded_json(value: Value) -> Value {
    match value {
        Value::String(value) => serde_json::from_str(&value).unwrap_or(Value::String(value)),
        value => value,
    }
}

impl<S> Layer<S> for ObservationLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn enabled(&self, metadata: &Metadata<'_>, _ctx: Context<'_, S>) -> bool {
        metadata.target().starts_with("goose::")
    }

    fn on_new_span(&self, attrs: &span::Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        let parent_context = ctx
            .span_scope(id)
            .and_then(|mut scope| scope.nth(1))
            .and_then(|parent| parent.extensions().get::<ObservationContext>().cloned());

        let mut visitor = JsonVisitor::new();
        attrs.record(&mut visitor);
        let mut fields = visitor.recorded_fields;

        let observation_type =
            ObservationType::from_value(take_value(&mut fields, &["observation_type"]));
        let trace_boundary = take_value(&mut fields, &["trace_boundary"])
            .and_then(|value| value.as_bool())
            .unwrap_or(false);
        let observation_id = take_value(&mut fields, &["observation_id"])
            .as_ref()
            .and_then(value_as_string)
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        let explicit_trace_id = take_value(&mut fields, &["trace_id"])
            .as_ref()
            .and_then(value_as_string);
        let explicit_parent_id = take_value(&mut fields, &["parent_observation_id"])
            .as_ref()
            .and_then(value_as_string);

        let trace_id = explicit_trace_id
            .or_else(|| {
                parent_context
                    .as_ref()
                    .map(|parent| parent.trace_id.clone())
            })
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        let parent_observation_id = explicit_parent_id.or_else(|| {
            parent_context
                .as_ref()
                .map(|parent| parent.observation_id.clone())
        });
        let context = ObservationContext {
            observation_id,
            trace_id,
            observation_type,
            trace_field_target: if trace_boundary {
                if parent_context.is_some() {
                    TraceFieldTarget::Observation
                } else {
                    TraceFieldTarget::Trace
                }
            } else {
                parent_context
                    .as_ref()
                    .map(|parent| parent.trace_field_target)
                    .unwrap_or(TraceFieldTarget::Trace)
            },
        };

        if let Some(span) = ctx.span(id) {
            span.extensions_mut().insert(context.clone());
        }

        self.enqueue_new_observation(NewObservation {
            context,
            parent_observation_id,
            name: attrs.metadata().name().to_string(),
            start_time: Utc::now().to_rfc3339(),
            level: map_level(attrs.metadata().level()).to_owned(),
            fields,
            creates_trace: parent_context.is_none(),
        });
    }

    fn on_close(&self, id: Id, ctx: Context<'_, S>) {
        let context = ctx
            .span(&id)
            .and_then(|span| span.extensions().get::<ObservationContext>().cloned());
        if let Some(context) = context {
            self.enqueue_close(context);
        }
    }

    fn on_record(&self, span: &Id, values: &span::Record<'_>, ctx: Context<'_, S>) {
        let context = ctx
            .span(span)
            .and_then(|span| span.extensions().get::<ObservationContext>().cloned());
        let Some(context) = context else {
            return;
        };

        let mut visitor = JsonVisitor::new();
        values.record(&mut visitor);
        if !visitor.recorded_fields.is_empty() {
            self.enqueue_record(context, visitor.recorded_fields);
        }
    }

    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        let context = ctx
            .lookup_current()
            .and_then(|span| span.extensions().get::<ObservationContext>().cloned());
        let Some(context) = context else {
            return;
        };

        let mut visitor = JsonVisitor::new();
        event.record(&mut visitor);
        if !visitor.recorded_fields.is_empty() {
            self.enqueue_record(context, visitor.recorded_fields);
        }
    }
}

#[derive(Debug)]
struct JsonVisitor {
    recorded_fields: Map<String, Value>,
}

impl JsonVisitor {
    fn new() -> Self {
        Self {
            recorded_fields: Map::new(),
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
        self.insert_value(field, Value::String(format!("{value:?}")));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tracing::dispatcher::{self, Dispatch};
    use tracing::Instrument;
    use tracing_subscriber::layer::SubscriberExt;

    type Events = Arc<StdMutex<Vec<(String, Value)>>>;

    struct MockBatchManager {
        events: Events,
    }

    impl BatchManager for MockBatchManager {
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

    fn test_layer() -> (ObservationLayer, Events) {
        let events = Arc::new(StdMutex::new(Vec::new()));
        let manager = MockBatchManager {
            events: events.clone(),
        };
        (ObservationLayer::new(Arc::new(Mutex::new(manager))), events)
    }

    #[test]
    fn queues_events_when_constructed_without_a_runtime() {
        let (layer, events) = test_layer();
        layer.enqueue("trace-create", json!({ "id": "queued-before-runtime" }));
        assert!(events.lock().unwrap().is_empty());

        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async move {
                layer.enqueue("trace-create", json!({ "id": "queued-inside-runtime" }));
                let delivered = wait_for_events(&events, 2).await;
                assert_eq!(delivered[0].1["id"], "queued-before-runtime");
                assert_eq!(delivered[1].1["id"], "queued-inside-runtime");
            });
    }

    async fn wait_for_events(events: &Events, expected: usize) -> Vec<(String, Value)> {
        tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                let current = events.lock().unwrap().clone();
                if current.len() >= expected {
                    return current;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("timed out waiting for tracing events")
    }

    #[tokio::test]
    async fn creates_generation_with_protocol_fields_and_parent() {
        let (layer, events) = test_layer();
        let subscriber = tracing_subscriber::registry().with(layer);
        let dispatch = Dispatch::new(subscriber);

        dispatcher::with_default(&dispatch, || {
            let parent = tracing::info_span!(
                target: "goose::test",
                "agent_reply",
                trace_input = tracing::field::Empty,
                session.id = "session-1"
            );
            let _parent_guard = parent.enter();
            parent.record("trace_input", "hello");

            let generation = tracing::info_span!(
                target: "goose::test",
                "provider_generation",
                observation_type = "GENERATION",
                model = "gpt-4o",
                model_parameters = tracing::field::Empty,
                usage_details = tracing::field::Empty,
                cost_details = tracing::field::Empty,
                completion_start_time = tracing::field::Empty,
                input = tracing::field::Empty,
                output = tracing::field::Empty
            );
            generation.record(
                "model_parameters",
                serde_json::to_string(&json!({"temperature": 0.2}))
                    .unwrap()
                    .as_str(),
            );
            generation.record(
                "usage_details",
                serde_json::to_string(&json!({"input": 12, "output": 5, "total": 17}))
                    .unwrap()
                    .as_str(),
            );
            generation.record(
                "cost_details",
                serde_json::to_string(&json!({"total": 0.01}))
                    .unwrap()
                    .as_str(),
            );
            generation.record("completion_start_time", "2026-01-01T00:00:01Z");
            generation.record("output", "done");
        });

        let events = wait_for_events(&events, 11).await;
        assert_eq!(events[0].0, "trace-create");
        assert_eq!(events[0].1["sessionId"], "session-1");
        assert_eq!(events[1].0, "span-create");
        assert!(events
            .iter()
            .any(|(event_type, body)| event_type == "trace-create" && body["input"] == "hello"));
        assert_eq!(events[3].0, "generation-create");

        let parent_id = events[1].1["id"].as_str().unwrap();
        let generation = &events[3].1;
        assert_eq!(generation["parentObservationId"], parent_id);
        assert_eq!(generation["traceId"], events[1].1["traceId"]);
        assert_eq!(generation["model"], "gpt-4o");

        let generation_updates: Vec<_> = events
            .iter()
            .filter(|(event_type, _)| event_type == "generation-update")
            .collect();
        assert!(generation_updates
            .iter()
            .any(|(_, body)| body["modelParameters"]["temperature"] == 0.2));
        assert!(generation_updates
            .iter()
            .any(|(_, body)| body["usageDetails"]["total"] == 17));
        assert!(generation_updates
            .iter()
            .any(|(_, body)| body["costDetails"]["total"] == 0.01));
        assert!(generation_updates
            .iter()
            .any(|(_, body)| body["completionStartTime"] == "2026-01-01T00:00:01Z"));
        assert!(generation_updates
            .iter()
            .any(|(_, body)| body["output"] == "done"));
        assert!(generation_updates
            .iter()
            .any(|(_, body)| body.get("endTime").is_some()));
    }

    #[tokio::test]
    async fn keeps_json_looking_text_fields_as_strings() {
        let (layer, events) = test_layer();
        let subscriber = tracing_subscriber::registry().with(layer);
        let dispatch = Dispatch::new(subscriber);
        let json_text = r#"{"error":"still text"}"#;

        dispatcher::with_default(&dispatch, || {
            let generation = tracing::info_span!(
                target: "goose::test",
                "provider_generation",
                observation_type = "GENERATION",
                trace_input = tracing::field::Empty,
                input = tracing::field::Empty,
                output = tracing::field::Empty,
                status_message = tracing::field::Empty,
            );
            generation.record("trace_input", json_text);
            generation.record("input", json_text);
            generation.record("output", json_text);
            generation.record("status_message", json_text);
        });

        let events = wait_for_events(&events, 7).await;
        assert!(events.iter().any(|(event_type, body)| {
            event_type == "trace-create" && body["input"] == json_text
        }));
        for field in ["input", "output", "statusMessage"] {
            assert!(events.iter().any(|(event_type, body)| {
                event_type == "generation-update" && body[field] == json_text
            }));
        }
    }

    #[tokio::test]
    async fn creates_a_trace_per_root_span() {
        let (layer, events) = test_layer();
        let subscriber = tracing_subscriber::registry().with(layer);
        let dispatch = Dispatch::new(subscriber);

        dispatcher::with_default(&dispatch, || {
            drop(tracing::info_span!(target: "goose::test", "first"));
            drop(tracing::info_span!(target: "goose::test", "second"));
        });

        let events = wait_for_events(&events, 6).await;
        let trace_ids: Vec<_> = events
            .iter()
            .filter(|(event_type, _)| event_type == "trace-create")
            .map(|(_, body)| body["id"].as_str().unwrap())
            .collect();
        assert_eq!(trace_ids.len(), 2);
        assert_ne!(trace_ids[0], trace_ids[1]);
    }

    #[tokio::test]
    async fn nested_reply_fields_update_the_observation_not_the_parent_trace() {
        let (layer, events) = test_layer();
        let subscriber = tracing_subscriber::registry().with(layer);
        let dispatch = Dispatch::new(subscriber);

        dispatcher::with_default(&dispatch, || {
            let parent = tracing::info_span!(
                target: "goose::test",
                "parent_reply",
                trace_boundary = true,
                trace_input = tracing::field::Empty
            );
            let _parent_guard = parent.enter();
            parent.record("trace_input", "parent input");

            let child = tracing::info_span!(
                target: "goose::test",
                "subagent_reply",
                trace_boundary = true,
                trace_input = tracing::field::Empty,
                trace_output = tracing::field::Empty
            );
            child.record("trace_input", "child input");
            child.record("trace_output", "child output");
        });

        let events = wait_for_events(&events, 8).await;
        let trace_inputs: Vec<_> = events
            .iter()
            .filter(|(event_type, body)| {
                event_type == "trace-create" && body.get("input").is_some()
            })
            .map(|(_, body)| &body["input"])
            .collect();
        assert_eq!(trace_inputs.len(), 1);
        assert_eq!(trace_inputs[0], "parent input");

        let child_updates: Vec<_> = events
            .iter()
            .filter(|(event_type, body)| {
                event_type == "span-update"
                    && (body.get("input").is_some() || body.get("output").is_some())
            })
            .collect();
        assert!(child_updates
            .iter()
            .any(|(_, body)| body["input"] == "child input"));
        assert!(child_updates
            .iter()
            .any(|(_, body)| body["output"] == "child output"));
    }

    #[tokio::test]
    async fn instrumented_background_task_keeps_parent_trace() {
        let (layer, events) = test_layer();
        let subscriber = tracing_subscriber::registry().with(layer);
        let dispatch = Dispatch::new(subscriber);
        let background_dispatch = dispatch.clone();

        let task = dispatcher::with_default(&dispatch, || {
            let parent = tracing::info_span!(target: "goose::test", "parent");
            let background = {
                let _guard = parent.enter();
                tracing::info_span!(target: "goose::test", "background_subagent")
            };
            tokio::spawn(
                async move {
                    dispatcher::with_default(&background_dispatch, || {
                        drop(tracing::info_span!(target: "goose::test", "subagent_reply"));
                    });
                }
                .instrument(background),
            )
        });
        task.await.unwrap();

        let events = wait_for_events(&events, 6).await;
        let creates: Vec<_> = events
            .iter()
            .filter(|(event_type, _)| event_type == "span-create")
            .map(|(_, body)| body)
            .collect();
        assert_eq!(creates.len(), 3);
        assert!(creates
            .windows(2)
            .all(|pair| pair[0]["traceId"] == pair[1]["traceId"]));
        assert_eq!(creates[1]["parentObservationId"], creates[0]["id"]);
        assert_eq!(creates[2]["parentObservationId"], creates[1]["id"]);
    }

    #[test]
    fn flattens_text_wrappers() {
        let flattened = flatten_metadata(Map::from_iter([
            ("simple".to_string(), json!("value")),
            ("complex".to_string(), json!({"text": "inner value"})),
        ]));
        assert_eq!(flattened["simple"], "value");
        assert_eq!(flattened["complex"], "inner value");
    }
}
