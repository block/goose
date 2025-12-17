use std::sync::Arc;

use axum::{routing::{post, get, put, delete}, Json, Router, extract::{Path, Query}};
use http::StatusCode;
use serde::{Deserialize, Serialize};
use chrono::{Utc, DateTime};
use uuid::Uuid;

use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct SubmitTrainingDataRequest {
    pub conversation_id: String,
    pub session_id: Option<String>,
    pub messages: Vec<goose::conversation::message::Message>,
    pub provider_used: Option<String>,
    pub model_used: Option<String>,
    pub response_time: Option<f32>,
    pub rating: Option<u8>,
    pub correction: Option<String>,
    pub comments: Option<String>,
    pub domain_tags: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
pub struct SubmitTrainingDataResponse {
    pub example_id: Option<String>,
}

pub async fn submit_training_data(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
    Json(req): Json<SubmitTrainingDataRequest>,
) -> Result<Json<SubmitTrainingDataResponse>, StatusCode> {
    use goose::training_data::schema::TrainingExample;

    let provider = req
        .provider_used
        .unwrap_or_else(|| "native_model".to_string());
    let model = req.model_used.unwrap_or_else(|| {
        goose::providers::base::get_current_model().unwrap_or_else(|| "qwen2.5-7b".to_string())
    });

    // Create a TrainingExample and store it directly
    let mut example = TrainingExample::new(
        req.conversation_id.clone(),
        req.messages.clone(),
        provider,
        model,
    );
    example.session_id = req.session_id;

    // Apply optional fields
    if let Some(tags) = req.domain_tags {
        example.domain_tags = tags;
    }

    // Update quality metrics heuristically from rating
    if let Some(r) = req.rating {
        let score = match r {
            5 => 1.0,
            4 => 0.8,
            3 => 0.6,
            2 => 0.4,
            1 => 0.2,
            _ => 0.5,
        };
        example.quality_metrics.overall_score = score;
    }

    // Add correction as metadata
    if let Some(correction) = req.correction {
        example
            .metadata
            .custom_fields
            .insert("correction".into(), serde_json::json!(correction));
    }
    if let Some(comments) = req.comments {
        example
            .metadata
            .custom_fields
            .insert("comments".into(), serde_json::json!(comments));
    }

    // Store the example
    let example_id = example.id;
    state
        .training_state
        .storage
        .store_example(example)
        .await
        .map_err(|e| {
            tracing::error!("Failed to store training example: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    tracing::info!("Stored training example: {}", example_id);

    Ok(Json(SubmitTrainingDataResponse {
        example_id: Some(example_id.to_string()),
    }))
}

// -------- List with filters/pagination --------
#[derive(Debug, Deserialize)]
pub struct ListExamplesQuery {
    pub page: Option<usize>,
    pub per_page: Option<usize>,
    pub min_quality_score: Option<f32>,
    pub tags: Option<String>, // comma-separated
    pub search: Option<String>,
    pub after: Option<String>, // ISO 8601
    pub before: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TrainingExampleSummary {
    pub id: String,
    pub conversation_id: String,
    pub created_at: String,
    pub quality_score: f32,
    pub message_count: usize,
    pub domain_tags: Vec<String>,
    pub provider_used: String,
    pub model_used: String,
}

#[derive(Debug, Serialize)]
pub struct ListExamplesResponse {
    pub count: usize,
    pub page: usize,
    pub per_page: usize,
    pub examples: Vec<TrainingExampleSummary>,
}

pub async fn list_examples_detailed(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
    Query(q): Query<ListExamplesQuery>,
) -> Result<Json<ListExamplesResponse>, StatusCode> {
    use goose::training_data::storage::TrainingDataStorage;

    let page = q.page.unwrap_or(1).max(1);
    let per_page = q.per_page.unwrap_or(25).clamp(1, 200);

    // Parse tags
    let domain_tags = q.tags.as_ref().map(|s| {
        s.split(',')
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty())
            .collect::<Vec<_>>()
    });

    // Fetch with coarse filters first (min_quality_score, tags)
    let mut items = state
        .training_state
        .storage
        .get_examples_for_training(None, q.min_quality_score, domain_tags)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Additional filters: search, date range
    if let Some(search) = q.search.as_ref().map(|s| s.to_lowercase()) {
        items.retain(|ex| {
            if ex.conversation_id.to_lowercase().contains(&search) { return true; }
            // simple text search over messages
            ex.messages.iter().any(|m| m.as_concat_text().to_lowercase().contains(&search))
        });
    }

    // Date range
    let after = if let Some(a) = &q.after { DateTime::parse_from_rfc3339(a).ok().map(|d| d.with_timezone(&Utc)) } else { None };
    let before = if let Some(b) = &q.before { DateTime::parse_from_rfc3339(b).ok().map(|d| d.with_timezone(&Utc)) } else { None };
    if after.is_some() || before.is_some() {
        items.retain(|ex| {
            let ts = ex.created_at;
            if let Some(a) = after { if ts < a { return false; } }
            if let Some(b) = before { if ts > b { return false; } }
            true
        });
    }

    let count = items.len();
    let start = (page - 1) * per_page;
    let end = (start + per_page).min(count);
    let page_slice = if start < end { &items[start..end] } else { &[] };

    let examples: Vec<TrainingExampleSummary> = page_slice
        .iter()
        .map(|ex| TrainingExampleSummary {
            id: ex.id.to_string(),
            conversation_id: ex.conversation_id.clone(),
            created_at: ex.created_at.to_rfc3339(),
            quality_score: ex.quality_metrics.overall_score,
            message_count: ex.messages.len(),
            domain_tags: ex.domain_tags.clone(),
            provider_used: ex.metadata.provider_used.clone(),
            model_used: ex.metadata.model_used.clone(),
        })
        .collect();

    Ok(Json(ListExamplesResponse { count, page, per_page, examples }))
}

// -------- Get by ID --------
#[derive(Debug, Serialize)]
pub struct GetExampleResponse {
    pub example: goose::training_data::schema::TrainingExample,
}

pub async fn get_example(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<GetExampleResponse>, StatusCode> {
    let uid = Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let ex = state
        .training_state
        .storage
        .get_example(uid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    Ok(Json(GetExampleResponse { example: ex }))
}

/* moved to training.rs */
#[allow(dead_code)]
pub struct _MovedImportMarker;

/* moved import_jsonl to training.rs */
// -------- Update --------
#[derive(Debug, Deserialize)]
pub struct UpdateExampleRequest {
    pub domain_tags: Option<Vec<String>>,
    pub overall_quality_score: Option<f32>,
    pub correction: Option<String>,
    pub comments: Option<String>,
}

pub async fn update_example(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateExampleRequest>,
) -> Result<Json<String>, StatusCode> {
    let uid = Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let mut ex = state
        .training_state
        .storage
        .get_example(uid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    if let Some(tags) = req.domain_tags { ex.domain_tags = tags; }
    if let Some(score) = req.overall_quality_score { ex.quality_metrics.overall_score = score; }
    if let Some(c) = req.correction {
        ex.metadata.custom_fields.insert("correction".into(), serde_json::json!(c));
    }
    if let Some(cm) = req.comments {
        ex.metadata.custom_fields.insert("comments".into(), serde_json::json!(cm));
    }
    ex.updated_at = Utc::now();

    state
        .training_state
        .storage
        .update_example(ex)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json("ok".into()))
}

// -------- Delete --------
pub async fn delete_example(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<String>, StatusCode> {
    let uid = Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let deleted = state
        .training_state
        .storage
        .delete_example(uid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if !deleted { return Err(StatusCode::NOT_FOUND); }
    Ok(Json("deleted".into()))
}

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/training/submit", post(submit_training_data))
        .route("/training/examples/list", get(list_examples_detailed))
        .route("/training/examples/{id}", get(get_example).put(update_example).delete(delete_example))
        .with_state(state)
}
