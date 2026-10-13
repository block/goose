#![cfg(feature = "hf-hub")]

use goose_local_inference::recommendations::catalogue::{self, Catalogue};

fn sample() -> Catalogue {
    let mut catalogue = catalogue::bundled().unwrap().clone();
    catalogue.models.truncate(1);
    catalogue.models[0].variants.truncate(1);
    catalogue
}

#[test]
fn bundled_catalogue_is_current_and_includes_models_larger_than_a_small_machine() {
    let catalogue = catalogue::bundled().unwrap();
    catalogue::validate(catalogue).unwrap();
    assert!(catalogue
        .models
        .iter()
        .flat_map(|model| &model.variants)
        .any(|item| item.variant.size_bytes > 32 << 30));
}

#[test]
fn stale_policy_engine_profile_and_invalid_bounds_are_rejected() {
    let mut catalogue = sample();
    catalogue.policy_version += 1;
    assert!(catalogue::validate(&catalogue).is_err());
    let mut catalogue = sample();
    catalogue.eredu_revision = "0".repeat(40);
    assert!(catalogue::validate(&catalogue).is_err());
    let mut catalogue = sample();
    catalogue.profile.input_tokens += 1;
    assert!(catalogue::validate(&catalogue).is_err());
    let mut catalogue = sample();
    catalogue.models[0].variants[0].memory.upper_bytes = Some(0);
    assert!(catalogue::validate(&catalogue).is_err());
}

#[test]
fn manifest_requires_safe_unique_files_and_an_immutable_revision() {
    let mut catalogue = sample();
    catalogue.models[0].revision = "main".into();
    assert!(catalogue::validate(&catalogue).is_err());
    let mut catalogue = sample();
    catalogue.models[0].variants[0].files[0].name = "../model.gguf".into();
    assert!(catalogue::validate(&catalogue).is_err());
    let mut catalogue = sample();
    let files = &mut catalogue.models[0].variants[0].files;
    files.push(files[0].clone());
    assert!(catalogue::validate(&catalogue).is_err());
}

#[cfg(all(feature = "mlx", target_os = "macos", target_arch = "aarch64"))]
#[test]
fn offline_forecasts_reapply_memory_and_preserve_unknown_bounds() {
    use goose_sdk_types::custom_requests::LocalInferenceMemoryFit as Fit;
    let catalogue = catalogue::bundled().unwrap();
    let (model, item) = catalogue
        .models
        .iter()
        .flat_map(|model| model.variants.iter().map(move |item| (model, item)))
        .find(|(model, item)| {
            catalogue::assess(model, item, 1 << 50).unwrap().fit == Fit::LikelyFit
        })
        .unwrap();
    assert_eq!(
        catalogue::assess(model, item, 1).unwrap().fit,
        Fit::LikelyShortfall
    );
    assert_eq!(catalogue::assess(model, item, 0).unwrap().fit, Fit::Unknown);
    assert_eq!(
        catalogue::assess(model, item, 1 << 50)
            .unwrap()
            .revision
            .as_deref(),
        Some(model.revision.as_str())
    );
    let mut unbounded = item.clone();
    unbounded.memory.upper_bytes = None;
    assert_eq!(
        catalogue::assess(model, &unbounded, 1 << 50).unwrap().fit,
        Fit::Unknown
    );
}

#[cfg(all(feature = "mlx", target_os = "macos", target_arch = "aarch64"))]
#[tokio::test]
async fn landing_page_and_catalogue_variants_need_no_hub() {
    let models = goose_local_inference::recommendations::discover("", 20)
        .await
        .unwrap();
    assert!(!models.is_empty());
    let repo = &models[0].repo_id;
    let (variants, quality, assessments) =
        goose_local_inference::recommendations::repo_variants(repo, 1 << 50, true, None)
            .await
            .unwrap();
    assert!(quality.is_some());
    assert_eq!(variants.len(), assessments.len());
    assert!(assessments
        .iter()
        .all(|assessment| assessment.revision.is_some()));
}

