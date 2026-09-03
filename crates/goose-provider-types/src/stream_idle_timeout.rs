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
const DEFAULT_STREAM_IDLE_TIMEOUT_SECS: u64 = 150;

type LineStream = Pin<Box<dyn Stream<Item = anyhow::Result<String>> + Send>>;

fn resolve_stream_idle_timeout() -> Option<Duration> {
    let secs = std::env::var(STREAM_TIMEOUT_ENV_VAR)
        .ok()
        .and_then(|raw| raw.trim().parse::<u64>().ok())
        .unwrap_or(DEFAULT_STREAM_IDLE_TIMEOUT_SECS);
    (secs > 0).then(|| Duration::from_secs(secs))
}

fn is_data_line(line: &str) -> bool {
    !line.trim().is_empty() && !line.starts_with(':')
}

fn stall_error(idle: Duration, keepalive_frames: u64) -> anyhow::Error {
    let detail = if keepalive_frames > 0 {
        format!("{keepalive_frames} keepalive frames kept arriving")
    } else {
        "the stream went silent".to_string()
    };
    anyhow::Error::new(ProviderError::NetworkError(format!(
        "Stream stalled: no data received for {}s ({detail}). \
         Raise {STREAM_TIMEOUT_ENV_VAR} if this provider queues requests \
         slowly, or set it to 0 to disable this watchdog.",
        idle.as_secs()
    )))
}

/// Wrap a framed SSE line stream with the idle watchdog configured via
/// [`STREAM_TIMEOUT_ENV_VAR`]. Returns the stream unchanged when disabled.
pub fn with_stream_idle_timeout_from_env<S>(stream: S) -> LineStream
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
/// comment frames (`: ...`) and blank separators do not extend the deadline.
///
/// The timer is poll-driven (the deadline is only evaluated while the stream
/// is being polled), so a consumer that stops polling cannot trip it.
pub fn with_stream_idle_timeout<S>(stream: S, idle: Duration) -> LineStream
where
    S: Stream<Item = anyhow::Result<String>> + Unpin + Send + 'static,
{
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
                        if let Some(next) = tokio::time::Instant::now().checked_add(idle) {
                            deadline = next;
                        }
                    } else {
                        keepalive_frames += 1;
                    }
                    yield line;
                }
                Ok(None) => break,
                Err(_) => Err::<(), anyhow::Error>(stall_error(idle, keepalive_frames))?,
            }
        }
    })
}

