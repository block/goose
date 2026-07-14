//! Shared logic for building a [`CanonicalModelRegistry`] from the upstream
//! models.dev inventory.
//!
//! This lives in the library (rather than only in the `build_canonical_models`
//! binary) so both the offline build step and the runtime dynamic refresh
//! (see [`super::dynamic`]) parse models.dev identically.

use super::{
    canonical_name, CanonicalModel, CanonicalModelRegistry, Limit, Modalities, Modality, Pricing,
    ThinkingMode,
};
use anyhow::{Context, Result};
use serde_json::Value;
use std::collections::BTreeMap;

/// Default models.dev inventory endpoint (raw, provider-nested shape). Used by
/// the offline builder / CI to produce the canonical inventory.
pub const MODELS_DEV_API_URL: &str = "https://models.dev/api.json";

/// Default published canonical inventory endpoint.
///
/// This is the flat `Vec<CanonicalModel>` produced by the offline builder and
/// published by CI as a rolling GitHub Release asset (no commits to the repo).
/// It already reflects goose's curated overrides and variant-collision
/// resolution, so the runtime consumes it directly rather than re-deriving from
/// the raw models.dev shape.
pub const CANONICAL_MODELS_URL: &str =
    "https://github.com/aaif-goose/goose/releases/download/canonical-models/canonical_models.json";

const DEFAULT_CONTEXT_LIMIT: usize = 128_000;

/// Build a [`CanonicalModelRegistry`] from a flat `Vec<CanonicalModel>` JSON
/// document (the published canonical format, identical to the bundled file).
pub fn registry_from_canonical_json(json: &str) -> Result<CanonicalModelRegistry> {
    let models: Vec<CanonicalModel> =
        serde_json::from_str(json).context("Failed to parse canonical models JSON")?;

    let mut registry = CanonicalModelRegistry::new();
    for model in models {
        if let Some((provider, model_name)) = model.id.split_once('/') {
            let (provider, model_name) = (provider.to_string(), model_name.to_string());
            registry.register(&provider, &model_name, model);
        }
    }
    Ok(registry)
}

/// Fetch the published canonical inventory (flat format) from `url`.
pub async fn fetch_canonical_json(url: &str) -> Result<String> {
    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .header("User-Agent", "goose/canonical-fetch")
        .send()
        .await
        .context("Failed to fetch canonical inventory")?
        .error_for_status()
        .context("canonical inventory endpoint returned an error status")?;

    response
        .text()
        .await
        .context("Failed to read canonical inventory body")
}

/// Normalize a models.dev provider id to the canonical provider namespace goose
/// uses internally (e.g. `xai` -> `x-ai`).
pub fn normalize_provider_name(provider: &str) -> &str {
    match provider {
        "llama" => "meta-llama",
        "xai" => "x-ai",
        "mistral" => "mistralai",
        _ => provider,
    }
}

/// Fetch the raw models.dev inventory JSON from `url`.
pub async fn fetch_models_dev(url: &str) -> Result<Value> {
    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .header("User-Agent", "goose/canonical-builder")
        .send()
        .await
        .context("Failed to fetch from models.dev API")?
        .error_for_status()
        .context("models.dev API returned an error status")?;

    response
        .json()
        .await
        .context("Failed to parse models.dev response")
}

fn get_string(value: &Value, field: &str) -> Option<String> {
    value.get(field).and_then(|v| v.as_str()).map(String::from)
}

fn get_thinking_mode(canonical_id: &str, value: &Value) -> Option<ThinkingMode> {
    value
        .get("thinking_mode")
        .and_then(|v| v.as_str())
        .and_then(|mode| serde_json::from_value(Value::String(mode.to_string())).ok())
        .or_else(|| inferred_thinking_mode(canonical_id))
}

/// Curated overrides for thinking mode where models.dev is incomplete.
///
/// This is the one place to record manually-probed provider facts, rather than
/// scattering `name.contains(...)` branches through provider request-builders.
fn inferred_thinking_mode(canonical_id: &str) -> Option<ThinkingMode> {
    match canonical_id {
        "anthropic/claude-fable-5" => Some(ThinkingMode::AlwaysOnAdaptive),
        "anthropic/claude-opus-4.6" => Some(ThinkingMode::Adaptive),
        "anthropic/claude-opus-4.7" => Some(ThinkingMode::Adaptive),
        "anthropic/claude-opus-4.8" => Some(ThinkingMode::Adaptive),
        "anthropic/claude-sonnet-4.6" => Some(ThinkingMode::Adaptive),
        "anthropic/claude-sonnet-5" => Some(ThinkingMode::Adaptive),
        _ => None,
    }
}

fn parse_modalities(model_data: &Value, field: &str) -> Vec<Modality> {
    model_data
        .get("modalities")
        .and_then(|m| m.get(field))
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .filter_map(|s| serde_json::from_value(Value::String(s.to_string())).ok())
                .collect()
        })
        .unwrap_or_else(|| vec![Modality::Text])
}

