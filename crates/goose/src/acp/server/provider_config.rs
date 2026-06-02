use super::*;

pub fn builtin_to_extension_config(name: &str) -> ExtensionConfig {
    if let Some(def) = PLATFORM_EXTENSIONS.get(name) {
        ExtensionConfig::Platform {
            name: def.name.into(),
            description: def.description.into(),
            display_name: Some(def.display_name.into()),
            bundled: Some(true),
            available_tools: vec![],
        }
    } else {
        ExtensionConfig::Builtin {
            name: name.into(),
            display_name: None,
            timeout: None,
            bundled: Some(true),
            description: name.into(),
            available_tools: vec![],
        }
    }
}

pub fn build_model_state(
    current_model: &str,
    inventory: &ProviderInventoryEntry,
) -> SessionModelState {
    let mut available_models = inventory
        .models
        .iter()
        .map(|model| ModelInfo::new(ModelId::new(model.id.as_str()), model.name.as_str()))
        .collect::<Vec<_>>();
    if !available_models
        .iter()
        .any(|model| model.model_id.0.as_ref() == current_model)
    {
        available_models.insert(
            0,
            ModelInfo::new(ModelId::new(current_model), current_model),
        );
    }
    SessionModelState::new(ModelId::new(current_model), available_models)
}

pub struct ProviderOptionEntry {
    pub id: String,
    pub label: String,
}

pub async fn list_provider_entries(current_provider: Option<&str>) -> Vec<ProviderOptionEntry> {
    let mut providers = crate::providers::providers()
        .await
        .into_iter()
        .map(|(metadata, _)| ProviderOptionEntry {
            id: metadata.name,
            label: metadata.display_name,
        })
        .collect::<Vec<_>>();
    providers.sort_by(|left, right| left.id.cmp(&right.id));
    providers.dedup_by(|left, right| left.id == right.id);

    if let Some(current_provider) = current_provider {
        if current_provider != DEFAULT_PROVIDER_ID
            && !providers
                .iter()
                .any(|provider| provider.id == current_provider)
        {
            providers.push(ProviderOptionEntry {
                id: current_provider.to_string(),
                label: current_provider.to_string(),
            });
            providers.sort_by(|left, right| left.id.cmp(&right.id));
        }
    }

    let mut entries = Vec::with_capacity(providers.len() + 1);
    entries.push(ProviderOptionEntry {
        id: DEFAULT_PROVIDER_ID.to_string(),
        label: DEFAULT_PROVIDER_LABEL.to_string(),
    });
    entries.extend(providers);
    entries
}

pub async fn build_provider_options(
    current_provider: Option<&str>,
) -> Vec<SessionConfigSelectOption> {
    list_provider_entries(current_provider)
        .await
        .into_iter()
        .map(|provider| SessionConfigSelectOption::new(provider.id, provider.label))
        .collect()
}

pub fn session_provider_selection(session: &Session) -> &str {
    session
        .provider_name
        .as_deref()
        .unwrap_or(DEFAULT_PROVIDER_ID)
}

/// Resolve the provider name and model config for a session from an
/// already-loaded `Config`.
pub async fn resolve_provider_and_model_from_config(
    config: &Config,
    goose_session: &Session,
) -> Result<(String, crate::model::ModelConfig), String> {
    let global_provider = config.get_goose_provider().ok();
    let provider_override = goose_session
        .provider_name
        .as_deref()
        .filter(|p| *p != DEFAULT_PROVIDER_ID);
    let provider_name = provider_override
        .map(ToOwned::to_owned)
        .or_else(|| global_provider.clone())
        .ok_or_else(|| "Missing provider".to_string())?;
    let explicitly_switched =
        provider_override.is_some() && provider_override != global_provider.as_deref();
    let model_config = match &goose_session.model_config {
        Some(mc) => mc.clone(),
        None if explicitly_switched => {
            let entry = crate::providers::get_from_registry(&provider_name)
                .await
                .map_err(|e| e.to_string())?;
            let default_model = &entry.metadata().default_model;
            crate::model::ModelConfig::new(default_model)
                .map_err(|e| e.to_string())?
                .with_canonical_limits(&provider_name)
        }
        None => {
            let model_id = config.get_goose_model().map_err(|e| e.to_string())?;
            crate::model::ModelConfig::new(&model_id)
                .map_err(|e| e.to_string())?
                .with_canonical_limits(&provider_name)
        }
    };
    Ok((provider_name, model_config))
}

pub fn with_preserved_session_request_params(
    mut model_config: crate::model::ModelConfig,
    current_model_config: Option<&crate::model::ModelConfig>,
    request_params: Option<HashMap<String, serde_json::Value>>,
) -> crate::model::ModelConfig {
    let has_model_effort = model_config
        .request_params
        .as_ref()
        .and_then(|params| params.get("thinking_effort"))
        .is_some();
    if !has_model_effort {
        if let Some(thinking_effort) = current_model_config
            .and_then(|config| config.request_params.as_ref())
            .and_then(|params| params.get("thinking_effort"))
            .cloned()
        {
            model_config = model_config.with_merged_request_params(HashMap::from([(
                "thinking_effort".into(),
                thinking_effort,
            )]));
        }
    }
    if let Some(request_params) = request_params {
        model_config = model_config.with_merged_request_params(request_params);
    }
    model_config
}

