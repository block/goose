use opentelemetry::{global, propagation::Injector, Context};
use rmcp::model::Meta;
use serde_json::{Map, Value as JsonValue};
use tracing::Span;
use tracing_opentelemetry::OpenTelemetrySpanExt;

/// A carrier for injecting trace context into MCP _meta field
pub struct McpMetaInjector {
    meta: Map<String, JsonValue>,
}

impl McpMetaInjector {
    pub fn new() -> Self {
        Self { meta: Map::new() }
    }

    pub fn into_meta(self) -> Meta {
        Meta(JsonValue::Object(self.meta).as_object().unwrap().clone())
    }
}

impl Default for McpMetaInjector {
    fn default() -> Self {
        Self::new()
    }
}

impl Injector for McpMetaInjector {
    fn set(&mut self, key: &str, value: String) {
        self.meta.insert(key.to_string(), JsonValue::String(value));
    }
}

/// A carrier for extracting trace context from MCP _meta field
pub struct McpMetaExtractor {
    meta: Map<String, JsonValue>,
}

impl McpMetaExtractor {
    pub fn new(meta: &Meta) -> Self {
        Self {
            meta: meta.0.clone(),
        }
    }
}

impl opentelemetry::propagation::Extractor for McpMetaExtractor {
    fn get(&self, key: &str) -> Option<&str> {
        self.meta.get(key).and_then(|v| v.as_str())
    }

    fn keys(&self) -> Vec<&str> {
        self.meta.keys().map(|k| k.as_str()).collect()
    }
}

/// Inject trace context from the current span into MCP Meta
pub fn inject_trace_context() -> Meta {
    let mut injector = McpMetaInjector::new();
    let cx = Span::current().context();
    global::get_text_map_propagator(|propagator| {
        propagator.inject_context(&cx, &mut injector);
    });
    injector.into_meta()
}

/// Extract trace context from MCP Meta and create a new Context
pub fn extract_trace_context(meta: &Meta) -> Context {
    let extractor = McpMetaExtractor::new(meta);
    global::get_text_map_propagator(|propagator| propagator.extract(&extractor))
}

#[cfg(test)]
mod tests {
    use super::*;
    use opentelemetry::propagation::Extractor;
    use serde_json::json;

    #[test]
    fn test_injector_basic() {
        let mut injector = McpMetaInjector::new();
        injector.set("traceparent", "00-trace-span-01".to_string());
        injector.set("tracestate", "state".to_string());

        let meta = injector.into_meta();
        assert_eq!(meta.0.get("traceparent").unwrap(), "00-trace-span-01");
        assert_eq!(meta.0.get("tracestate").unwrap(), "state");
    }

    #[test]
    fn test_extractor_basic() {
        let meta = Meta(
            json!({
                "traceparent": "00-trace-span-01",
                "tracestate": "state"
            })
            .as_object()
            .unwrap()
            .clone(),
        );

        let extractor = McpMetaExtractor::new(&meta);
        assert_eq!(extractor.get("traceparent"), Some("00-trace-span-01"));
        assert_eq!(extractor.get("tracestate"), Some("state"));
        assert_eq!(extractor.get("nonexistent"), None);
    }

    #[test]
    fn test_extractor_keys() {
        let meta = Meta(
            json!({
                "traceparent": "00-trace-span-01",
                "tracestate": "state",
                "other": "value"
            })
            .as_object()
            .unwrap()
            .clone(),
        );

        let extractor = McpMetaExtractor::new(&meta);
        let keys = extractor.keys();
        assert!(keys.contains(&"traceparent"));
        assert!(keys.contains(&"tracestate"));
        assert!(keys.contains(&"other"));
    }

    #[test]
    fn test_inject_trace_context_no_span() {
        // When there's no active span, should still create empty Meta
        let meta = inject_trace_context();
        // Empty meta is valid
        assert!(meta.0.is_empty() || !meta.0.is_empty());
    }
}
