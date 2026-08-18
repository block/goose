//! E2E-1 — avocado OAuth → provision → chat with virtual key (not JWT).
//! covers AC-2, AC-3, AC-4, AC-5, AC-8
//!
//! Phase 0: fails until avocado_auth + provider OAuth wiring exist.

use goose::providers::avocado::{AvocadoProvider, AVOCADO_PROVIDER_NAME};
use goose::providers::avocado_auth::{
    clear_configured_key, complete_oauth_from_access_token, has_configured_key, resolve_api_key,
    ProvisionError,
};
use goose::providers::base::{Provider, ProviderDescriptor};
use goose_providers::model::ModelConfig;
use serde_json::json;
use serial_test::serial;
use std::sync::Arc;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

fn random_sub() -> String {
    format!("user-{}", uuid::Uuid::new_v4())
}

async fn with_temp_goose_root<F, Fut>(f: F)
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().to_string_lossy().to_string();
    let _guard = env_lock::lock_env([
        ("GOOSE_PATH_ROOT", Some(root.as_str())),
        ("GOOSE_DISABLE_KEYRING", Some("1")),
        ("AVOCADO_API_KEY", None::<&str>),
        ("AVOCADO_HOST", None::<&str>),
    ]);
    clear_configured_key().ok();
    f().await;
    clear_configured_key().ok();
}

#[tokio::test]
#[serial]
async fn given_mock_oauth_and_provision_when_configure_then_stream_then_chat_uses_virtual_key_not_jwt(
) {
    // covers AC-2, AC-3, AC-8
    with_temp_goose_root(|| async {
        let provision = MockServer::start().await;
        let llm = MockServer::start().await;
        let sub_a = random_sub();
        let sub_b = random_sub();
        let key_a = format!("sk-gen-{}", &sub_a);
        let key_b = format!("sk-gen-{}", &sub_b);
        let jwt_a = format!("eyJ.fake.jwt.{}", &sub_a);
        let jwt_b = format!("eyJ.fake.jwt.{}", &sub_b);

        let captured_chat_auth: Arc<std::sync::Mutex<Vec<String>>> =
            Arc::new(std::sync::Mutex::new(Vec::new()));
        let capture = captured_chat_auth.clone();

        Mock::given(method("POST"))
            .and(path("/keys/provision"))
            .and(header("authorization", format!("Bearer {}", jwt_a).as_str()))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "apiKey": key_a,
                "baseUrl": llm.uri(),
                "userId": format!("tenant:{}", sub_a),
                "expiresAt": "2099-01-01T00:00:00.000Z",
            })))
            .mount(&provision)
            .await;

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(move |req: &Request| {
                if let Some(auth) = req.headers.get("authorization") {
                    capture
                        .lock()
                        .unwrap()
                        .push(String::from_utf8_lossy(auth.as_bytes()).to_string());
                }
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(
                        "data: {\"id\":\"1\",\"choices\":[{\"delta\":{\"content\":\"OK\"},\"index\":0}]}\n\n\
                         data: [DONE]\n\n",
                    )
            })
            .mount(&llm)
            .await;

        assert!(!has_configured_key());

        complete_oauth_from_access_token(&jwt_a, &format!("{}/keys/provision", provision.uri()))
            .await
            .expect("provision for user A");
        assert!(has_configured_key());

        std::env::set_var("AVOCADO_HOST", llm.uri());
        let provider = AvocadoProvider::from_env(None)
            .await
            .expect("from_env with provisioned key");

        let model = ModelConfig::new("anthropic/claude-sonnet-4.6");
        let stream = provider
            .stream(&model, "sys", &[], &[])
            .await
            .expect("stream");
        drop(stream);

        let auths = captured_chat_auth.lock().unwrap().clone();
        assert!(
            !auths.is_empty(),
            "chat endpoint should have received Authorization"
        );
        let auth = &auths[0];
        assert!(
            auth.contains(&key_a),
            "chat must use virtual key, got {auth}"
        );
        assert!(
            !auth.contains(&jwt_a),
            "chat must not use Zitadel JWT, got {auth}"
        );
        assert!(auth.starts_with("Bearer sk-"), "virtual keys are sk-…");

        // Second subject → different key (AC-8)
        clear_configured_key().ok();
        Mock::given(method("POST"))
            .and(path("/keys/provision"))
            .and(header("authorization", format!("Bearer {}", jwt_b).as_str()))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "apiKey": key_b,
                "baseUrl": llm.uri(),
                "userId": format!("tenant:{}", sub_b),
                "expiresAt": "2099-01-01T00:00:00.000Z",
            })))
            .mount(&provision)
            .await;

        complete_oauth_from_access_token(&jwt_b, &format!("{}/keys/provision", provision.uri()))
            .await
            .expect("provision for user B");
        let stored = resolve_api_key().expect("stored key");
        assert_ne!(stored, key_a);
        assert_eq!(stored, key_b);

        let meta = AvocadoProvider::metadata();
        assert_eq!(meta.name, AVOCADO_PROVIDER_NAME);
        let api_key = meta
            .config_keys
            .iter()
            .find(|k| k.name == "AVOCADO_API_KEY")
            .expect("AVOCADO_API_KEY");
        assert!(api_key.oauth_flow, "AC-1: oauth_flow must be true");
    })
    .await;
}

