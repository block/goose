use super::*;

pub fn default_inventory_identity(
    provider_id: &str,
    provider_family: &str,
    config_keys: &[ConfigKey],
    config: &Config,
) -> InventoryIdentityInput {
    let mut identity = InventoryIdentityInput::new(provider_id, provider_family);

    for key in config_keys {
        if key.secret {
            if let Some(value) = config_secret_value(config, &key.name) {
                identity.secret_inputs.insert(key.name.clone(), value);
            }
        } else if let Some(value) = config_param_value(config, &key.name) {
            identity.public_inputs.insert(key.name.clone(), value);
        }
    }

    identity
}

pub fn default_inventory_configured(config_keys: &[ConfigKey], config: &Config) -> bool {
    config_keys.iter().all(|key| {
        if !key.required {
            return true;
        }
        if key.default.is_some() {
            return true;
        }
        if key.secret {
            config.get_secret::<serde_json::Value>(&key.name).is_ok()
        } else {
            config.get_param::<serde_json::Value>(&key.name).is_ok()
        }
    })
}

pub fn declarative_inventory_identity(
    config: &DeclarativeProviderConfig,
) -> Result<InventoryIdentityInput> {
    let global = Config::global();
    let mut identity = InventoryIdentityInput::new(
        config.name.clone(),
        config
            .catalog_provider_id
            .clone()
            .unwrap_or_else(|| match config.engine {
                ProviderEngine::OpenAI => "openai".to_string(),
                ProviderEngine::Anthropic => "anthropic".to_string(),
                ProviderEngine::Ollama => "ollama".to_string(),
            }),
    );

    identity
        .public_inputs
        .insert("base_url".to_string(), config.base_url.clone());

    if let Some(base_path) = &config.base_path {
        identity
            .public_inputs
            .insert("base_path".to_string(), base_path.clone());
    }
    if let Some(catalog_provider_id) = &config.catalog_provider_id {
        identity.public_inputs.insert(
            "catalog_provider_id".to_string(),
            catalog_provider_id.clone(),
        );
    }
    if let Some(dynamic_models) = config.dynamic_models {
        identity
            .public_inputs
            .insert("dynamic_models".to_string(), dynamic_models.to_string());
    }
    identity.public_inputs.insert(
        "skip_canonical_filtering".to_string(),
        config.skip_canonical_filtering.to_string(),
    );
    if !config.models.is_empty() {
        identity.public_inputs.insert(
            "models".to_string(),
            serde_json::to_string(
                &config
                    .models
                    .iter()
                    .map(|model| &model.name)
                    .collect::<Vec<_>>(),
            )?,
        );
    }
    if let Some(headers) = &config.headers {
        identity
            .public_inputs
            .insert("headers".to_string(), serialize_string_map(headers)?);
    }
    if !config.api_key_env.is_empty() {
        if let Some(value) = config_secret_value(global, &config.api_key_env) {
            identity
                .secret_inputs
                .insert(config.api_key_env.clone(), value);
        }
    }

    Ok(identity)
}

pub fn config_param_value(config: &Config, key: &str) -> Option<String> {
    config
        .get_param::<serde_json::Value>(key)
        .ok()
        .and_then(|value| normalize_json_value(&value))
}

pub fn config_secret_value(config: &Config, key: &str) -> Option<String> {
    config
        .get_secret::<serde_json::Value>(key)
        .ok()
        .and_then(|value| normalize_json_value(&value))
}

pub fn serialize_string_map(map: &HashMap<String, String>) -> Result<String> {
    let ordered = map
        .iter()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect::<BTreeMap<_, _>>();
    Ok(serde_json::to_string(&ordered)?)
}

pub(super) fn parse_optional_datetime(value: Option<String>) -> Result<Option<DateTime<Utc>>> {
    value
        .map(|value| value.parse::<DateTime<Utc>>())
        .transpose()
        .map_err(Into::into)
}

pub(super) fn normalize_json_value(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::Null => None,
        serde_json::Value::String(value) if value.is_empty() => None,
        serde_json::Value::String(value) => Some(value.clone()),
        other => serde_json::to_string(other).ok(),
    }
}

