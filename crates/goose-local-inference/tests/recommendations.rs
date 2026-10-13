#![cfg(feature = "hf-hub")]

use goose_local_inference::recommendations::{
    gguf::header_end,
    quality::{compare_quality, quantized_parent},
    recommended_variant, unknown_assessment,
};
use goose_sdk_types::custom_requests::{
    LocalInferenceCompatibility as Compatibility, LocalInferenceHfModelVariantDto as Variant,
    LocalInferenceMemoryFit as Fit, LocalInferenceModelQualityDto as Quality,
    LocalInferenceSuitability as Suitability,
};
use serde_json::json;

#[test]
fn recommendation_requests_are_send() {
    fn assert_send(_: impl Send) {}

    assert_send(async {
        let query = String::from("model");
        goose_local_inference::recommendations::discover(&query, 20).await
    });
    assert_send(async {
        let repo = String::from("org/model");
        let variant = String::from("Q4");
        goose_local_inference::recommendations::repo_variants(&repo, 8 << 30, true, Some(&variant))
            .await
    });
}

fn quality(model: &str, rank: u64) -> Quality {
    Quality {
        evaluated_model_id: model.into(),
        aa_model_id: "aa-model".into(),
        source_url: "https://artificialanalysis.ai/models/model".into(),
        evaluation_name: "Model (reasoning)".into(),
        catalogue_order: rank,
        ..Default::default()
    }
}

fn variant(id: &str, fit: Fit, fidelity: u8) -> Variant {
    let mut assessment = unknown_assessment(8 << 30);
    assessment.fit = fit;
    assessment.compatibility = Compatibility::Compatible;
    assessment.suitability = Suitability::Suitable;
    Variant {
        variant_id: id.into(),
        supported: true,
        quality_rank: fidelity,
        assessment: Some(assessment),
        ..Default::default()
    }
}

#[test]
fn missing_fit_never_turns_into_a_recommendation_even_for_a_tiny_file() {
    let models = vec![
        variant("tiny", Fit::Unknown, 95),
        variant("oversized", Fit::LikelyShortfall, 99),
    ];
    assert_eq!(
        recommended_variant(&models, Some(&quality("org/model", 1))),
        None
    );
}

#[test]
fn recommends_by_stable_variant_id_across_formats_and_preserves_fit_without_quality() {
    let models = vec![
        variant("gguf", Fit::LikelyShortfall, 95),
        variant("safetensors", Fit::LikelyFit, 91),
        variant("Q4", Fit::LikelyFit, 45),
    ];
    assert_eq!(
        recommended_variant(&models, Some(&quality("org/model", 85))),
        Some("safetensors".into())
    );
    assert_eq!(recommended_variant(&models, None), None);
    assert_eq!(models[1].assessment.as_ref().unwrap().fit, Fit::LikelyFit);
}

#[test]
fn unsupported_variants_are_not_recommended_despite_a_memory_fit() {
    let mut model = variant("unsupported", Fit::LikelyFit, 95);
    model.supported = false;
    assert_eq!(
        recommended_variant(&[model], Some(&quality("org/model", 1))),
        None
    );
}

#[test]
fn download_size_cannot_override_a_successful_forecast() {
    let mut model = variant("fits", Fit::LikelyFit, 45);
    model.size_bytes = u64::MAX;
    assert_eq!(
        recommended_variant(&[model], Some(&quality("org/model", 10))),
        Some("fits".into())
    );
}

#[test]
fn scores_transfer_only_through_an_unambiguous_quantization_relationship() {
    assert_eq!(
        quantized_parent(
            &json!({"cardData":{"base_model":"org/instruct","base_model_relation":"quantized"}})
        ),
        Some("org/instruct")
    );
    assert_eq!(
        quantized_parent(
            &json!({"cardData":{"base_model":"org/instruct"},"tags":["base_model:quantized:org/instruct"]})
        ),
        Some("org/instruct")
    );
    for relation in ["finetune", "adapter", "merge"] {
        assert_eq!(
            quantized_parent(
                &json!({"cardData":{"base_model":"org/instruct","base_model_relation":relation},"tags":["base_model:quantized:org/instruct"]})
            ),
            None
        );
    }
    assert_eq!(
        quantized_parent(&json!({"cardData":{"base_model":"org/instruct"}})),
        None
    );
    assert_eq!(
        quantized_parent(
            &json!({"cardData":{"base_model":["org/a","org/b"],"base_model_relation":"quantized"}})
        ),
        None
    );
}