#[test]
fn fit_boundaries_preserve_reserve_straddling_and_unknown_information() {
    use catalogue::{memory_fit, MemoryForecast};
    use goose_local_inference::recommendations::RESERVE_BYTES;
    use goose_sdk_types::custom_requests::LocalInferenceMemoryFit as Fit;
    let mut memory = MemoryForecast {
        lower_bytes: 100,
        upper_bytes: Some(200),
        uncertainties: vec![],
    };
    assert_eq!(memory_fit(&memory, 0), Fit::Unknown);
    assert_eq!(
        memory_fit(&memory, RESERVE_BYTES + 99),
        Fit::LikelyShortfall
    );
    assert_eq!(memory_fit(&memory, RESERVE_BYTES + 100), Fit::Unknown);
    assert_eq!(memory_fit(&memory, RESERVE_BYTES + 199), Fit::Unknown);
    assert_eq!(memory_fit(&memory, RESERVE_BYTES + 200), Fit::LikelyFit);
    memory.upper_bytes = None;
    assert_eq!(memory_fit(&memory, u64::MAX), Fit::Unknown);
    memory.lower_bytes = u64::MAX;
    assert_eq!(memory_fit(&memory, u64::MAX), Fit::LikelyShortfall);
}

#[cfg(all(feature = "mlx", target_os = "macos", target_arch = "aarch64"))]
#[test]
fn isolated_metadata_worker_reports_incompatibility_without_network_or_weights() {
    use base64::{engine::general_purpose::STANDARD, Engine};
    use std::{
        io::Write,
        process::{Command, Stdio},
    };
    for config in [
        Some(STANDARD.encode(br#"{"model_type":"goose_unsupported_fixture"}"#)),
        None,
    ] {
        let mut worker = Command::new(env!("CARGO_BIN_EXE_local-model-catalogue"))
            .arg("--inspect-metadata")
            .env("HTTPS_PROXY", "http://127.0.0.1:9")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let input = serde_json::json!({
            "provenance": {"source": "test/unsupported", "revision": "a".repeat(40)},
            "config": config, "index": null, "headers": [], "projector": [], "sidecars": {}
        });
        worker
            .stdin
            .take()
            .unwrap()
            .write_all(&serde_json::to_vec(&input).unwrap())
            .unwrap();
        let output = worker.wait_with_output().unwrap();
        assert!(output.status.success());
        let outcome: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert!(!outcome["Incompatible"].as_array().unwrap().is_empty());
    }
}

#[test]
fn inherited_scores_require_matching_recorded_checkpoint_identities() {
    let mut catalogue = catalogue::bundled().unwrap().clone();
    catalogue.models.retain(|model| model.quality.inherited);
    catalogue.models.truncate(1);
    catalogue.models[0].variants.truncate(1);
    catalogue::validate(&catalogue).unwrap();
    let original = catalogue.clone();
    catalogue.models[0].variants[0].lineage = None;
    assert!(catalogue::validate(&catalogue).is_err());
    let mut catalogue = original;
    catalogue.models[0].variants[0]
        .lineage
        .as_mut()
        .unwrap()
        .artifact
        .architecture = "unrelated".into();
    assert!(catalogue::validate(&catalogue).is_err());
}

#[test]
fn catalogue_stores_order_and_provenance_without_scores_or_ranks() {
    let catalogue = catalogue::bundled().unwrap();
    for (position, model) in catalogue.models.iter().enumerate() {
        assert_eq!(model.quality.catalogue_order, position as u64 + 1);
    }
    let encoded = serde_json::to_value(catalogue).unwrap();
    for model in encoded["models"].as_array().unwrap() {
        let quality = model["quality"].as_object().unwrap();
        assert!(!quality.contains_key("catalogueOrder"));
        assert!(!quality.contains_key("evidence"));
        assert!(!quality.contains_key("score"));
        assert!(quality.contains_key("sourceUrl"));
    }
}