/// Convert a single models.dev model entry into a [`CanonicalModel`], returning
/// the canonical short name (provider-stripped) alongside it.
pub fn process_model(
    model_id: &str,
    model_data: &Value,
    normalized_provider: &str,
) -> Result<(String, CanonicalModel)> {
    let name = model_data["name"]
        .as_str()
        .with_context(|| format!("Model {} missing name", model_id))?;

    let canonical_id = canonical_name(normalized_provider, model_id);

    let modalities = Modalities {
        input: parse_modalities(model_data, "input"),
        output: parse_modalities(model_data, "output"),
    };

    let cost = match model_data.get("cost") {
        Some(c) if !c.is_null() => Pricing {
            input: c.get("input").and_then(|v| v.as_f64()),
            output: c.get("output").and_then(|v| v.as_f64()),
            cache_read: c.get("cache_read").and_then(|v| v.as_f64()),
            cache_write: c.get("cache_write").and_then(|v| v.as_f64()),
        },
        _ => Pricing::default(),
    };

    let limit = Limit {
        context: model_data
            .get("limit")
            .and_then(|l| l.get("context"))
            .and_then(|v| v.as_u64())
            .unwrap_or(DEFAULT_CONTEXT_LIMIT as u64) as usize,
        output: model_data
            .get("limit")
            .and_then(|l| l.get("output"))
            .and_then(|v| v.as_u64())
            .map(|v| v as usize),
    };

    let canonical_model = CanonicalModel {
        id: canonical_id.clone(),
        name: name.to_string(),
        family: get_string(model_data, "family"),
        attachment: model_data.get("attachment").and_then(|v| v.as_bool()),
        reasoning: model_data.get("reasoning").and_then(|v| v.as_bool()),
        thinking_mode: get_thinking_mode(&canonical_id, model_data),
        tool_call: model_data
            .get("tool_call")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        temperature: model_data.get("temperature").and_then(|v| v.as_bool()),
        knowledge: get_string(model_data, "knowledge"),
        release_date: get_string(model_data, "release_date"),
        last_updated: get_string(model_data, "last_updated"),
        modalities,
        open_weights: model_data.get("open_weights").and_then(|v| v.as_bool()),
        cost,
        limit,
    };

    let model_name = canonical_id
        .strip_prefix(&format!("{}/", normalized_provider))
        .unwrap_or(model_id)
        .to_string();

    Ok((model_name, canonical_model))
}

/// When multiple provider model ids collapse to the same canonical key, prefer
/// the shortest id (the unversioned alias), then the most recently updated /
/// released, breaking ties lexicographically.
pub fn pick_winning_variant(variants: &[(String, CanonicalModel)]) -> usize {
    variants
        .iter()
        .enumerate()
        .min_by(|(_, (id_a, a)), (_, (id_b, b))| {
            id_a.len()
                .cmp(&id_b.len())
                .then_with(|| b.last_updated.cmp(&a.last_updated))
                .then_with(|| b.release_date.cmp(&a.release_date))
                .then_with(|| id_a.cmp(id_b))
        })
        .map(|(idx, _)| idx)
        .unwrap_or(0)
}

/// Build a [`CanonicalModelRegistry`] from a parsed models.dev inventory value.
///
/// `on_collision` is invoked whenever multiple provider model ids collide on a
/// canonical key, so callers (e.g. the offline builder) can log the resolution.
pub fn registry_from_models_dev(
    json: &Value,
    mut on_collision: impl FnMut(&str, &str, &[(String, CanonicalModel)], &str),
) -> Result<CanonicalModelRegistry> {
    let providers_obj = json
        .as_object()
        .context("Expected object in models.dev response")?;

    let mut registry = CanonicalModelRegistry::new();

    for (provider_key, provider_data) in providers_obj {
        let models = match provider_data.get("models").and_then(|v| v.as_object()) {
            Some(m) => m,
            None => continue,
        };

        let normalized_provider = normalize_provider_name(provider_key);

        let mut candidates: BTreeMap<String, Vec<(String, CanonicalModel)>> = BTreeMap::new();
        for (model_id, model_data) in models {
            let (model_name, canonical_model) =
                process_model(model_id, model_data, normalized_provider)?;
            candidates
                .entry(model_name)
                .or_default()
                .push((model_id.clone(), canonical_model));
        }

        for (canonical_key, mut variants) in candidates {
            let winner = if variants.len() == 1 {
                variants.pop().unwrap().1
            } else {
                let chosen_idx = pick_winning_variant(&variants);
                let chosen_id = variants[chosen_idx].0.clone();
                on_collision(normalized_provider, &canonical_key, &variants, &chosen_id);
                variants.swap_remove(chosen_idx).1
            };
            registry.register(normalized_provider, &canonical_key, winner);
        }
    }

    Ok(registry)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canonical::{Limit, Modalities, Pricing};

    fn variant(id: &str, release: Option<&str>, updated: Option<&str>) -> (String, CanonicalModel) {
        (
            id.to_string(),
            CanonicalModel {
                id: format!("openai/{}", id),
                name: id.to_string(),
                family: None,
                attachment: None,
                reasoning: None,
                thinking_mode: None,
                tool_call: false,
                temperature: None,
                knowledge: None,
                release_date: release.map(String::from),
                last_updated: updated.map(String::from),
                modalities: Modalities::default(),
                open_weights: None,
                cost: Pricing::default(),
                limit: Limit::default(),
            },
        )
    }

    #[test]
    fn shortest_variant_wins() {
        let variants = vec![
            variant("gpt-4o-2024-08-06", Some("2024-08-06"), Some("2024-08-06")),
            variant("gpt-4o", Some("2024-05-13"), Some("2024-08-06")),
            variant("gpt-4o-2024-11-20", Some("2024-11-20"), Some("2024-11-20")),
            variant("gpt-4o-2024-05-13", Some("2024-05-13"), Some("2024-05-13")),
        ];
        let idx = pick_winning_variant(&variants);
        assert_eq!(variants[idx].0, "gpt-4o");

        let variants = vec![
            variant(
                "claude-haiku-4-5-20251001",
                Some("2025-10-16"),
                Some("2025-10-16"),
            ),
            variant("claude-haiku-4-5", Some("2025-10-16"), Some("2025-10-16")),
        ];
        let idx = pick_winning_variant(&variants);
        assert_eq!(variants[idx].0, "claude-haiku-4-5");
    }
}
