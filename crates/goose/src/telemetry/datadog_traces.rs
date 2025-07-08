use serde_json::{json, Value};
use crate::telemetry::events::RecipeExecution;

#[derive(Debug, Clone)]
pub struct DatadogTracesExporter {
    client: reqwest::Client,
    api_key: String,
    endpoint: String,
    service_name: String,
    tags: Vec<String>,
}

impl DatadogTracesExporter {
    pub fn new(api_key: String, endpoint: String, service_name: String, service_version: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            endpoint,
            tags: vec![
                format!("service:{}", service_name),
                format!("version:{}", service_version),
                "source:rust-telemetry".to_string(),
            ],
            service_name,
        }
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags.extend(tags);
        self
    }

    pub async fn send_recipe_trace(
        &self,
        execution: &RecipeExecution,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let start_time = execution.start_time;

        let duration_ns = execution.duration_ms.unwrap_or(0) as u64 * 1_000_000;

        let trace_id = format!("{:016x}", start_time);
        let span_id = format!("{:08x}", start_time % 0xFFFFFFFF);

        let mut span_tags = self.tags.clone();
        span_tags.extend(vec![
            format!("recipe.name:{}", execution.recipe_name),
            format!("recipe.version:{}", execution.recipe_version),
            format!("usage.type:{:?}", execution.usage_type),
            format!("result:{:?}", execution.result.as_ref().unwrap_or(&crate::telemetry::events::RecipeResult::Success)),
            format!("user.id:{}", execution.user_id),
        ]);

        if let Some(env) = &execution.environment {
            span_tags.push(format!("env:{}", env));
        }

        for tool_usage in &execution.tool_usage {
            span_tags.push(format!("tool.{}:{}+{}", tool_usage.tool_name, tool_usage.success_count, tool_usage.error_count));
        }

        let span = json!({
            "trace_id": trace_id,
            "span_id": span_id,
            "name": format!("recipe.{}", execution.recipe_name),
            "service": self.service_name,
            "resource": format!("recipe:{}", execution.recipe_name),
            "type": "custom",
            "start": start_time * 1_000_000_000,
            "duration": duration_ns,
            "meta": {
                "recipe.name": execution.recipe_name,
                "recipe.version": execution.recipe_version,
                "usage.type": format!("{:?}", execution.usage_type),
                "user.id": execution.user_id,
                "result": format!("{:?}", execution.result.as_ref().unwrap_or(&crate::telemetry::events::RecipeResult::Success))
            },
            "metrics": {
                "duration.ms": execution.duration_ms.unwrap_or(0) as f64,
                "tool.count": execution.tool_usage.len() as f64
            }
        });

        let mut span_obj = span;
        if let Some(error_details) = &execution.error_details {
            span_obj["error"] = json!(1);
            span_obj["meta"]["error.type"] = json!(error_details.error_type);
            span_obj["meta"]["error.message"] = json!(error_details.error_message);
        }

        if let Some(token_usage) = &execution.token_usage {
            span_obj["metrics"]["tokens.input"] = json!(token_usage.input_tokens as f64);
            span_obj["metrics"]["tokens.output"] = json!(token_usage.output_tokens as f64);
            span_obj["metrics"]["tokens.total"] = json!((token_usage.input_tokens + token_usage.output_tokens) as f64);
            
            if let Some(model) = &token_usage.model {
                span_obj["meta"]["model"] = json!(model);
            }
            if let Some(provider) = &token_usage.provider {
                span_obj["meta"]["provider"] = json!(provider);
            }
        }

        let traces_payload = json!([[span_obj]]);

        self.send_traces_to_datadog(traces_payload).await
    }

    async fn send_traces_to_datadog(&self, traces: Value) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("{}/v0.4/traces", self.endpoint);

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("DD-API-KEY", &self.api_key)
            .json(&traces)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format!(
                "Datadog traces API error {}: {}",
                status, body
            ).into());
        }

        tracing::debug!("Successfully sent trace to Datadog");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::telemetry::events::{RecipeResult, TokenUsage, ToolUsage};

    #[test]
    fn test_datadog_traces_exporter_creation() {
        let exporter = DatadogTracesExporter::new(
            "test-key".to_string(),
            "https://app.datadoghq.com".to_string(),
            "goose".to_string(),
            "1.0.0".to_string(),
        );
        
        assert_eq!(exporter.api_key, "test-key");
        assert_eq!(exporter.endpoint, "https://app.datadoghq.com");
        assert_eq!(exporter.service_name, "goose");
        assert!(exporter.tags.contains(&"service:goose".to_string()));
    }

    #[tokio::test]
    async fn test_send_recipe_trace() {
        let exporter = DatadogTracesExporter::new(
            "test-key".to_string(),
            "https://httpbin.org/post".to_string(),
            "goose".to_string(),
            "1.0.0".to_string(),
        );
        
        let mut execution = RecipeExecution::new("test-recipe", "1.0.0")
            .with_result(RecipeResult::Success)
            .with_token_usage(TokenUsage::new(100, 50));
        
        execution.add_tool_usage(ToolUsage::new("test-tool"));
        
        let result = exporter.send_recipe_trace(&execution).await;
        
        assert!(result.is_err());
    }
}
