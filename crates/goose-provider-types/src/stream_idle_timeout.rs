//! Watchdog for provider streams that stall while keepalive bytes keep
//! arriving.
//!
//! Byte-level timeouts (reqwest's `read_timeout`, line-based timers) reset on
//! any inbound bytes, so a gateway that wedges mid-response but keeps sending
//! SSE comment frames (`: ping`) hangs a turn forever with no error and no
//! retry. The wrappers here reset their timer only on *data-bearing* lines,
//! which a stalled stream never produces, and surface a retryable
//! [`ProviderError::NetworkError`] when the stream goes idle.
//!
//! Non-data means: comment frames (`: ...`), blank separators, SSE control
//! fields (`event:`, `id:`, `retry:`), and `data:` fields with an empty
//! payload — none of which the downstream parsers can turn into progress, so
//! none of them may hold the watchdog off. Payload-bearing events do reset
//! the timer, including protocol-level pings such as Anthropic's
//! `data: {"type":"ping"}`: telling those apart from real progress needs
//! per-provider payload knowledge, which this line-level watchdog
//! deliberately does not have.
//!
//! The idle deadline is never enforced before the stream's first line, so
//! time-to-first-line stays governed by the request timeout — preserving slow
//! starts (reasoning models, queued gateways, large local models) while still
//! catching keepalive-masked stalls, which by definition send lines.

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

/// SSE field names that never carry payload. A live stream always pairs
/// them with a `data:` line, so exempting them from the idle deadline cannot
/// turn a slow-but-live stream into a stall.
const SSE_CONTROL_FIELDS: &[&str] = &["event", "id", "retry"];

/// The value of an SSE `name: value` field line, with the single leading
/// space the spec allows stripped; `None` when `line` is not that field.
fn sse_field<'a>(line: &'a str, name: &str) -> Option<&'a str> {
    let value = line.strip_prefix(name)?.strip_prefix(':')?;
    Some(value.strip_prefix(' ').unwrap_or(value))
}

fn is_data_line(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with(':') {
        return false;
    }
    if SSE_CONTROL_FIELDS
        .iter()
        .any(|field| sse_field(trimmed, field).is_some())
    {
        return false;
    }
    // An empty `data:` payload dispatches an event the parsers skip, so it
    // is not progress either.
    match sse_field(trimmed, "data") {
        Some(value) => !value.trim().is_empty(),
        None => true,
    }
}

fn stall_error(idle: Duration, keepalive_frames: u64, env_var: &str) -> anyhow::Error {
    let detail = if keepalive_frames > 0 {
        format!("{keepalive_frames} keepalive frames kept arriving")
    } else {
        "the stream went silent".to_string()
    };
    anyhow::Error::new(ProviderError::NetworkError(format!(
        "Stream stalled: no data received for {}s ({detail}). \
         Raise {env_var} if this provider streams data slowly, or set it to \
         0 to disable this watchdog.",
        idle.as_secs()
    )))
}

/// Wrap a framed SSE line stream with the idle watchdog configured via
/// [`STREAM_TIMEOUT_ENV_VAR`]. Returns the stream unchanged when disabled.
/// The idle deadline is not enforced until the first line arrives.
pub fn with_stream_idle_timeout_after_first_line_from_env<S>(stream: S) -> LineStream
where
    S: Stream<Item = anyhow::Result<String>> + Unpin + Send + 'static,
{
    match resolve_stream_idle_timeout() {
        Some(idle) => {
            with_stream_idle_timeout_after_first_line(stream, idle, STREAM_TIMEOUT_ENV_VAR)
        }
        None => Box::pin(stream),
    }
}

/// Pass lines through untouched, but error when no data-bearing line has
/// arrived for `idle` since the previous one (or since stream start). SSE
/// comment frames (`: ...`), blank separators, control fields (`event:`,
/// `id:`, `retry:`), and empty `data:` fields do not extend the deadline.
///
/// This is the eager core: the deadline applies from the first poll. Prefer
/// [`with_stream_idle_timeout_after_first_line`], which exempts time to first
/// line, for provider streams.
///
/// `env_var` is the variable named in the stall error's remediation; callers
/// that resolve the timeout from a different variable pass its name so the
/// advice points at a knob that actually takes effect.
///
/// The timer is poll-driven (the deadline is only evaluated while the stream
/// is being polled), so a consumer that stops polling cannot trip it.
pub fn with_stream_idle_timeout<S>(stream: S, idle: Duration, env_var: &'static str) -> LineStream
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
                    } else if !line.trim().is_empty() {
                        // Any non-blank line that carries no payload —
                        // comment frames, control fields, empty data fields —
                        // is a keepalive.
                        keepalive_frames += 1;
                    }
                    yield line;
                }
                Ok(None) => break,
                Err(_) => Err::<(), anyhow::Error>(stall_error(idle, keepalive_frames, env_var))?,
            }
        }
    })
}

