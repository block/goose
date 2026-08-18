//! covers AC-2, AC-3 — avocado catalog → inventory alias/subtext/order

use goose::providers::avocado::{
    catalog_url_from_provision_url, clear_last_fetched_catalog, fetch_model_catalog_from_url,
    take_last_fetched_catalog, AVOCADO_KNOWN_MODELS, AVOCADO_PROVIDER_NAME,
};
use goose::providers::inventory::{InventoryIdentity, ProviderInventoryService};
use goose::session::session_manager::SessionStorage;
use serde_json::json;
use serial_test::serial;
use std::sync::Arc;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[test]
fn given_provision_url_when_deriving_catalog_url_then_replaces_keys_path() {
    assert_eq!(
        catalog_url_from_provision_url("https://dev.avocado.tech/llm-api/keys/provision"),
        "https://dev.avocado.tech/llm-api/models/catalog"
    );
    assert_eq!(
        catalog_url_from_provision_url("http://127.0.0.1:3001/keys/provision"),
        "http://127.0.0.1:3001/models/catalog"
    );
}

#[tokio::test]
#[serial]
async fn given_catalog_endpoint_when_inventory_refresh_then_lists_alias_subtext_in_order() {
    // covers AC-2, AC-3
    clear_last_fetched_catalog();
    let catalog_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/models/catalog"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "provider": "avocado",
            "defaultModel": "zeta/first",
            "models": [
                {
                    "name": "zeta/first",
                    "alias": "Zeta First",
                    "subtext": "Should stay first"
                },
                {
                    "name": "alpha/second",
                    "alias": "Zeta Model QA",
                    "subtext": "Distinct alias for anti-hardcode"
                }
            ]
        })))
        .mount(&catalog_server)
        .await;

    let catalog_url = format!("{}/models/catalog", catalog_server.uri());
    let catalog = fetch_model_catalog_from_url(&catalog_url)
        .await
        .expect("catalog fetch");
    assert_eq!(catalog.models[0].name, "zeta/first");
    assert_eq!(catalog.models[1].alias, "Zeta Model QA");
    assert!(take_last_fetched_catalog().is_some());

    let temp_dir = tempfile::tempdir().unwrap();
    let service =
        ProviderInventoryService::new(Arc::new(SessionStorage::new(temp_dir.path().to_path_buf())));
    let identity = InventoryIdentity {
        provider_id: AVOCADO_PROVIDER_NAME.to_string(),
        provider_family: AVOCADO_PROVIDER_NAME.to_string(),
        inventory_key: "avocado-catalog-test".to_string(),
    };
    let ids: Vec<String> = catalog.models.iter().map(|m| m.name.clone()).collect();
    service
        .store_refreshed_models_preferring_catalog(AVOCADO_PROVIDER_NAME, &identity, &ids)
        .await
        .expect("store catalog");

    let snapshot = service
        .read_snapshot(&identity)
        .await
        .expect("read ok")
        .expect("snapshot present");
    assert_eq!(snapshot.models.len(), 2);
    assert_eq!(snapshot.models[0].id, "zeta/first");
    assert_eq!(snapshot.models[0].alias.as_deref(), Some("Zeta First"));
    assert_eq!(
        snapshot.models[0].subtext.as_deref(),
        Some("Should stay first")
    );
    assert_eq!(snapshot.models[1].id, "alpha/second");
    assert_eq!(snapshot.models[1].alias.as_deref(), Some("Zeta Model QA"));
    // Not alphabetical — zeta before alpha
    assert_ne!(snapshot.models[0].id, "alpha/second");
}

#[tokio::test]
#[serial]
async fn given_catalog_unreachable_when_refresh_then_falls_back_to_known_models_ids_only() {
    // covers AC-2 negative
    clear_last_fetched_catalog();
    let catalog_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/models/catalog"))
        .respond_with(ResponseTemplate::new(500).set_body_json(json!({
            "error": "catalog_unavailable"
        })))
        .mount(&catalog_server)
        .await;

    let catalog_url = format!("{}/models/catalog", catalog_server.uri());
    let err = fetch_model_catalog_from_url(&catalog_url)
        .await
        .expect_err("catalog should fail");
    assert!(err.to_string().contains("catalog") || err.to_string().contains("500"));
    assert!(take_last_fetched_catalog().is_none());

    let temp_dir = tempfile::tempdir().unwrap();
    let service =
        ProviderInventoryService::new(Arc::new(SessionStorage::new(temp_dir.path().to_path_buf())));
    let identity = InventoryIdentity {
        provider_id: AVOCADO_PROVIDER_NAME.to_string(),
        provider_family: AVOCADO_PROVIDER_NAME.to_string(),
        inventory_key: "avocado-fallback-test".to_string(),
    };
    let ids: Vec<String> = AVOCADO_KNOWN_MODELS
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    service
        .store_refreshed_models_preferring_catalog(AVOCADO_PROVIDER_NAME, &identity, &ids)
        .await
        .expect("store fallback");

    let snapshot = service
        .read_snapshot(&identity)
        .await
        .expect("read ok")
        .expect("snapshot present");
    assert!(!snapshot.models.is_empty());
    assert_eq!(snapshot.models[0].id, AVOCADO_KNOWN_MODELS[0]);
    assert!(snapshot.models[0].alias.is_none());
    assert!(snapshot.models[0].subtext.is_none());
}