#[test]
fn quality_uses_comparable_ranks_and_has_a_consistent_missing_evidence_order() {
    let high = quality("org/a", 1);
    let low = quality("org/b", 50);
    assert!(compare_quality(Some(&high), Some(&low)).is_lt());
    assert!(compare_quality(Some(&low), None).is_lt());
    assert!(compare_quality(Some(&high), None).is_lt());
    assert!(compare_quality(None, None).is_eq());
}

fn gguf() -> Vec<u8> {
    let mut bytes = b"GGUF".to_vec();
    bytes.extend(3u32.to_le_bytes());
    bytes.extend(1u64.to_le_bytes());
    bytes.extend(0u64.to_le_bytes());
    bytes.extend(6u64.to_le_bytes());
    bytes.extend(b"weight");
    bytes.extend(1u32.to_le_bytes());
    bytes.extend(16u64.to_le_bytes());
    bytes.extend(0u32.to_le_bytes());
    bytes.extend(0u64.to_le_bytes());
    bytes
}

#[test]
fn gguf_framing_accepts_headers_without_payload_and_handles_every_truncation() {
    let bytes = gguf();
    for end in 0..bytes.len() {
        assert!(header_end(&bytes[..end]).unwrap().unwrap_err() > end);
    }
    assert_eq!(header_end(&bytes).unwrap(), Ok(bytes.len()));
    let mut payload = bytes.clone();
    payload.extend([42; 128]);
    assert_eq!(header_end(&payload).unwrap(), Ok(bytes.len()));
}

#[test]
fn untrusted_gguf_lengths_cannot_request_unbounded_metadata() {
    let mut bytes = gguf();
    bytes[24..32].copy_from_slice(&u64::MAX.to_le_bytes());
    assert!(header_end(&bytes).is_err());
    bytes[0] = b'X';
    assert!(header_end(&bytes).is_err());
}

#[test]
fn fitting_models_need_both_suitability_and_compatibility() {
    for (compatibility, suitability) in [
        (Compatibility::Unknown, Suitability::Suitable),
        (Compatibility::Incompatible, Suitability::Suitable),
        (Compatibility::Compatible, Suitability::Unknown),
        (Compatibility::Compatible, Suitability::Unsuitable),
    ] {
        let mut model = variant("Q4", Fit::LikelyFit, 45);
        let assessment = model.assessment.as_mut().unwrap();
        assessment.compatibility = compatibility;
        assessment.suitability = suitability;
        assert_eq!(
            recommended_variant(&[model], Some(&quality("org/model", 1))),
            None
        );
    }
}

#[test]
fn task_and_purpose_override_popularity_chat_templates_and_benchmarks() {
    use goose_local_inference::recommendations::suitability::classify;
    let scored = quality("org/model", 1);
    for info in [
        json!({"pipeline_tag":"text-to-image"}),
        json!({"pipeline_tag":"voice-activity-detection"}),
        json!({"pipeline_tag":"automatic-speech-recognition"}),
        json!({"pipeline_tag":"sentence-similarity"}),
        json!({"pipeline_tag":"text-generation","tags":["reward-model"]}),
        json!({"pipeline_tag":"text-generation","config":{"architectures":["Qwen3ForSequenceClassification"]}}),
        json!({"pipeline_tag":"text-generation","tags":["pretrained"]}),
    ] {
        assert_eq!(
            classify(&info, Some(&scored), true).0,
            Suitability::Unsuitable
        );
    }
}

#[test]
fn task_and_general_knowledge_alone_do_not_prove_conversational_use() {
    use goose_local_inference::recommendations::suitability::classify;
    let info = json!({"pipeline_tag":"text-generation"});
    assert_eq!(
        classify(&info, Some(&quality("org/model", 1)), false).0,
        Suitability::Unknown
    );
    assert_eq!(classify(&json!({}), None, false).0, Suitability::Unknown);
    assert_eq!(classify(&info, None, true).0, Suitability::Suitable);
    for task in [
        "text-generation",
        "image-text-to-text",
        "video-text-to-text",
    ] {
        assert_eq!(
            classify(
                &json!({"pipeline_tag":task,"tags":["instruct","code"]}),
                None,
                false
            )
            .0,
            Suitability::Suitable
        );
    }
}

