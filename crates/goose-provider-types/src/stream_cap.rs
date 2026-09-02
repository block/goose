//! Wall-clock cap on the lifetime of a single provider response.
//!
//! Byte-level timeouts (reqwest `read_timeout`) are reset by SSE keepalive
//! comments, so a wedged upstream behind a gateway that emits heartbeats
//! (e.g. `: OPENROUTER PROCESSING`) can hang a turn forever (#11679). A
//! wall-clock cap is deliberately coarse: it cannot mistake legitimate
//! upstream queueing or slow generation for a stall (any finite heartbeat
//! phase still completes well inside it), but it guarantees every provider
//! response — including headless `goose run` — terminates.

use std::time::Duration;

use async_stream::try_stream;
use futures::StreamExt;

use crate::base::MessageStream;
use crate::errors::ProviderError;

/// Default wall-clock cap on a single provider response, in seconds.
///
/// Not derived from provider behaviour — no provider documents a maximum
/// response time. It is bounded from below by the longest single completion
/// that is plausibly healthy (a high-effort reasoning turn, minutes), and
/// from above by the point at which a user has already given up and killed
/// the process: the stalls reported in #11679 ran 17, 25 and 37 minutes and
/// every one of them ended because the human intervened, not because the
/// stream resolved. A cap set above that window is never the thing that
/// notices. 15 minutes sits between the two, and `GOOSE_STREAM_MAX_DURATION`
/// exists because the right value is workload-specific.
pub const DEFAULT_STREAM_MAX_DURATION_SECS: u64 = 900;

/// Wall-clock cap per provider response: `GOOSE_STREAM_MAX_DURATION`
/// (seconds, `0` disables) or the 15-minute default
/// ([`DEFAULT_STREAM_MAX_DURATION_SECS`]).
pub fn stream_max_duration() -> Option<Duration> {
    let secs = std::env::var("GOOSE_STREAM_MAX_DURATION")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(DEFAULT_STREAM_MAX_DURATION_SECS);
    (secs > 0).then(|| Duration::from_secs(secs))
}

