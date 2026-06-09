use super::{session_meta, GooseAcpAgent, ResultExt};
use crate::session::session_manager::{
    SessionListCursor, SessionListFilters, SessionListPageQuery, SessionType,
};
use crate::session::Session;
use agent_client_protocol::schema::{
    ListSessionsRequest, ListSessionsResponse, SessionId, SessionInfo,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const SESSION_LIST_PAGE_SIZE: usize = 50;
const ACP_SESSION_LIST_TYPES: [SessionType; 3] =
    [SessionType::User, SessionType::Scheduled, SessionType::Acp];

#[derive(Debug, Serialize, Deserialize)]
struct SessionListCursorToken {
    updated_at: chrono::DateTime<chrono::Utc>,
    // Goose stores updated_at with second precision in common write paths, so the
    // cursor needs the full (updated_at, id) sort key to avoid skipping tied rows.
    session_id: String,
    filter_hash: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SessionListCursorFilters {
    cwd: Option<String>,
    session_types: Vec<String>,
    non_empty: bool,
}

fn invalid_session_list_cursor(message: &'static str) -> agent_client_protocol::Error {
    agent_client_protocol::Error::invalid_params().data(message)
}

// bind cursors to the effective filters so they cannot be reused for a different list.
fn session_list_filter_hash(
    cwd: Option<&std::path::Path>,
    session_types: &[SessionType],
) -> Result<String, agent_client_protocol::Error> {
    let mut session_type_names = session_types
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    session_type_names.sort();
    let filters = SessionListCursorFilters {
        cwd: cwd.map(|path| path.to_string_lossy().to_string()),
        session_types: session_type_names,
        non_empty: true,
    };
    let bytes =
        serde_json::to_vec(&filters).internal_err_ctx("Failed to encode session list filters")?;
    Ok(URL_SAFE_NO_PAD.encode(Sha256::digest(bytes)))
}

fn decode_session_list_cursor(
    cursor: Option<&str>,
    cwd: Option<&std::path::Path>,
    session_types: &[SessionType],
) -> Result<Option<SessionListCursor>, agent_client_protocol::Error> {
    let Some(cursor) = cursor else {
        return Ok(None);
    };

    let bytes = URL_SAFE_NO_PAD
        .decode(cursor)
        .map_err(|_| invalid_session_list_cursor("malformed session list cursor"))?;
    let token: SessionListCursorToken = serde_json::from_slice(&bytes)
        .map_err(|_| invalid_session_list_cursor("malformed session list cursor"))?;

    if token.session_id.is_empty() || token.filter_hash.is_empty() {
        return Err(invalid_session_list_cursor("malformed session list cursor"));
    }

    let expected_filter_hash = session_list_filter_hash(cwd, session_types)?;
    if token.filter_hash != expected_filter_hash {
        return Err(invalid_session_list_cursor(
            "session list cursor does not match filters",
        ));
    }

    Ok(Some(SessionListCursor {
        updated_at: token.updated_at,
        session_id: token.session_id,
    }))
}

fn encode_session_list_cursor(
    cursor: &SessionListCursor,
    cwd: Option<&std::path::Path>,
    session_types: &[SessionType],
) -> Result<String, agent_client_protocol::Error> {
    let token = SessionListCursorToken {
        updated_at: cursor.updated_at,
        session_id: cursor.session_id.clone(),
        filter_hash: session_list_filter_hash(cwd, session_types)?,
    };
    let bytes =
        serde_json::to_vec(&token).internal_err_ctx("Failed to encode session list cursor")?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

fn display_title(s: &Session) -> Option<String> {
    if !s.user_set_name {
        if let Some(recipe) = &s.recipe {
            return Some(recipe.title.clone());
        }
    }
    if s.name.is_empty() {
        None
    } else {
        Some(s.name.clone())
    }
}

impl GooseAcpAgent {
    pub(super) async fn on_list_sessions(
        &self,
        req: ListSessionsRequest,
    ) -> Result<ListSessionsResponse, agent_client_protocol::Error> {
        if let Some(cwd) = req.cwd.as_deref() {
            if !cwd.is_absolute() {
                return Err(agent_client_protocol::Error::invalid_params()
                    .data("cwd must be an absolute path"));
            }
        }

        let cwd = req.cwd.as_deref();
        let cursor =
            decode_session_list_cursor(req.cursor.as_deref(), cwd, &ACP_SESSION_LIST_TYPES)?;

        // ACP clients see their own (Acp) sessions plus legacy User/Scheduled ones.
        let page = self
            .session_manager
            .list_sessions_paged(SessionListPageQuery {
                filters: SessionListFilters {
                    types: Some(&ACP_SESSION_LIST_TYPES),
                    working_dir: cwd,
                    require_messages: true,
                    ..Default::default()
                },
                cursor: cursor.as_ref(),
                page_size: SESSION_LIST_PAGE_SIZE,
            })
            .await
            .internal_err()?;
        let session_infos: Vec<SessionInfo> = page
            .sessions
            .into_iter()
            .map(|s| {
                let meta = session_meta(&s);
                let title = display_title(&s);
                let mut info = SessionInfo::new(SessionId::new(s.id), s.working_dir)
                    .updated_at(s.updated_at.to_rfc3339())
                    .meta(meta);
                if let Some(t) = title {
                    info = info.title(t);
                }
                info
            })
            .collect();
        let next_cursor = page
            .next_cursor
            .as_ref()
            .map(|cursor| encode_session_list_cursor(cursor, cwd, &ACP_SESSION_LIST_TYPES))
            .transpose()?;
        Ok(ListSessionsResponse::new(session_infos).next_cursor(next_cursor))
    }
}
