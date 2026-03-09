use std::time::Duration;

use sacp::schema::{LoadSessionRequest, NewSessionResponse, SessionId, SessionNotification};
use sacp::util::MatchMessage;
use sacp::{AgentPeer, HasPeer, JrConnectionCx, SessionMessage};

use crate::display;
use crate::input;
use crate::wire;

/// Gap between history replay notifications before we consider the burst complete.
const HISTORY_REPLAY_TIMEOUT: Duration = Duration::from_millis(500);

pub(crate) async fn load_existing_session<Link>(
    cx: &JrConnectionCx<Link>,
    session_id: &str,
) -> Result<sacp::ActiveSession<'static, Link>, sacp::Error>
where
    Link: sacp::JrLink + HasPeer<AgentPeer> + 'static,
{
    let sid = SessionId::new(session_id);
    let cwd =
        std::env::current_dir().map_err(|e| sacp::Error::internal_error().data(e.to_string()))?;

    // Register the session notification handler *before* sending the load request.
    // on_load_session streams history as notifications during request processing;
    // without a handler registered first, those notifications would be lost.
    let placeholder = NewSessionResponse::new(sid.clone());
    let mut active_session = cx.attach_session(placeholder, vec![])?;

    cx.send_request_to(AgentPeer, LoadSessionRequest::new(sid, cwd))
        .block_task()
        .await?;

    replay_history(&mut active_session).await;

    Ok(active_session)
}

pub(crate) async fn replay_history<Link>(session: &mut sacp::ActiveSession<'_, Link>)
where
    Link: sacp::JrLink + HasPeer<AgentPeer>,
{
    use tokio::time::timeout;

    // History notifications arrive in a burst. Read until we hit a gap.
    loop {
        match timeout(HISTORY_REPLAY_TIMEOUT, session.read_update()).await {
            Ok(Ok(SessionMessage::SessionMessage(msg))) => {
                MatchMessage::new(msg)
                    .if_notification(async |notif: SessionNotification| {
                        display::display_history_item(&notif.update);
                        Ok(())
                    })
                    .await
                    .otherwise_ignore()
                    .ok();
            }
            Ok(Ok(SessionMessage::StopReason(_))) => break,
            Ok(Err(e)) => {
                tracing::debug!("history replay stream error: {e}");
                break;
            }
            Err(_) => {
                tracing::debug!(
                    "history replay: no message for {HISTORY_REPLAY_TIMEOUT:?}, assuming complete"
                );
                break;
            }
            _ => break,
        }
    }
    eprintln!();
}

pub(crate) fn extract_prompt_data(session: &serde_json::Value) -> (Option<String>, Option<u8>) {
    let model_name = session
        .pointer("/model_config/model_name")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let context_pct = (|| {
        let limit_v = session.pointer("/model_config/context_limit")?;
        let limit = (limit_v
            .as_u64()
            .or_else(|| limit_v.as_i64().and_then(|i| u64::try_from(i).ok())))?
            as f64;
        let v = session.get("total_tokens")?;
        let used = (v
            .as_u64()
            .or_else(|| v.as_i64().and_then(|i| u64::try_from(i).ok())))? as f64;
        if limit > 0.0 {
            Some((used / limit * 100.0).min(100.0) as u8)
        } else {
            None
        }
    })();

    (model_name, context_pct)
}

pub(crate) async fn poll_session_data<Link>(
    session: &sacp::ActiveSession<'_, Link>,
    prompt: &mut input::GoosePrompt,
) where
    Link: sacp::JrLink + sacp::HasPeer<sacp::AgentPeer>,
{
    let session_id = session.session_id().0.to_string();
    if let Ok(resp) = session
        .connection_cx()
        .send_request_to(
            sacp::AgentPeer,
            wire::GetSessionRequest {
                session_id,
                include_messages: false,
            },
        )
        .block_task()
        .await
    {
        let (model, pct) = extract_prompt_data(&resp.session);
        if let Some(name) = model {
            prompt.model_name = Some(name);
        }
        if let Some(p) = pct {
            prompt.update_context(p);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn extract_model_and_context() {
        let session = json!({
            "model_config": { "model_name": "gpt-4", "context_limit": 100000 },
            "total_tokens": 50000
        });
        let (model, pct) = extract_prompt_data(&session);
        assert_eq!(model.as_deref(), Some("gpt-4"));
        assert_eq!(pct, Some(50));
    }

    #[test]
    fn extract_missing_fields() {
        let session = json!({});
        let (model, pct) = extract_prompt_data(&session);
        assert!(model.is_none());
        assert!(pct.is_none());
    }

    #[test]
    fn extract_zero_context_limit() {
        let session = json!({
            "model_config": { "model_name": "m", "context_limit": 0 },
            "total_tokens": 100
        });
        let (_, pct) = extract_prompt_data(&session);
        assert!(pct.is_none(), "zero limit should return None");
    }

    #[test]
    fn extract_context_capped_at_100() {
        let session = json!({
            "model_config": { "context_limit": 1000 },
            "total_tokens": 2000
        });
        let (_, pct) = extract_prompt_data(&session);
        assert_eq!(pct, Some(100));
    }
}
