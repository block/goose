#![cfg(feature = "hf-hub")]

use goose_local_inference::recommendations::{
    gguf::header_end,
    quality::{compare_quality, quantized_parent},
    recommended_variant, unknown_assessment,
};
use goose_sdk_types::custom_requests::{
    LocalInferenceBenchmarkEvidenceDto as Evidence, LocalInferenceHfModelVariantDto as Variant,
    LocalInferenceMemoryFit as Fit, LocalInferenceModelQualityDto as Quality,
};
use serde_json::json;

fn quality(model: &str, rank: u64) -> Quality {
    Quality {
        evaluated_model_id: model.into(),
        evidence: vec![Evidence {
            benchmark: "TIGER-Lab/MMLU-Pro".into(),
            rank,
            participants: 100,
            ..Default::default()
        }],
        ..Default::default()
    }
}

fn variant(id: &str, fit: Fit, fidelity: u8) -> Variant {
    let mut assessment = unknown_assessment(8 << 30);
    assessment.fit = fit;
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
