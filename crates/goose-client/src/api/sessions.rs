use crate::error::{GooseClientError, Result};
use crate::types::requests::{
    ForkRequest, ImportSessionRequest, UpdateSessionNameRequest, UpdateUserRecipeValuesRequest,
};
use crate::types::responses::{
    ForkResponse, SessionExtensionsResponse, SessionListResponse, UpdateUserRecipeValuesResponse,
};
use crate::GooseClient;
use goose::session::{Session, SessionInsights};
use std::collections::HashMap;

impl GooseClient {
    pub async fn list_sessions(&self) -> Result<SessionListResponse> {
        self.http.get("/sessions").await
    }

    pub async fn get_session(&self, session_id: impl AsRef<str>) -> Result<Session> {
        self.http
            .get(&format!("/sessions/{}", session_id.as_ref()))
            .await
    }

    pub async fn delete_session(&self, session_id: impl AsRef<str>) -> Result<()> {
        self.http
            .delete(&format!("/sessions/{}", session_id.as_ref()))
            .await
    }

    pub async fn rename_session(
        &self,
        session_id: impl AsRef<str>,
        name: impl Into<String>,
    ) -> Result<()> {
        self.http
            .put_empty(
                &format!("/sessions/{}/name", session_id.as_ref()),
                &UpdateSessionNameRequest { name: name.into() },
            )
            .await
    }

    pub async fn export_session(&self, session_id: impl AsRef<str>) -> Result<String> {
        self.http
            .get(&format!("/sessions/{}/export", session_id.as_ref()))
            .await
    }

    pub async fn import_session(&self, json: impl Into<String>) -> Result<Session> {
        self.http
            .post(
                "/sessions/import",
                &ImportSessionRequest { json: json.into() },
            )
            .await
    }

    pub async fn fork_session(
        &self,
        session_id: impl AsRef<str>,
        truncate: bool,
        copy: bool,
        timestamp: Option<i64>,
    ) -> Result<ForkResponse> {
        if truncate && timestamp.is_none() {
            return Err(GooseClientError::Config(
                "truncate=true requires a timestamp".into(),
            ));
        }
        self.http
            .post(
                &format!("/sessions/{}/fork", session_id.as_ref()),
                &ForkRequest {
                    timestamp,
                    truncate,
                    copy,
                },
            )
            .await
    }

    pub async fn search_sessions(
        &self,
        query: &str,
        limit: Option<usize>,
        after_date: Option<&str>,
        before_date: Option<&str>,
    ) -> Result<Vec<Session>> {
        let mut params: Vec<(&str, String)> = vec![("query", query.to_string())];
        if let Some(l) = limit {
            params.push(("limit", l.to_string()));
        }
        if let Some(ad) = after_date {
            params.push(("afterDate", ad.to_string()));
        }
        if let Some(bd) = before_date {
            params.push(("beforeDate", bd.to_string()));
        }
        let query_refs: Vec<(&str, &str)> = params.iter().map(|(k, v)| (*k, v.as_str())).collect();
        self.http
            .get_with_query("/sessions/search", &query_refs)
            .await
    }

    pub async fn get_session_insights(&self) -> Result<SessionInsights> {
        self.http.get("/sessions/insights").await
    }

    pub async fn get_session_extensions(
        &self,
        session_id: impl AsRef<str>,
    ) -> Result<SessionExtensionsResponse> {
        self.http
            .get(&format!("/sessions/{}/extensions", session_id.as_ref()))
            .await
    }

    pub async fn update_user_recipe_values(
        &self,
        session_id: impl AsRef<str>,
        values: HashMap<String, String>,
    ) -> Result<UpdateUserRecipeValuesResponse> {
        self.http
            .put(
                &format!("/sessions/{}/user_recipe_values", session_id.as_ref()),
                &UpdateUserRecipeValuesRequest {
                    user_recipe_values: values,
                },
            )
            .await
    }
}
