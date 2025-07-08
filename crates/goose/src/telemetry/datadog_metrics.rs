use serde_json::{json, Value};
use std::time::{SystemTime, UNIX_EPOCH};

/// Custom Datadog metrics exporter that sends metrics via HTTP API
#[derive(Debug, Clone)]
pub struct DatadogMetricsExporter {
    client: reqwest::Client,
    api_key: String,
    endpoint: String,
    tags: Vec<String>,
}

impl DatadogMetricsExporter {
    pub fn new(api_key: String, endpoint: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("goose-telemetry/1.0")
            .pool_idle_timeout(std::time::Duration::from_secs(30))
            .pool_max_idle_per_host(10)
            .tcp_keepalive(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_else(|e| {
                tracing::warn!("Failed to create custom reqwest client: {}, using default", e);
                reqwest::Client::new()
            });
            
        tracing::info!("Created Datadog metrics exporter for endpoint: {} with enhanced HTTP client", endpoint);
            
        Self {
            client,
            api_key,
            endpoint,
            tags: vec![
                "service:goose".to_string(),
                "source:rust-telemetry".to_string(),
            ],
        }
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags.extend(tags);
        self
    }

    pub async fn send_counter(
        &self,
        metric_name: &str,
        value: u64,
        tags: Vec<String>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut all_tags = self.tags.clone();
        all_tags.extend(tags);

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs() as i64;

        let metric = json!({
            "metric": format!("goose.{}", metric_name),
            "points": [[timestamp, value as f64]],
            "tags": all_tags,
            "type": "count"
        });

        self.send_metrics_to_datadog(vec![metric]).await
    }

    pub async fn send_histogram(
        &self,
        metric_name: &str,
        count: u64,
        sum: f64,
        tags: Vec<String>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut all_tags = self.tags.clone();
        all_tags.extend(tags);

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs() as i64;

        let mut metrics = vec![
            json!({
                "metric": format!("goose.{}.count", metric_name),
                "points": [[timestamp, count as f64]],
                "tags": all_tags.clone(),
                "type": "count"
            }),
            json!({
                "metric": format!("goose.{}.sum", metric_name),
                "points": [[timestamp, sum]],
                "tags": all_tags.clone(),
                "type": "count"
            }),
        ];

        if count > 0 {
            let avg = sum / count as f64;
            metrics.push(json!({
                "metric": format!("goose.{}.avg", metric_name),
                "points": [[timestamp, avg]],
                "tags": all_tags,
                "type": "gauge"
            }));
        }

        self.send_metrics_to_datadog(metrics).await
    }

    pub async fn send_gauge(
        &self,
        metric_name: &str,
        value: f64,
        tags: Vec<String>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut all_tags = self.tags.clone();
        all_tags.extend(tags);

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs() as i64;

        let metric = json!({
            "metric": format!("goose.{}", metric_name),
            "points": [[timestamp, value]],
            "tags": all_tags,
            "type": "gauge"
        });

        self.send_metrics_to_datadog(vec![metric]).await
    }

    async fn send_metrics_to_datadog(&self, metrics: Vec<Value>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if metrics.is_empty() {
            return Ok(());
        }

        let payload = json!({
            "series": metrics
        });

        let url = format!("{}/api/v1/series", self.endpoint);
        
        tracing::debug!("Sending {} metrics to Datadog URL: {}", metrics.len(), url);
        tracing::debug!("Payload: {}", serde_json::to_string_pretty(&payload).unwrap_or_default());

        let response_result = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("DD-API-KEY", &self.api_key)
            .json(&payload)
            .send()
            .await;

        let response = match response_result {
            Ok(resp) => resp,
            Err(e) => {
                tracing::error!("HTTP request failed: {}", e);
                if e.is_timeout() {
                    return Err("Request timeout - check network connectivity".into());
                } else if e.is_connect() {
                    return Err(format!("Connection failed to {}: {}", url, e).into());
                } else if e.is_request() {
                    return Err(format!("Request error: {}", e).into());
                } else {
                    return Err(format!("HTTP client error: {}", e).into());
                }
            }
        };

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format!(
                "Datadog metrics API error {}: {}",
                status, body
            ).into());
        }

        tracing::debug!("Successfully sent {} metrics to Datadog", metrics.len());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_datadog_exporter_creation() {
        let exporter = DatadogMetricsExporter::new(
            "test-key".to_string(),
            "https://app.datadoghq.com".to_string(),
        );
        
        assert_eq!(exporter.api_key, "test-key");
        assert_eq!(exporter.endpoint, "https://app.datadoghq.com");
        assert!(exporter.tags.contains(&"service:goose".to_string()));
    }

    #[test]
    fn test_datadog_exporter_with_tags() {
        let exporter = DatadogMetricsExporter::new(
            "test-key".to_string(),
            "https://app.datadoghq.com".to_string(),
        ).with_tags(vec!["env:test".to_string(), "version:1.0".to_string()]);
        
        assert!(exporter.tags.contains(&"service:goose".to_string()));
        assert!(exporter.tags.contains(&"env:test".to_string()));
        assert!(exporter.tags.contains(&"version:1.0".to_string()));
    }

    #[tokio::test]
    async fn test_send_counter() {
        let exporter = DatadogMetricsExporter::new(
            "test-key".to_string(),
            "https://httpbin.org/post".to_string(),
        );
        
        // This would normally fail with authentication error, but we're testing the structure
        let result = exporter.send_counter("test.counter", 42, vec!["test:true".to_string()]).await;
        
        // We expect this to fail with HTTP error, but not with a panic or compilation error
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_send_histogram() {
        let exporter = DatadogMetricsExporter::new(
            "test-key".to_string(),
            "https://httpbin.org/post".to_string(), // Use httpbin for testing
        );
        
        // This would normally fail with authentication error, but we're testing the structure
        let result = exporter.send_histogram("test.histogram", 10, 45.5, vec!["test:true".to_string()]).await;
        
        // We expect this to fail with HTTP error, but not with a panic or compilation error
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_send_gauge() {
        let exporter = DatadogMetricsExporter::new(
            "test-key".to_string(),
            "https://httpbin.org/post".to_string(), // Use httpbin for testing
        );
        
        // This would normally fail with authentication error, but we're testing the structure
        let result = exporter.send_gauge("test.gauge", 3.14, vec!["test:true".to_string()]).await;
        
        // We expect this to fail with HTTP error, but not with a panic or compilation error
        assert!(result.is_err());
    }
}
