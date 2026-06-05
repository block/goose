use crate::config::GooseMode;
use crate::providers::inventory::{ProviderInventoryEntry, ProviderInventoryService};
use crate::session::Session;
use agent_client_protocol::schema::{
    AvailableCommand, AvailableCommandInput, AvailableCommandsUpdate, ModelId, ModelInfo,
    SessionConfigOption, SessionConfigOptionCategory, SessionConfigSelectOption, SessionId,
    SessionMode, SessionModeId, SessionModeState, SessionModelState, SessionNotification,
    SessionUpdate, UnstructuredCommandInput,
};
use agent_client_protocol::{Client, ConnectionTo};
use strum::{EnumMessage, VariantNames};

use super::server::{build_usage_updates, DEFAULT_PROVIDER_ID, DEFAULT_PROVIDER_LABEL};

pub(super) fn session_provider_selection(session: &Session) -> &str {
    session
        .provider_name
        .as_deref()
        .unwrap_or(DEFAULT_PROVIDER_ID)
}

pub(super) fn build_model_state(
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

struct ProviderOptionEntry {
    id: String,
    label: String,
}

async fn list_provider_entries(current_provider: Option<&str>) -> Vec<ProviderOptionEntry> {
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

pub(super) async fn build_provider_options(
    current_provider: Option<&str>,
) -> Vec<SessionConfigSelectOption> {
    list_provider_entries(current_provider)
        .await
        .into_iter()
        .map(|provider| SessionConfigSelectOption::new(provider.id, provider.label))
        .collect()
}

pub(super) fn should_refresh_inventory_for_session_init(entry: &ProviderInventoryEntry) -> bool {
    entry.configured
        && entry.supports_refresh
        && (entry.last_updated_at.is_none() || ProviderInventoryService::is_stale(entry))
}

pub(super) fn build_mode_state(
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

pub(super) async fn build_session_setup_config(
    provider_inventory: &ProviderInventoryService,
    session: &Session,
) -> Result<
    (
        SessionModeState,
        Option<SessionModelState>,
        Option<Vec<SessionConfigOption>>,
    ),
    agent_client_protocol::Error,
> {
    let mode_state = build_mode_state(session.goose_mode)?;

    let (Some(provider_name), Some(model_config)) = (
        session.provider_name.as_deref(),
        session.model_config.as_ref(),
    ) else {
        return Ok((mode_state, None, None));
    };
    let Some(inventory) = provider_inventory
        .find_entry_for_provider(provider_name)
        .await
    else {
        return Ok((mode_state, None, None));
    };
    let model_state = build_model_state(model_config.model_name.as_str(), &inventory);
    let provider_selection = session_provider_selection(session);
    let provider_options = build_provider_options(Some(provider_name)).await;
    let config_options = build_config_options(
        &mode_state,
        &model_state,
        provider_selection,
        provider_options,
    );
    Ok((mode_state, Some(model_state), Some(config_options)))
}

pub(super) fn build_config_options(
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

fn available_commands_update(working_dir: &std::path::Path) -> AvailableCommandsUpdate {
    let commands = crate::slash_commands::slash_command::list_acp_commands(Some(working_dir))
        .into_iter()
        .map(|entry| {
            let mut command = AvailableCommand::new(entry.name, entry.description);
            if let Some(input_hint) = entry.input_hint {
                command = command.input(AvailableCommandInput::Unstructured(
                    UnstructuredCommandInput::new(input_hint),
                ));
            }
            command
        })
        .collect();

    AvailableCommandsUpdate::new(commands)
}

pub(super) fn send_session_setup_notifications(
    cx: &ConnectionTo<Client>,
    session: &Session,
    supports_goose_custom_notifications: bool,
) -> Result<(), agent_client_protocol::Error> {
    let session_id = SessionId::new(session.id.clone());
    if let Some(updates) = build_usage_updates(session) {
        if supports_goose_custom_notifications {
            cx.send_notification(updates.custom)?;
        }
        cx.send_notification(SessionNotification::new(
            session_id.clone(),
            SessionUpdate::UsageUpdate(updates.standard),
        ))?;
    }
    cx.send_notification(SessionNotification::new(
        session_id,
        SessionUpdate::AvailableCommandsUpdate(available_commands_update(&session.working_dir)),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    #[test_case(
        vec!["model-a".into(), "model-b".into()]
        => SessionModelState::new(
            ModelId::new("unused"),
            vec![ModelInfo::new(ModelId::new("unused"), "unused"),
                 ModelInfo::new(ModelId::new("model-a"), "model-a"),
                 ModelInfo::new(ModelId::new("model-b"), "model-b")],
        )
        ; "returns current and available models"
    )]
    #[test_case(
        vec![]
        => SessionModelState::new(
            ModelId::new("unused"),
            vec![ModelInfo::new(ModelId::new("unused"), "unused")],
        )
        ; "empty model list"
    )]
    fn test_build_model_state(models: Vec<String>) -> SessionModelState {
        let inventory = ProviderInventoryEntry {
            provider_id: "mock".to_string(),
            provider_name: "Mock".to_string(),
            description: "Mock".to_string(),
            default_model: "unused".to_string(),
            configured: true,
            provider_type: crate::providers::base::ProviderType::Builtin,
            category: crate::providers::catalog::ProviderSetupCategory::Model,
            config_keys: vec![],
            setup_steps: vec![],
            supports_refresh: true,
            refreshing: false,
            models: models
                .into_iter()
                .map(|id| crate::providers::inventory::InventoryModel {
                    name: id.clone(),
                    id,
                    family: None,
                    context_limit: None,
                    reasoning: None,
                    recommended: false,
                })
                .collect(),
            last_updated_at: None,
            last_refresh_attempt_at: None,
            last_refresh_error: None,
            model_selection_hint: None,
        };
        build_model_state("unused", &inventory)
    }

    #[test_case(
        GooseMode::Auto
        => Ok(SessionModeState::new(
            SessionModeId::new("auto"),
            vec![
                SessionMode::new(SessionModeId::new("auto"), "auto")
                    .description("Automatically approve tool calls"),
                SessionMode::new(SessionModeId::new("approve"), "approve")
                    .description("Ask before every tool call"),
                SessionMode::new(SessionModeId::new("smart_approve"), "smart_approve")
                    .description("Ask only for sensitive tool calls"),
                SessionMode::new(SessionModeId::new("chat"), "chat")
                    .description("Chat only, no tool calls"),
            ],
        ))
        ; "auto mode"
    )]
    #[test_case(
        GooseMode::Approve
        => Ok(SessionModeState::new(
            SessionModeId::new("approve"),
            vec![
                SessionMode::new(SessionModeId::new("auto"), "auto")
                    .description("Automatically approve tool calls"),
                SessionMode::new(SessionModeId::new("approve"), "approve")
                    .description("Ask before every tool call"),
                SessionMode::new(SessionModeId::new("smart_approve"), "smart_approve")
                    .description("Ask only for sensitive tool calls"),
                SessionMode::new(SessionModeId::new("chat"), "chat")
                    .description("Chat only, no tool calls"),
            ],
        ))
        ; "approve mode"
    )]
    fn test_build_mode_state(
        current_mode: GooseMode,
    ) -> Result<SessionModeState, agent_client_protocol::Error> {
        build_mode_state(current_mode)
    }

    #[test_case(
        build_mode_state(GooseMode::Auto).unwrap(),
        "openai",
        vec![
            SessionConfigSelectOption::new("anthropic", "anthropic"),
            SessionConfigSelectOption::new("openai", "openai"),
        ],
        SessionModelState::new(
            ModelId::new("gpt-4"),
            vec![ModelInfo::new(ModelId::new("gpt-4"), "gpt-4"), ModelInfo::new(ModelId::new("gpt-3.5"), "gpt-3.5")],
        )
        => vec![
            SessionConfigOption::select(
                "provider", "Provider", "openai",
                vec![
                    SessionConfigSelectOption::new("anthropic", "anthropic"),
                    SessionConfigSelectOption::new("openai", "openai"),
                ],
            ),
            SessionConfigOption::select(
                "mode", "Mode", "auto",
                vec![
                    SessionConfigSelectOption::new("auto", "auto").description("Automatically approve tool calls"),
                    SessionConfigSelectOption::new("approve", "approve").description("Ask before every tool call"),
                    SessionConfigSelectOption::new("smart_approve", "smart_approve").description("Ask only for sensitive tool calls"),
                    SessionConfigSelectOption::new("chat", "chat").description("Chat only, no tool calls"),
                ],
            ).category(SessionConfigOptionCategory::Mode),
            SessionConfigOption::select(
                "model", "Model", "gpt-4",
                vec![
                    SessionConfigSelectOption::new("gpt-4", "gpt-4"),
                    SessionConfigSelectOption::new("gpt-3.5", "gpt-3.5"),
                ],
            ).category(SessionConfigOptionCategory::Model),
        ]
        ; "auto mode with multiple models"
    )]
    #[test_case(
        build_mode_state(GooseMode::Approve).unwrap(),
        "openai",
        vec![SessionConfigSelectOption::new("openai", "openai")],
        SessionModelState::new(ModelId::new("only-model"), vec![ModelInfo::new(ModelId::new("only-model"), "only-model")])
        => vec![
            SessionConfigOption::select(
                "provider", "Provider", "openai",
                vec![SessionConfigSelectOption::new("openai", "openai")],
            ),
            SessionConfigOption::select(
                "mode", "Mode", "approve",
                vec![
                    SessionConfigSelectOption::new("auto", "auto").description("Automatically approve tool calls"),
                    SessionConfigSelectOption::new("approve", "approve").description("Ask before every tool call"),
                    SessionConfigSelectOption::new("smart_approve", "smart_approve").description("Ask only for sensitive tool calls"),
                    SessionConfigSelectOption::new("chat", "chat").description("Chat only, no tool calls"),
                ],
            ).category(SessionConfigOptionCategory::Mode),
            SessionConfigOption::select(
                "model", "Model", "only-model",
                vec![SessionConfigSelectOption::new("only-model", "only-model")],
            ).category(SessionConfigOptionCategory::Model),
        ]
        ; "approve mode with single model"
    )]
    fn test_build_config_options(
        mode_state: SessionModeState,
        provider_name: &'static str,
        provider_options: Vec<SessionConfigSelectOption>,
        model_state: SessionModelState,
    ) -> Vec<SessionConfigOption> {
        build_config_options(&mode_state, &model_state, provider_name, provider_options)
    }
}