/// Caps the total lifetime of a provider stream at [`stream_max_duration`].
///
/// The deadline covers the whole response (first token through completion) and
/// the resulting error is a `RequestFailed`, which the agent's pre-first-item
/// retry loop treats as non-transient — a capped response fails the turn
/// instead of silently re-queueing.
///
/// Callers pass `manages_own_context` from `Provider::manages_own_context`.
/// Those providers (ACP, Claude Code, Gemini CLI) are never capped: their
/// "stream" is an entire nested agent run that can legitimately run for hours
/// and block on human permission prompts, and as subprocess-based providers
/// they cannot hit the SSE keepalive stall this cap exists to catch.
pub fn cap_stream_duration(stream: MessageStream, manages_own_context: bool) -> MessageStream {
    if manages_own_context {
        return stream;
    }
    let Some(max_duration) = stream_max_duration() else {
        return stream;
    };
    let Some(deadline) = tokio::time::Instant::now().checked_add(max_duration) else {
        return stream;
    };
    Box::pin(try_stream! {
        let mut stream = stream;
        loop {
            match tokio::time::timeout_at(deadline, stream.next()).await {
                Ok(Some(item)) => yield item?,
                Ok(None) => break,
                Err(_) => {
                    Err(ProviderError::RequestFailed(format!(
                        "Provider response exceeded the maximum duration of {}s without \
                         completing. The upstream model or gateway may have wedged \
                         mid-response behind SSE keepalives. Increase \
                         GOOSE_STREAM_MAX_DURATION (seconds, 0 to disable) if your \
                         responses legitimately run this long.",
                        max_duration.as_secs()
                    )))?;
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversation::message::Message;
    use crate::retry::{should_retry, RetryConfig};
    use futures::stream;

    fn text_item() -> Result<
        (
            Option<Message>,
            Option<crate::conversation::token_usage::ProviderUsage>,
        ),
        ProviderError,
    > {
        Ok((Some(Message::assistant().with_text("a")), None))
    }

    #[test]
    fn stream_max_duration_parses_env() {
        // Each case gets its own scope: env_lock's ENV_MUTEX is non-reentrant,
        // so a shadowed `_guard` (which lives to end of scope) would deadlock.
        {
            let _guard = env_lock::lock_env([("GOOSE_STREAM_MAX_DURATION", None::<&str>)]);
            assert_eq!(
                stream_max_duration(),
                Some(Duration::from_secs(DEFAULT_STREAM_MAX_DURATION_SECS))
            );
        }
        {
            let _guard = env_lock::lock_env([("GOOSE_STREAM_MAX_DURATION", Some("90"))]);
            assert_eq!(stream_max_duration(), Some(Duration::from_secs(90)));
        }
        {
            let _guard = env_lock::lock_env([("GOOSE_STREAM_MAX_DURATION", Some("0"))]);
            assert_eq!(stream_max_duration(), None);
        }
        {
            let _guard = env_lock::lock_env([("GOOSE_STREAM_MAX_DURATION", Some("bogus"))]);
            assert_eq!(
                stream_max_duration(),
                Some(Duration::from_secs(DEFAULT_STREAM_MAX_DURATION_SECS))
            );
        }
    }

    #[tokio::test(start_paused = true)]
    async fn capped_stream_passes_items_through() {
        let _guard = env_lock::lock_env([("GOOSE_STREAM_MAX_DURATION", Some("60"))]);
        let inner: MessageStream = Box::pin(stream::iter(vec![text_item(), text_item()]));

        let items: Vec<_> = cap_stream_duration(inner, false).collect().await;

        assert_eq!(items.len(), 2);
        assert!(items.iter().all(|item| item.is_ok()));
    }

    #[tokio::test(start_paused = true)]
    async fn wedged_stream_errors_at_cap() {
        let _guard = env_lock::lock_env([("GOOSE_STREAM_MAX_DURATION", Some("60"))]);
        let inner: MessageStream =
            Box::pin(stream::once(async { text_item() }).chain(stream::pending()));

        let mut stream = cap_stream_duration(inner, false);
        assert!(stream.next().await.unwrap().is_ok());

        let error = stream.next().await.unwrap().unwrap_err();
        assert!(matches!(error, ProviderError::RequestFailed(_)));
        assert!(!should_retry(
            &error,
            &RetryConfig::default().transient_only()
        ));
        assert!(stream.next().await.is_none());
    }

    #[tokio::test(start_paused = true)]
    async fn cap_disabled_with_zero() {
        let _guard = env_lock::lock_env([("GOOSE_STREAM_MAX_DURATION", Some("0"))]);
        let inner: MessageStream = Box::pin(stream::pending());

        let mut stream = cap_stream_duration(inner, false);
        let result = tokio::time::timeout(Duration::from_secs(86_400), stream.next()).await;

        assert!(result.is_err(), "disabled cap must never fire");
    }

    #[tokio::test(start_paused = true)]
    async fn cap_skipped_for_context_managing_providers() {
        let _guard = env_lock::lock_env([("GOOSE_STREAM_MAX_DURATION", Some("60"))]);
        let inner: MessageStream = Box::pin(stream::pending());

        let mut stream = cap_stream_duration(inner, true);
        let result = tokio::time::timeout(Duration::from_secs(86_400), stream.next()).await;

        assert!(
            result.is_err(),
            "cap must never fire for managed-context providers"
        );
    }

    #[test]
    fn huge_duration_does_not_panic() {
        let _guard =
            env_lock::lock_env([("GOOSE_STREAM_MAX_DURATION", Some("18446744073709551615"))]);
        assert_eq!(stream_max_duration(), Some(Duration::from_secs(u64::MAX)));

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap();
        rt.block_on(async {
            let inner: MessageStream = Box::pin(stream::iter(vec![text_item()]));
            let items: Vec<_> = cap_stream_duration(inner, false).collect().await;
            assert_eq!(items.len(), 1);
        });
    }
}
