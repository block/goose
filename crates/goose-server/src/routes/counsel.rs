use crate::state::AppState;
use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use goose::counsel::{CounselOrchestrator, CounselResult, Opinion};
use goose::model::ModelConfig;
use goose::providers::create;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use utoipa::ToSchema;

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct CounselRequest {
    /// The prompt or question to get counsel on
    prompt: String,
    /// Optional provider override (uses session default if not specified)
    provider: Option<String>,
    /// Optional model override (uses session default if not specified)
    model: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CounselResponse {
    /// The winning opinion
    winner: OpinionResponse,
    /// All opinions from the counsel
    all_opinions: Vec<OpinionResponse>,
    /// Vote counts for each member
    vote_counts: HashMap<String, u32>,
    /// Total number of votes cast
    total_votes: u32,
    /// Members that were unavailable
    unavailable_members: Vec<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct OpinionResponse {
    /// Unique identifier for the counsel member
    member_id: String,
    /// Display name of the counsel member
    member_name: String,
    /// The opinion content
    content: String,
    /// The reasoning behind the opinion
    reasoning: String,
}

impl From<Opinion> for OpinionResponse {
    fn from(opinion: Opinion) -> Self {
        Self {
            member_id: opinion.member_id,
            member_name: opinion.member_name,
            content: opinion.content,
            reasoning: opinion.reasoning,
        }
    }
}

impl From<CounselResult> for CounselResponse {
    fn from(result: CounselResult) -> Self {
        Self {
            winner: result.winner.into(),
            all_opinions: result.all_opinions.into_iter().map(Into::into).collect(),
            vote_counts: result.vote_counts,
            total_votes: result.total_votes,
            unavailable_members: result.unavailable_members,
        }
    }
}

#[utoipa::path(
    post,
    path = "/counsel",
    request_body = CounselRequest,
    responses(
        (status = 200, description = "Counsel completed successfully", body = CounselResponse),
        (status = 400, description = "Bad request - invalid parameters"),
        (status = 500, description = "Internal server error - counsel process failed")
    ),
    tag = "counsel"
)]
pub async fn counsel(
    State(_state): State<Arc<AppState>>,
    Json(request): Json<CounselRequest>,
) -> Result<Json<CounselResponse>, StatusCode> {
    tracing::info!(
        counter.goose.counsel_requests = 1,
        interface = "ui",
        "Counsel request started"
    );

    let counsel_start = std::time::Instant::now();

    // Get provider and model configuration
    let config = goose::config::Config::global();

    let provider_name = request
        .provider
        .or_else(|| config.get_goose_provider().ok())
        .ok_or_else(|| {
            tracing::error!("No provider configured");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let model_name = request
        .model
        .or_else(|| config.get_goose_model().ok())
        .ok_or_else(|| {
            tracing::error!("No model configured");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Create model config and provider
    let model_config = ModelConfig::new(&model_name).map_err(|e| {
        tracing::error!("Failed to create model configuration: {}", e);
        StatusCode::BAD_REQUEST
    })?;

    let provider = create(&provider_name, model_config).await.map_err(|e| {
        tracing::error!("Failed to create provider: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Create orchestrator and run counsel process
    let orchestrator = CounselOrchestrator::new(provider);

    let result = orchestrator
        .run(request.prompt.clone())
        .await
        .map_err(|e| {
            tracing::error!("Counsel process failed: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let counsel_duration = counsel_start.elapsed();
    let opinion_count = result.all_opinions.len();
    let total_votes = result.total_votes;

    tracing::info!(
        counter.goose.counsel_completions = 1,
        interface = "ui",
        duration_ms = counsel_duration.as_millis() as u64,
        opinion_count = opinion_count,
        total_votes = total_votes,
        "Counsel request completed"
    );

    tracing::info!(
        counter.goose.counsel_duration_ms = counsel_duration.as_millis() as u64,
        interface = "ui",
        "Counsel duration"
    );

    Ok(Json(result.into()))
}

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/counsel", post(counsel))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opinion_response_conversion() {
        let opinion = Opinion::new("test_id", "Test Member", "Test content", "Test reasoning");

        let response: OpinionResponse = opinion.into();

        assert_eq!(response.member_id, "test_id");
        assert_eq!(response.member_name, "Test Member");
        assert_eq!(response.content, "Test content");
        assert_eq!(response.reasoning, "Test reasoning");
    }

    #[test]
    fn test_counsel_response_conversion() {
        let opinions = vec![
            Opinion::new("id1", "Member 1", "Content 1", "Reasoning 1"),
            Opinion::new("id2", "Member 2", "Content 2", "Reasoning 2"),
        ];

        let mut vote_counts = HashMap::new();
        vote_counts.insert("id1".to_string(), 3);
        vote_counts.insert("id2".to_string(), 2);

        let result = CounselResult::new(
            opinions[0].clone(),
            opinions,
            vote_counts.clone(),
            5,
            vec!["unavailable_member".to_string()],
        );

        let response: CounselResponse = result.into();

        assert_eq!(response.winner.member_id, "id1");
        assert_eq!(response.all_opinions.len(), 2);
        assert_eq!(response.vote_counts, vote_counts);
        assert_eq!(response.total_votes, 5);
        assert_eq!(response.unavailable_members.len(), 1);
    }
}