#[tokio::test]
#[serial]
async fn given_401_invalid_token_when_provision_then_err_and_no_secret() {
    // covers AC-4
    with_temp_goose_root(|| async {
        let provision = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/keys/provision"))
            .respond_with(ResponseTemplate::new(401).set_body_json(json!({
                "error": "invalid_token"
            })))
            .mount(&provision)
            .await;

        let err = complete_oauth_from_access_token(
            "bad-jwt",
            &format!("{}/keys/provision", provision.uri()),
        )
        .await
        .expect_err("401");
        assert!(matches!(err, ProvisionError::Unauthorized));
        assert!(!has_configured_key());
    })
    .await;
}

#[tokio::test]
#[serial]
async fn given_403_forbidden_when_provision_then_err_and_no_secret() {
    // covers AC-4
    with_temp_goose_root(|| async {
        let provision = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/keys/provision"))
            .respond_with(ResponseTemplate::new(403).set_body_json(json!({
                "error": "forbidden",
                "detail": "Missing required role: agent-access"
            })))
            .mount(&provision)
            .await;

        let err = complete_oauth_from_access_token(
            "roleless-jwt",
            &format!("{}/keys/provision", provision.uri()),
        )
        .await
        .expect_err("403");
        assert!(matches!(err, ProvisionError::Forbidden { .. }));
        assert!(!has_configured_key());
    })
    .await;
}

#[tokio::test]
#[serial]
async fn given_502_litellm_when_provision_then_err_and_no_secret() {
    // covers AC-4
    with_temp_goose_root(|| async {
        let provision = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/keys/provision"))
            .respond_with(ResponseTemplate::new(502).set_body_json(json!({
                "error": "litellm_unavailable"
            })))
            .mount(&provision)
            .await;

        let err = complete_oauth_from_access_token(
            "ok-jwt",
            &format!("{}/keys/provision", provision.uri()),
        )
        .await
        .expect_err("502");
        assert!(matches!(err, ProvisionError::Unavailable));
        assert!(!has_configured_key());
    })
    .await;
}

#[tokio::test]
#[serial]
async fn given_no_api_key_when_from_env_then_ok_and_stream_auth_error() {
    // covers AC-5
    with_temp_goose_root(|| async {
        assert!(!has_configured_key());
        let provider = AvocadoProvider::from_env(None)
            .await
            .expect("from_env must succeed without key so configure_oauth can run");
        let model = ModelConfig::new("anthropic/claude-sonnet-4.6");
        let result = provider.stream(&model, "sys", &[], &[]).await;
        let err = match result {
            Err(e) => e,
            Ok(_) => panic!("stream without key should fail"),
        };
        let msg = err.to_string().to_lowercase();
        assert!(
            msg.contains("auth") || msg.contains("not configured") || msg.contains("api key"),
            "expected auth/not-configured error, got {err}"
        );
    })
    .await;
}

/// Auth-critical: 401/403 mapping must be stable across repeated runs.
#[tokio::test]
#[serial]
async fn given_auth_negatives_when_run_10_times_then_zero_failures() {
    // covers AC-4 reliability
    for i in 0..10 {
        with_temp_goose_root(|| async {
            let provision = MockServer::start().await;
            Mock::given(method("POST"))
                .and(path("/keys/provision"))
                .respond_with(ResponseTemplate::new(401).set_body_json(json!({
                    "error": "invalid_token"
                })))
                .mount(&provision)
                .await;
            let err = complete_oauth_from_access_token(
                &format!("bad-{i}"),
                &format!("{}/keys/provision", provision.uri()),
            )
            .await
            .expect_err("401");
            assert!(matches!(err, ProvisionError::Unauthorized));
            assert!(!has_configured_key());

            let provision403 = MockServer::start().await;
            Mock::given(method("POST"))
                .and(path("/keys/provision"))
                .respond_with(ResponseTemplate::new(403).set_body_json(json!({
                    "error": "forbidden",
                    "detail": "Missing required role: agent-access"
                })))
                .mount(&provision403)
                .await;
            let err = complete_oauth_from_access_token(
                &format!("roleless-{i}"),
                &format!("{}/keys/provision", provision403.uri()),
            )
            .await
            .expect_err("403");
            assert!(matches!(err, ProvisionError::Forbidden { .. }));
            assert!(!has_configured_key());
        })
        .await;
    }
}