#[test]
fn quantization_lineage_can_be_tagged_but_must_not_conflict() {
    assert_eq!(
        quantized_parent(&json!({"tags":["base_model:quantized:org/chat"]})),
        Some("org/chat")
    );
    assert_eq!(
        quantized_parent(
            &json!({"tags":["base_model:quantized:org/a","base_model:quantized:org/b"]})
        ),
        None
    );
    assert_eq!(
        quantized_parent(
            &json!({"cardData":{"base_model":"org/a","base_model_relation":"quantized"},"tags":["base_model:quantized:org/b"]})
        ),
        None
    );
}

#[test]
fn catalogue_order_replaces_benchmark_percentiles() {
    let first = quality("org/a", 1);
    let second = quality("org/b", 2);
    assert!(compare_quality(Some(&first), Some(&second)).is_lt());
    assert!(compare_quality(Some(&second), None).is_lt());
    assert!(compare_quality(None, None).is_eq());
    let mut missing = first.clone();
    missing.catalogue_order = 0;
    assert!(compare_quality(Some(&missing), Some(&second)).is_gt());
}

#[test]
fn gguf_architecture_comes_from_metadata_not_repository_names() {
    use goose_local_inference::recommendations::gguf::header_architecture;
    let mut bytes = gguf();
    bytes[16..24].copy_from_slice(&1u64.to_le_bytes());
    let mut metadata = Vec::new();
    metadata.extend(20u64.to_le_bytes());
    metadata.extend(b"general.architecture");
    metadata.extend(8u32.to_le_bytes());
    metadata.extend(6u64.to_le_bytes());
    metadata.extend(b"future");
    let prefix_len = 24 + metadata.len();
    bytes.splice(24..24, metadata);
    assert_eq!(
        goose_local_inference::recommendations::gguf::prefix_architecture(&bytes[..prefix_len])
            .unwrap()
            .as_deref(),
        Some("future")
    );
    assert_eq!(
        header_architecture(&bytes).unwrap().as_deref(),
        Some("future")
    );
    for end in 0..bytes.len() {
        assert!(header_architecture(&bytes[..end]).is_err());
    }
    assert_eq!(header_architecture(&gguf()).unwrap(), None);
}

#[test]
fn conflicting_relation_tags_and_additional_datasets_do_not_inherit_benchmarks() {
    use goose_local_inference::recommendations::quality::preserves_evaluated_datasets;
    assert_eq!(
        quantized_parent(
            &json!({"cardData": {"base_model": "org/model", "base_model_relation": "quantized"}, "tags": ["base_model:quantized:org/model", "base_model:finetune:org/model"]})
        ),
        None
    );
    let parent = json!({"cardData": {"datasets": ["org/original"]}});
    assert!(preserves_evaluated_datasets(
        &json!({"tags": ["dataset:org/original"]}),
        &parent
    ));
    assert!(preserves_evaluated_datasets(&json!({}), &parent));
    assert!(!preserves_evaluated_datasets(
        &json!({"cardData": {"datasets": ["org/original", "org/distillation"]}}),
        &parent
    ));
    assert!(!preserves_evaluated_datasets(
        &json!({"tags": ["dataset:org/calibration"]}),
        &parent
    ));
}

#[test]
fn unbounded_compatible_forecasts_are_eligible_only_when_lower_bound_and_reserve_fit() {
    let mut model = variant("unbounded", Fit::Unknown, 95);
    let quality = quality("org/model", 1);
    let assessment = model.assessment.as_mut().unwrap();
    assessment.lower_bytes = Some(assessment.available_memory_bytes - assessment.reserve_bytes);
    assert_eq!(
        recommended_variant(&[model.clone()], Some(&quality)),
        Some("unbounded".into())
    );
    let bounded = variant("bounded", Fit::LikelyFit, 45);
    assert_eq!(
        recommended_variant(&[model.clone(), bounded], Some(&quality)),
        Some("bounded".into())
    );
    let eligible = model.clone();
    model.assessment.as_mut().unwrap().lower_bytes = Some(u64::MAX);
    assert_eq!(recommended_variant(&[model], Some(&quality)), None);
    let mut model = eligible.clone();
    model.assessment.as_mut().unwrap().available_memory_bytes = 0;
    assert_eq!(recommended_variant(&[model], Some(&quality)), None);
    let mut model = eligible;
    model.assessment.as_mut().unwrap().upper_bytes = Some(16 << 30);
    assert_eq!(recommended_variant(&[model], Some(&quality)), None);
}

