//! Watchdog for provider streams that stall while keepalive bytes keep
//! arriving.
//!
//! Byte-level timeouts (reqwest's `read_timeout`, line-based timers) reset on
//! any inbound bytes, so a gateway that wedges mid-response but keeps sending
//! SSE comment frames (`: ping`) hangs a turn forever with no error and no
//! retry. The wrappers here reset their timer only on *data-bearing* lines,
//! which a stalled stream never produces, and surface a retryable
//! [`ProviderError::NetworkError`] when the stream goes idle.

use crate::errors::ProviderError;
use futures::{Stream, StreamExt};
use std::pin::Pin;
use std::time::Duration;

/// Stream idle timeout in seconds, honored across all SSE providers.
/// `0` disables the watchdog. Garbage values fall back to the default.
pub const STREAM_TIMEOUT_ENV_VAR: &str = "GOOSE_STREAM_TIMEOUT";

/// Generous enough for slow-but-live models and short gateway queue phases;
/// far below the 17-37 minute silent stalls observed in the field (#11679).
pub const DEFAULT_STREAM_IDLE_TIMEOUT_SECS: u64 = 150;

pub fn resolve_stream_idle_timeout() -> Option<Duration> {
    let secs = std::env::var(STREAM_TIMEOUT_ENV_VAR)
        .ok()
        .and_then(|raw| raw.trim().parse::<u64>().ok())
        .unwrap_or(DEFAULT_STREAM_IDLE_TIMEOUT_SECS);
    (secs > 0).then(|| Duration::from_secs(secs))
}

fn is_data_line(line: &str) -> bool {
    !line.trim().is_empty() && !line.starts_with(':')
}

/// Wrap a framed SSE line stream with the idle watchdog configured via
/// [`STREAM_TIMEOUT_ENV_VAR`]. Returns the stream unchanged when disabled.
pub fn with_stream_idle_timeout_from_env<S>(
    stream: S,
) -> Pin<Box<dyn Stream<Item = anyhow::Result<String>> + Send>>
where
    S: Stream<Item = anyhow::Result<String>> + Unpin + Send + 'static,
{
    match resolve_stream_idle_timeout() {
        Some(idle) => with_stream_idle_timeout(stream, idle),
        None => Box::pin(stream),
    }
}

