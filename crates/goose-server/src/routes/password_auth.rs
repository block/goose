use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use goose::identity::{AuthMethod, UserIdentity};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;

use crate::state::AppState;

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};

#[derive(Debug, Deserialize, ToSchema)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub tenant_id: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RegisterResponse {
    pub user_id: String,
    pub username: String,
    pub display_name: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PasswordLoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PasswordLoginResponse {
    pub token: String,
    pub token_type: String,
    pub expires_in: u64,
    pub user: PasswordUserInfo,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PasswordUserInfo {
    pub id: String,
    pub name: String,
    pub auth_method: String,
    pub tenant: Option<String>,
}

async fn ensure_local_users_table(state: &AppState) -> Result<(), StatusCode> {
    let session_manager = state.session_manager();
    let pool = session_manager
        .storage()
        .pool()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS local_users (
            id TEXT PRIMARY KEY,
            username TEXT UNIQUE NOT NULL,
            password_hash TEXT NOT NULL,
            display_name TEXT NOT NULL,
            tenant_id TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        )"#,
    )
    .execute(pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(())
}

#[utoipa::path(
    post,
    path = "/auth/register",
    request_body = RegisterRequest,
    responses(
        (status = 201, description = "User registered", body = RegisterResponse),
        (status = 409, description = "Username already taken"),
        (status = 500, description = "Internal error"),
    ),
    tag = "auth"
)]
pub async fn register(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<RegisterResponse>), StatusCode> {
    ensure_local_users_table(&state).await?;

    if req.username.is_empty() || req.password.len() < 8 {
        return Err(StatusCode::BAD_REQUEST);
    }

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(req.password.as_bytes(), &salt)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .to_string();

    let user_id = format!("local:{}", req.username);
    let display_name = req.display_name.unwrap_or_else(|| req.username.clone());

    let pool = state
        .session_manager()
        .storage()
        .pool()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    sqlx::query(
        "INSERT INTO local_users (id, username, password_hash, display_name, tenant_id) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&user_id)
    .bind(&req.username)
    .bind(&password_hash)
    .bind(&display_name)
    .bind(&req.tenant_id)
    .execute(pool)
    .await
    .map_err(|e| {
        if e.to_string().contains("UNIQUE") {
            StatusCode::CONFLICT
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        }
    })?;

    Ok((
        StatusCode::CREATED,
        Json(RegisterResponse {
            user_id,
            username: req.username,
            display_name,
        }),
    ))
}

#[utoipa::path(
    post,
    path = "/auth/login/password",
    request_body = PasswordLoginRequest,
    responses(
        (status = 200, description = "Login successful", body = PasswordLoginResponse),
        (status = 401, description = "Invalid credentials"),
        (status = 500, description = "Internal error"),
    ),
    tag = "auth"
)]
pub async fn password_login(
    State(state): State<Arc<AppState>>,
    Json(req): Json<PasswordLoginRequest>,
) -> Result<Json<PasswordLoginResponse>, StatusCode> {
    ensure_local_users_table(&state).await?;

    let pool = state
        .session_manager()
        .storage()
        .pool()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let row: Option<(String, String, String, Option<String>)> = sqlx::query_as(
        "SELECT id, password_hash, display_name, tenant_id FROM local_users WHERE username = ?",
    )
    .bind(&req.username)
    .fetch_optional(pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let (user_id, stored_hash, display_name, tenant_id) = row.ok_or(StatusCode::UNAUTHORIZED)?;

    let parsed_hash =
        PasswordHash::new(&stored_hash).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Argon2::default()
        .verify_password(req.password.as_bytes(), &parsed_hash)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    let user = UserIdentity {
        id: user_id.clone(),
        name: display_name.clone(),
        auth_method: AuthMethod::Password,
        tenant: tenant_id.clone(),
    };

    let token = state
        .session_token_store
        .issue_token(&user)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let ttl = state.session_token_store.ttl().as_secs();

    Ok(Json(PasswordLoginResponse {
        token,
        token_type: "Bearer".to_string(),
        expires_in: ttl,
        user: PasswordUserInfo {
            id: user_id,
            name: display_name,
            auth_method: "password".to_string(),
            tenant: tenant_id,
        },
    }))
}

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/auth/register", post(register))
        .route("/auth/login/password", post(password_login))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    fn unique(prefix: &str) -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let ns = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        format!("{prefix}_{ns}")
    }

    #[tokio::test]
    async fn test_register_and_login() {
        let state = AppState::new().await.unwrap();
        let app = routes(state);
        let user = unique("login");

        let register_body = serde_json::json!({
            "username": user,
            "password": "securepassword123",
            "display_name": "Test User"
        });

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/register")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&register_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        let login_body = serde_json::json!({
            "username": user,
            "password": "securepassword123"
        });

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/login/password")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&login_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_login_wrong_password() {
        let state = AppState::new().await.unwrap();
        let app = routes(state);
        let user = unique("wrong_pw");

        let register_body = serde_json::json!({
            "username": user,
            "password": "correctpassword1",
        });

        let _ = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/register")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&register_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let login_body = serde_json::json!({
            "username": user,
            "password": "wrongpassword12",
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/login/password")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&login_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_duplicate_registration() {
        let state = AppState::new().await.unwrap();
        let app = routes(state);
        let user = unique("dup");

        let body = serde_json::json!({
            "username": user,
            "password": "password1234",
        });

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/register")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/register")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CONFLICT);
    }
}