/// Convenience wrapper: reads config from disk, then resolves provider + model.
/// Cheap enough to call from `on_new_session` (file + registry reads, no network).
pub async fn resolve_provider_and_model(
    config_dir: &std::path::Path,
    goose_session: &Session,
) -> Result<(String, crate::model::ModelConfig), String> {
    let config =
        Config::new(config_dir.join(CONFIG_YAML_NAME), "goose").map_err(|e| e.to_string())?;
    resolve_provider_and_model_from_config(&config, goose_session).await
}

pub fn build_mode_state(
    current_mode: GooseMode,
) -> Result<SessionModeState, agent_client_protocol::Error> {
    let mut available = Vec::with_capacity(GooseMode::VARIANTS.len());
    for &name in GooseMode::VARIANTS {
        let goose_mode: GooseMode = name.parse().map_err(|_| {
            agent_client_protocol::Error::internal_error() // impossible but satisfy linters
                .data(format!("Failed to parse GooseMode variant: {}", name))
        })?;
        let mut mode = SessionMode::new(SessionModeId::new(name), name);
        mode.description = goose_mode.get_message().map(Into::into);
        available.push(mode);
    }
    Ok(SessionModeState::new(
        SessionModeId::new(current_mode.to_string()),
        available,
    ))
}

pub fn should_refresh_inventory_for_session_init(entry: &ProviderInventoryEntry) -> bool {
    entry.configured
        && entry.supports_refresh
        && (entry.last_updated_at.is_none() || ProviderInventoryService::is_stale(entry))
}

pub async fn build_eager_config_from_inventory(
    provider_name: &str,
    current_model: &str,
    inventory: &ProviderInventoryEntry,
    mode_state: &SessionModeState,
    goose_session: &Session,
) -> (SessionModelState, Vec<SessionConfigOption>) {
    let ms = build_model_state(current_model, inventory);
    let provider_selection = session_provider_selection(goose_session);
    let provider_options = build_provider_options(Some(provider_name)).await;
    let config_options =
        build_config_options(mode_state, &ms, provider_selection, provider_options);
    (ms, config_options)
}

pub fn build_config_options(
    mode_state: &SessionModeState,
    model_state: &SessionModelState,
    provider_selection: &str,
    provider_options: Vec<SessionConfigSelectOption>,
) -> Vec<SessionConfigOption> {
    let mode_options: Vec<SessionConfigSelectOption> = mode_state
        .available_modes
        .iter()
        .map(|m| {
            SessionConfigSelectOption::new(m.id.0.clone(), m.name.clone())
                .description(m.description.clone())
        })
        .collect();
    let model_options: Vec<SessionConfigSelectOption> = model_state
        .available_models
        .iter()
        .map(|m| SessionConfigSelectOption::new(m.model_id.0.clone(), m.name.clone()))
        .collect();
    vec![
        SessionConfigOption::select(
            "provider",
            "Provider",
            provider_selection.to_string(),
            provider_options,
        ),
        SessionConfigOption::select(
            "mode",
            "Mode",
            mode_state.current_mode_id.0.clone(),
            mode_options,
        )
        .category(SessionConfigOptionCategory::Mode),
        SessionConfigOption::select(
            "model",
            "Model",
            model_state.current_model_id.0.clone(),
            model_options,
        )
        .category(SessionConfigOptionCategory::Model),
    ]
}

pub fn to_nonnegative_u64(value: Option<i32>) -> Option<u64> {
    value.and_then(|v| u64::try_from(v).ok())
}

pub fn build_prompt_usage(session: &Session) -> Option<Usage> {
    let total = to_nonnegative_u64(session.total_tokens)?;
    let input = to_nonnegative_u64(session.input_tokens).unwrap_or(0);
    let output = to_nonnegative_u64(session.output_tokens).unwrap_or(0);
    Some(Usage::new(total, input, output))
}

pub fn build_usage_update(session: &Session, context_limit: usize) -> UsageUpdate {
    let used = session.total_tokens.unwrap_or(0).max(0) as u64;
    UsageUpdate::new(used, context_limit as u64)
}

pub fn validate_absolute_cwd(cwd: &Path) -> Result<(), agent_client_protocol::Error> {
    if !cwd.is_absolute() {
        return Err(
            agent_client_protocol::Error::invalid_params().data("cwd must be an absolute path")
        );
    }

    if !cwd.exists() || !cwd.is_dir() {
        return Err(agent_client_protocol::Error::invalid_params().data("invalid directory path"));
    }

    Ok(())
}