/// Like [`with_stream_idle_timeout`], except the idle deadline is not enforced
/// until the stream yields its first line: time-to-first-line stays governed by
/// the request timeout. This preserves slow local inference (large models can
/// take minutes before the first token) while still catching mid-stream stalls
/// that keepalive comments would otherwise mask.
pub fn with_stream_idle_timeout_after_first_line<S>(mut stream: S, idle: Duration) -> LineStream
where
    S: Stream<Item = anyhow::Result<String>> + Unpin + Send + 'static,
{
    Box::pin(async_stream::try_stream! {
        match stream.next().await {
            Some(item) => yield item?,
            None => return,
        }
        let mut watched = with_stream_idle_timeout(stream, idle);
        while let Some(item) = watched.next().await {
            yield item?;
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::retry::{should_retry, RetryConfig};

    fn finite_lines(lines: Vec<&'static str>) -> LineStream {
        Box::pin(futures::stream::iter(
            lines.into_iter().map(|l| Ok(l.to_string())),
        ))
    }

    /// Yields `first`, then `repeat` every 10s forever.
    fn data_then_repeating(first: &'static str, repeat: &'static str) -> LineStream {
        Box::pin(async_stream::stream! {
            yield Ok(first.to_string());
            loop {
                tokio::time::sleep(Duration::from_secs(10)).await;
                yield Ok(repeat.to_string());
            }
        })
    }

    /// Yields one data line, then never resolves.
    fn silent_after_data() -> LineStream {
        Box::pin(
            futures::stream::iter(vec![Ok("data: {\"a\":1}".to_string())])
                .chain(futures::stream::pending()),
        )
    }

    async fn drain_until_stall(stream: LineStream, idle: Duration) -> anyhow::Error {
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
        let collected: Vec<String> = watched.map(|item| item.unwrap()).collect().await;
        assert_eq!(collected, ["data: {\"a\":1}", ": ping", "", "data: [DONE]"]);
    }

    #[tokio::test(start_paused = true)]
    async fn keepalive_comments_do_not_reset_the_timer() {
        let err = drain_until_stall(
            data_then_repeating("data: {\"a\":1}", ": ping"),
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
    async fn first_line_is_exempt_from_the_idle_deadline() {
        // The stream stays pending long past the idle window before its first
        // line; the watchdog must not fire until after that line arrives.
        let slow_start = Box::pin(async_stream::stream! {
            tokio::time::sleep(Duration::from_secs(600)).await;
            yield Ok("data: {\"a\":1}".to_string());
            yield Ok("data: [DONE]".to_string());
        });
        let watched =
            with_stream_idle_timeout_after_first_line(slow_start, Duration::from_secs(150));
        let collected: Vec<String> = watched.map(|item| item.unwrap()).collect().await;
        assert_eq!(collected, ["data: {\"a\":1}", "data: [DONE]"]);
    }

    #[tokio::test(start_paused = true)]
    async fn keepalives_after_first_line_do_not_reset_the_timer() {
        let mut watched = with_stream_idle_timeout_after_first_line(
            data_then_repeating(": ping", ": ping"),
            Duration::from_secs(150),
        );
        assert_eq!(watched.next().await.unwrap().unwrap(), ": ping");
        let err = loop {
            match watched.next().await {
                Some(Err(e)) => break e,
                Some(Ok(_)) => continue,
                None => panic!("stream ended without a stall error"),
            }
        };
        let message = err.to_string();
        assert!(message.contains("Stream stalled"), "got: {message}");
        assert!(message.contains("keepalive"), "got: {message}");
        assert!(matches!(
            err.downcast_ref::<ProviderError>(),
            Some(ProviderError::NetworkError(_))
        ));
    }

    #[tokio::test(start_paused = true)]
    async fn empty_stream_passes_through() {
        let watched = with_stream_idle_timeout_after_first_line(
            finite_lines(vec![]),
            Duration::from_secs(150),
        );
        let collected: Vec<String> = watched.map(|item| item.unwrap()).collect().await;
        assert!(collected.is_empty());
    }

    #[tokio::test(start_paused = true)]
    async fn silent_stall_reports_silence() {
        let err = drain_until_stall(silent_after_data(), Duration::from_secs(150)).await;
        let message = err.to_string();
        assert!(message.contains("went silent"), "got: {message}");
        assert!(!message.contains("keepalive"), "got: {message}");
    }

    #[tokio::test(start_paused = true)]
    async fn blank_lines_do_not_reset_the_timer() {
        let err = drain_until_stall(
            data_then_repeating("data: {\"a\":1}", ""),
            Duration::from_secs(150),
        )
        .await;
        assert!(err.to_string().contains("Stream stalled"));
    }

    #[test]
    fn idle_timeout_env_parsing() {
        fn resolves_to(value: Option<&str>, expected: Option<Duration>) {
            let _guard = env_lock::lock_env([(STREAM_TIMEOUT_ENV_VAR, value)]);
            assert_eq!(resolve_stream_idle_timeout(), expected);
        }

        let default = Duration::from_secs(DEFAULT_STREAM_IDLE_TIMEOUT_SECS);
        resolves_to(None, Some(default));
        resolves_to(Some("60"), Some(Duration::from_secs(60)));
        resolves_to(Some("0"), None);
        resolves_to(Some("soon"), Some(default));
    }
}