#[test]
fn benchmark_lineage_requires_matching_architecture_and_dimensions() {
    use goose_local_inference::recommendations::lineage::Identity;
    let config = json!({"model_type":"qwen3", "hidden_size":4096,
        "num_hidden_layers":32,"num_attention_heads":32,"num_key_value_heads":8,"vocab_size":151936, "intermediate_size":14336, "num_experts":8});
    let evaluated = Identity::from_config(&config).unwrap();
    let gguf = json!({"general.architecture":"qwen3", "qwen3.embedding_length":4096,
        "qwen3.block_count":32,"qwen3.attention.head_count":32,"qwen3.attention.head_count_kv":8,
        "tokenizer.ggml.tokens.length":151936, "qwen3.feed_forward_length":14336, "qwen3.expert_count":8});
    let artifact = Identity::from_gguf(gguf.as_object().unwrap()).unwrap();
    artifact.matches(&evaluated, "gguf").unwrap();
    for field in [
        "hidden_size",
        "num_hidden_layers",
        "num_attention_heads",
        "num_key_value_heads",
        "vocab_size",
        "intermediate_size",
        "num_experts",
    ] {
        let mut altered = config.clone();
        altered[field] = json!(config[field].as_u64().unwrap() + 1);
        assert!(
            artifact
                .matches(&Identity::from_config(&altered).unwrap(), "gguf")
                .is_err(),
            "{field}"
        );
    }
    let mut deepseek = config.clone();
    deepseek["model_type"] = json!("deepseek_v3");
    assert!(artifact
        .matches(&Identity::from_config(&deepseek).unwrap(), "gguf")
        .is_err());
    assert!(Identity::from_config(&json!({"model_type":"qwen3"})).is_err());
    assert!(
        Identity::from_gguf(json!({"general.architecture":"qwen3"}).as_object().unwrap()).is_err()
    );
    let mut missing = evaluated.clone();
    missing.dimensions.clear();
    assert!(artifact.matches(&missing, "gguf").is_err());
    assert_eq!(
        Identity::from_config(&json!({"text_config":config})).unwrap(),
        evaluated
    );
}

#[test]
fn new_modification_claims_do_not_inherit_benchmarks() {
    use goose_local_inference::recommendations::lineage::preserves_modification_claims;
    for child in [
        "org/Model-abliterated-Q5_0",
        "org/Model-Distill-8B",
        "org/Model-DPO",
        "org/Model-merged",
    ] {
        assert!(!preserves_modification_claims(
            child,
            &json!({}),
            "parent/Model",
            &json!({})
        ));
    }
    assert!(!preserves_modification_claims(
        "org/Model-Q4",
        &json!({"tags":["abliterated"]}),
        "parent/Model",
        &json!({})
    ));
    assert!(preserves_modification_claims(
        "org/Model-Distill-Q4",
        &json!({}),
        "parent/Model-Distill",
        &json!({})
    ));
    assert!(preserves_modification_claims(
        "org/Model-Q4",
        &json!({}),
        "parent/Model",
        &json!({})
    ));
    assert!(quantized_parent(
        &json!({"tags":["base_model:quantized:org/a", "base_model:distill:org/a"]})
    )
    .is_none());
}

#[test]
fn gguf_lineage_can_read_dimensions_before_a_large_token_array() {
    use goose_local_inference::recommendations::gguf::prefix_metadata;
    fn string(bytes: &mut Vec<u8>, value: &str) {
        bytes.extend_from_slice(&(value.len() as u64).to_le_bytes());
        bytes.extend_from_slice(value.as_bytes());
    }
    let mut bytes = b"GGUF".to_vec();
    bytes.extend_from_slice(&3u32.to_le_bytes());
    bytes.extend_from_slice(&0u64.to_le_bytes());
    bytes.extend_from_slice(&3u64.to_le_bytes());
    string(&mut bytes, "general.architecture");
    bytes.extend_from_slice(&8u32.to_le_bytes());
    string(&mut bytes, "qwen3");
    string(&mut bytes, "qwen3.block_count");
    bytes.extend_from_slice(&4u32.to_le_bytes());
    bytes.extend_from_slice(&32u32.to_le_bytes());
    string(&mut bytes, "tokenizer.ggml.tokens");
    bytes.extend_from_slice(&9u32.to_le_bytes());
    bytes.extend_from_slice(&8u32.to_le_bytes());
    bytes.extend_from_slice(&151936u64.to_le_bytes());
    let metadata = prefix_metadata(&bytes).unwrap();
    assert_eq!(metadata["general.architecture"], "qwen3");
    assert_eq!(metadata["qwen3.block_count"], 32);
    assert_eq!(metadata["tokenizer.ggml.tokens.length"], 151936);
}