pub(super) fn fallback_inventory_identity(provider_id: &str) -> InventoryIdentityInput {
    InventoryIdentityInput::new(
        provider_id.to_string(),
        map_provider_name(provider_id).to_string(),
    )
}

pub(super) fn enrich_model_ids_with_canonical(
    provider_family: &str,
    model_ids: &[String],
) -> Vec<InventoryModel> {
    let mut models: Vec<InventoryModel> = Vec::new();
    let mut seen_names: HashSet<String> = HashSet::new();

    for id in model_ids {
        let model = enriched_model(provider_family, id, None);
        if !seen_names.insert(model.name.clone()) {
            continue;
        }
        models.push(model);
    }

    // For databricks, prefer goose- prefixed model_ids when there are duplicates.
    // Re-scan: if a later model_id with "goose-" prefix maps to the same display name,
    // swap it in.
    if provider_family == "databricks" {
        let mut name_to_idx: HashMap<String, usize> = HashMap::new();
        for (idx, model) in models.iter().enumerate() {
            name_to_idx.insert(model.name.clone(), idx);
        }
        for id in model_ids {
            if !id.starts_with("goose-") {
                continue;
            }
            let candidate = enriched_model(provider_family, id, None);
            if let Some(&idx) = name_to_idx.get(&candidate.name) {
                if !models[idx].id.starts_with("goose-") {
                    models[idx].id = candidate.id;
                }
            }
        }
    }

    // Mark the latest model per recommended family.
    let mut seen_recommended_families: HashSet<String> = HashSet::new();
    for model in &mut models {
        if let Some(family) = &model.family {
            if RECOMMENDED_FAMILIES.contains(&family.as_str())
                && seen_recommended_families.insert(family.clone())
            {
                model.recommended = true;
            }
        }
    }

    models
}

pub(super) fn configured_models_to_inventory(
    provider_family: &str,
    models: &[ModelInfo],
) -> Vec<InventoryModel> {
    let mut result: Vec<InventoryModel> = Vec::new();
    let mut seen_names: HashSet<String> = HashSet::new();
    for model in models {
        let enriched = enriched_model(provider_family, &model.name, Some(model.context_limit));
        if seen_names.insert(enriched.name.clone()) {
            result.push(enriched);
        }
    }

    let mut seen_recommended_families: HashSet<String> = HashSet::new();
    for model in &mut result {
        if let Some(family) = &model.family {
            if RECOMMENDED_FAMILIES.contains(&family.as_str())
                && seen_recommended_families.insert(family.clone())
            {
                model.recommended = true;
            }
        }
    }

    result
}

pub(super) fn inventory_models_from_snapshot(
    snapshot: Option<&InventorySnapshot>,
    provider_family: &str,
    configured_models: &[ModelInfo],
) -> Vec<InventoryModel> {
    match snapshot {
        Some(snapshot) if !snapshot.models.is_empty() || snapshot.last_updated_at.is_some() => {
            snapshot.models.clone()
        }
        _ => configured_models_to_inventory(provider_family, configured_models),
    }
}

pub(super) fn enriched_model(
    provider_family: &str,
    model_id: &str,
    fallback_context_limit: Option<usize>,
) -> InventoryModel {
    let registry = CanonicalModelRegistry::bundled().ok();
    let canonical = registry.as_ref().and_then(|registry| {
        let canonical_id = map_to_canonical_model(provider_family, model_id, registry)?;
        let (provider, model) = canonical_id.split_once('/')?;
        registry.get(provider, model).cloned()
    });

    InventoryModel {
        id: model_id.to_string(),
        name: canonical
            .as_ref()
            .map(|model| model.name.clone())
            .unwrap_or_else(|| model_id.to_string()),
        family: canonical.as_ref().and_then(|model| model.family.clone()),
        context_limit: canonical
            .as_ref()
            .map(|model| model.limit.context)
            .or(fallback_context_limit),
        reasoning: canonical.as_ref().and_then(|model| model.reasoning),
        recommended: false,
    }
}