/// Pass lines through untouched, but error when no data-bearing line has
/// arrived for `idle` since the previous one (or since stream start). SSE
/// comment frames (`: ...`) and blank separators pass through without
/// extending the deadline — this is what byte- and line-level timeouts cannot
/// do, and why they hang forever on keepalive-masked stalls.
///
/// The timer is poll-driven (the deadline is only evaluated while the stream
/// is being polled), so a consumer that stops polling cannot trip it.
pub fn with_stream_idle_timeout<S>(
    stream: S,
    idle: Duration,
) -> Pin<Box<dyn Stream<Item = anyhow::Result<String>> + Send>>
where
    S: Stream<Item = anyhow::Result<String>> + Unpin + Send + 'static,
{
    let idle_secs = idle.as_secs();
    Box::pin(async_stream::try_stream! {
        let mut stream = stream;
        let mut keepalive_frames: u64 = 0;
        let Some(mut deadline) = tokio::time::Instant::now().checked_add(idle) else {
            // Idle so large the deadline overflows; treat as disabled.
            while let Some(item) = stream.next().await {
                yield item?;
            }
            return;
        };
        loop {
            match tokio::time::timeout_at(deadline, stream.next()).await {
                Ok(Some(item)) => {
                    let line = item?;
                    if is_data_line(&line) {
                        keepalive_frames = 0;
                        if let Some(next_deadline) = tokio::time::Instant::now().checked_add(idle) {
                            deadline = next_deadline;
                        }
                    } else {
                        keepalive_frames += 1;
                    }
                    yield line;
                }
                Ok(None) => break,
                Err(_) => {
                    let detail = if keepalive_frames > 0 {
                        format!("{keepalive_frames} keepalive frames kept arriving")
                    } else {
                        "the stream went silent".to_string()
                    };
                    Err::<(), anyhow::Error>(anyhow::Error::new(
                        ProviderError::NetworkError(format!(
                            "Stream stalled: no data received for {idle_secs}s ({detail}). \
                             Raise {STREAM_TIMEOUT_ENV_VAR} if this provider queues requests \
                             slowly, or set it to 0 to disable this watchdog."
                        )),
                    ))?;
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::retry::{should_retry, RetryConfig};
    use futures::StreamExt;

    fn finite_lines(
        lines: Vec<&'static str>,
    ) -> Pin<Box<dyn Stream<Item = anyhow::Result<String>> + Send>> {
        Box::pin(futures::stream::iter(
            lines.into_iter().map(|l| Ok(l.to_string())),
        ))
    }

    /// Yields `first`, then a `: ping` comment frame every `gap_secs` forever —
    /// the failure signature from #11679.
    fn keepalive_forever(
        first: &'static str,
        gap_secs: u64,
    ) -> Pin<Box<dyn Stream<Item = anyhow::Result<String>> + Send>> {
        Box::pin(async_stream::stream! {
            yield Ok(first.to_string());
            loop {
                tokio::time::sleep(Duration::from_secs(gap_secs)).await;
                yield Ok(": ping".to_string());
            }
        })
    }

    /// Yields `first`, then never resolves.
    fn silent_after(
        first: &'static str,
    ) -> impl Stream<Item = anyhow::Result<String>> + Unpin + Send {
        futures::stream::iter(vec![Ok(first.to_string())]).chain(futures::stream::pending())
    }

    async fn drain_until_stall(
        stream: Pin<Box<dyn Stream<Item = anyhow::Result<String>> + Send>>,
        idle: Duration,
    ) -> anyhow::Error {
        let mut watched = with_stream_idle_timeout(stream, idle);
        while let Some(item) = watched.next().await {
            if let Err(e) = item {
                return e;
            }
        }
        panic!("stream ended without a stall error");
    }

    #[tokio::test(start_paused = true)]
    async fn flowing_stream_passes_all_lines_through() {
        let watched = with_stream_idle_timeout(
            finite_lines(vec!["data: {\"a\":1}", ": ping", "", "data: [DONE]"]),
            Duration::from_secs(150),
        );
        let collected: Vec<String> =
            futures::StreamExt::collect::<Vec<_>>(watched.map(|i| i.unwrap())).await;
        assert_eq!(collected.len(), 4);
        assert_eq!(collected[0], "data: {\"a\":1}");
        assert_eq!(collected[3], "data: [DONE]");
    }

    #[tokio::test(start_paused = true)]
    async fn keepalive_comments_do_not_reset_the_timer() {
        let err = drain_until_stall(
            keepalive_forever("data: {\"a\":1}", 10),
            Duration::from_secs(150),
        )
        .await;
        let message = err.to_string();
        assert!(message.contains("Stream stalled"), "got: {message}");
        assert!(message.contains("keepalive"), "got: {message}");
        assert!(message.contains("150"), "got: {message}");
        let provider_error = err.downcast_ref::<ProviderError>().expect("ProviderError");
        assert!(matches!(provider_error, ProviderError::NetworkError(_)));
        assert!(should_retry(provider_error, &RetryConfig::default()));
    }

    #[tokio::test(start_paused = true)]
    async fn silent_stall_reports_silence() {
        let err = drain_until_stall(
            Box::pin(silent_after("data: {\"a\":1}")),
            Duration::from_secs(150),
        )
        .await;
        let message = err.to_string();
        assert!(message.contains("went silent"), "got: {message}");
        assert!(!message.contains("keepalive"), "got: {message}");
    }

    #[tokio::test(start_paused = true)]
    async fn blank_lines_do_not_reset_the_timer() {
        let stream = Box::pin(async_stream::stream! {
            yield Ok("data: {\"a\":1}".to_string());
            loop {
                tokio::time::sleep(Duration::from_secs(10)).await;
                yield Ok(String::new());
            }
        });
        let err = drain_until_stall(stream, Duration::from_secs(150)).await;
        assert!(err.to_string().contains("Stream stalled"));
    }

    #[test]
    fn idle_timeout_env_parsing() {
        let _guard = env_lock::lock_env([(STREAM_TIMEOUT_ENV_VAR, None::<&str>)]);
        assert_eq!(
            resolve_stream_idle_timeout(),
            Some(Duration::from_secs(DEFAULT_STREAM_IDLE_TIMEOUT_SECS))
        );
        drop(_guard);

        let _guard = env_lock::lock_env([(STREAM_TIMEOUT_ENV_VAR, Some("60"))]);
        assert_eq!(resolve_stream_idle_timeout(), Some(Duration::from_secs(60)));
        drop(_guard);

        let _guard = env_lock::lock_env([(STREAM_TIMEOUT_ENV_VAR, Some("0"))]);
        assert_eq!(resolve_stream_idle_timeout(), None);
        drop(_guard);

        let _guard = env_lock::lock_env([(STREAM_TIMEOUT_ENV_VAR, Some("soon"))]);
        assert_eq!(
            resolve_stream_idle_timeout(),
            Some(Duration::from_secs(DEFAULT_STREAM_IDLE_TIMEOUT_SECS))
        );
    }
}