/// Like [`with_stream_idle_timeout`], except the idle deadline is not enforced
/// until the stream yields its first line: time-to-first-line stays governed by
/// the request timeout. This preserves slow starts — reasoning models, queued
/// gateways, large local models can all take minutes before the first token —
/// while still catching stalls that keepalive lines would otherwise mask,
/// since those lines arm the timer.
pub fn with_stream_idle_timeout_after_first_line<S>(
    stream: S,
    idle: Duration,
    env_var: &'static str,
) -> LineStream
where
    S: Stream<Item = anyhow::Result<String>> + Unpin + Send + 'static,
{
    Box::pin(async_stream::try_stream! {
        let mut stream = stream;
        if let Some(item) = stream.next().await {
            yield item?;
        }
        let mut watched = with_stream_idle_timeout(stream, idle, env_var);
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

    async fn first_stall_error(mut watched: LineStream) -> anyhow::Error {
        while let Some(item) = watched.next().await {
            if let Err(e) = item {
                return e;
            }
        }
        panic!("stream ended without a stall error");
    }

    fn assert_keepalive_masked_stall(err: &anyhow::Error) {
        let message = err.to_string();
        assert!(message.contains("Stream stalled"), "got: {message}");
        assert!(message.contains("keepalive"), "got: {message}");
        let provider_error = err.downcast_ref::<ProviderError>().expect("ProviderError");
        assert!(matches!(provider_error, ProviderError::NetworkError(_)));
        assert!(should_retry(provider_error, &RetryConfig::default()));
    }

    #[tokio::test(start_paused = true)]
    async fn flowing_stream_passes_all_lines_through() {
        let watched = with_stream_idle_timeout(
            finite_lines(vec![
                "event: message_start",
                "data: {\"a\":1}",
                ": ping",
                "data:",
                "id: 7",
                "retry: 3000",
                "",
                "data: [DONE]",
            ]),
            Duration::from_secs(150),
            STREAM_TIMEOUT_ENV_VAR,
        );
        let collected: Vec<String> = watched.map(|item| item.unwrap()).collect().await;
        assert_eq!(
            collected,
            [
                "event: message_start",
                "data: {\"a\":1}",
                ": ping",
                "data:",
                "id: 7",
                "retry: 3000",
                "",
                "data: [DONE]"
            ]
        );
    }

    #[tokio::test(start_paused = true)]
    async fn keepalive_comments_do_not_reset_the_timer() {
        let err = first_stall_error(with_stream_idle_timeout(
            data_then_repeating("data: {\"a\":1}", ": ping"),
            Duration::from_secs(150),
            STREAM_TIMEOUT_ENV_VAR,
        ))
        .await;
        assert_keepalive_masked_stall(&err);
        assert!(err.to_string().contains("150"), "got: {err}");
    }

    #[tokio::test(start_paused = true)]
    async fn sse_control_heartbeats_do_not_reset_the_timer() {
        // Gateways that heartbeat with control fields or empty data fields
        // instead of comments must not mask a stall either: the parsers skip
        // all of these shapes.
        for heartbeat in ["event: ping", "id: 7", "retry: 3000", "data:", "data:   "] {
            let err = first_stall_error(with_stream_idle_timeout(
                data_then_repeating("data: {\"a\":1}", heartbeat),
                Duration::from_secs(150),
                STREAM_TIMEOUT_ENV_VAR,
            ))
            .await;
            assert_keepalive_masked_stall(&err);
        }
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
        let watched = with_stream_idle_timeout_after_first_line(
            slow_start,
            Duration::from_secs(150),
            STREAM_TIMEOUT_ENV_VAR,
        );
        let collected: Vec<String> = watched.map(|item| item.unwrap()).collect().await;
        assert_eq!(collected, ["data: {\"a\":1}", "data: [DONE]"]);
    }

    #[tokio::test(start_paused = true)]
    async fn keepalives_after_first_line_do_not_reset_the_timer() {
        let mut watched = with_stream_idle_timeout_after_first_line(
            data_then_repeating(": ping", ": ping"),
            Duration::from_secs(150),
            STREAM_TIMEOUT_ENV_VAR,
        );
        assert_eq!(watched.next().await.unwrap().unwrap(), ": ping");
        assert_keepalive_masked_stall(&first_stall_error(watched).await);
    }

    #[tokio::test(start_paused = true)]
    async fn empty_stream_passes_through() {
        let watched = with_stream_idle_timeout_after_first_line(
            finite_lines(vec![]),
            Duration::from_secs(150),
            STREAM_TIMEOUT_ENV_VAR,
        );
        let collected: Vec<String> = watched.map(|item| item.unwrap()).collect().await;
        assert!(collected.is_empty());
    }

    #[tokio::test(start_paused = true)]
    async fn silent_stall_reports_silence() {
        let err = first_stall_error(with_stream_idle_timeout(
            silent_after_data(),
            Duration::from_secs(150),
            STREAM_TIMEOUT_ENV_VAR,
        ))
        .await;
        let message = err.to_string();
        assert!(message.contains("went silent"), "got: {message}");
        assert!(!message.contains("keepalive"), "got: {message}");
    }

    #[tokio::test(start_paused = true)]
    async fn stall_error_names_the_configured_env_var() {
        // Paths that resolve the timeout from a provider-specific variable
        // (Ollama: OLLAMA_STREAM_TIMEOUT) must have the stall error point at
        // that variable, not the shared one.
        let err = first_stall_error(with_stream_idle_timeout(
            silent_after_data(),
            Duration::from_secs(150),
            "OLLAMA_STREAM_TIMEOUT",
        ))
        .await;
        let message = err.to_string();
        assert!(message.contains("OLLAMA_STREAM_TIMEOUT"), "got: {message}");
        assert!(!message.contains("GOOSE_STREAM_TIMEOUT"), "got: {message}");
    }

    #[tokio::test(start_paused = true)]
    async fn blank_lines_do_not_reset_the_timer() {
        let err = first_stall_error(with_stream_idle_timeout(
            data_then_repeating("data: {\"a\":1}", ""),
            Duration::from_secs(150),
            STREAM_TIMEOUT_ENV_VAR,
        ))
        .await;
        let message = err.to_string();
        assert!(message.contains("Stream stalled"), "got: {message}");
        // Blank separators are not keepalive frames; the diagnostic must not
        // claim any arrived.
        assert!(!message.contains("keepalive"), "got: {message}");
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
