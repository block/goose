use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::time::sleep;
use tracing::{info, warn};

pub struct RateLimitedTelemetrySender {
    sender: mpsc::UnboundedSender<TelemetryEvent>,
}

#[derive(Debug, Clone)]
pub enum TelemetryEvent {
    Span(SpanData),
    Metric(MetricData),
}

#[derive(Debug, Clone)]
pub struct SpanData {
    pub name: String,
    pub attributes: Vec<(String, String)>,
    pub duration: Option<Duration>,
}

#[derive(Debug, Clone)]
pub struct MetricData {
    pub name: String,
    pub value: f64,
    pub labels: Vec<(String, String)>,
}

impl RateLimitedTelemetrySender {
    pub fn new(rate_limit_ms: u64) -> Self {
        let (sender, mut receiver) = mpsc::unbounded_channel::<TelemetryEvent>();

        tokio::spawn(async move {
            let mut last_send = Instant::now();
            let rate_limit_duration = Duration::from_millis(rate_limit_ms);

            info!(
                "Starting rate-limited telemetry sender with {}ms delay",
                rate_limit_ms
            );

            while let Some(event) = receiver.recv().await {
                let elapsed = last_send.elapsed();
                if elapsed < rate_limit_duration {
                    let sleep_duration = rate_limit_duration - elapsed;
                    sleep(sleep_duration).await;
                }

                match event {
                    TelemetryEvent::Span(span_data) => {
                        Self::process_span(span_data).await;
                    }
                    TelemetryEvent::Metric(metric_data) => {
                        Self::process_metric(metric_data).await;
                    }
                }

                last_send = Instant::now();
            }

            warn!("Rate-limited telemetry sender shutting down");
        });

        Self { sender }
    }

    pub fn send_span(
        &self,
        span_data: SpanData,
    ) -> Result<(), mpsc::error::SendError<TelemetryEvent>> {
        self.sender.send(TelemetryEvent::Span(span_data))
    }

    pub fn send_metric(
        &self,
        metric_data: MetricData,
    ) -> Result<(), mpsc::error::SendError<TelemetryEvent>> {
        self.sender.send(TelemetryEvent::Metric(metric_data))
    }

    async fn process_span(span_data: SpanData) {
        let span = tracing::info_span!("telemetry_span", name = %span_data.name);
        let _enter = span.enter();

        for (key, value) in span_data.attributes {
            tracing::Span::current().record(key.as_str(), value.as_str());
        }

        if let Some(duration) = span_data.duration {
            info!(duration_ms = duration.as_millis(), "span_duration");
        }
    }

    async fn process_metric(metric_data: MetricData) {
        info!(
            metric_name = %metric_data.name,
            metric_value = metric_data.value,
            labels = ?metric_data.labels,
            "telemetry_metric"
        );
    }
}

impl Default for RateLimitedTelemetrySender {
    fn default() -> Self {
        Self::new(400)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{timeout, Duration as TokioDuration};

    #[tokio::test]
    async fn test_rate_limited_sender() {
        let sender = RateLimitedTelemetrySender::new(100); // 100ms rate limit for testing

        let span_data = SpanData {
            name: "test_span".to_string(),
            attributes: vec![("key".to_string(), "value".to_string())],
            duration: Some(Duration::from_millis(50)),
        };

        let metric_data = MetricData {
            name: "test_metric".to_string(),
            value: 42.0,
            labels: vec![("label".to_string(), "value".to_string())],
        };

        // Send events
        assert!(sender.send_span(span_data).is_ok());
        assert!(sender.send_metric(metric_data).is_ok());

        // Give time for processing
        timeout(TokioDuration::from_millis(500), async {
            tokio::time::sleep(TokioDuration::from_millis(300)).await;
        })
        .await
        .unwrap();
    }
}
