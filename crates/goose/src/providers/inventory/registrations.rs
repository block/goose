use super::{
    config_secret_value, default_inventory_identity, default_inventory_identity_resolver,
    serialize_string_map, InventoryIdentityInput, InventoryRegistration,
};
use crate::config::Config;
use crate::providers::acp_tooling::{acp_adapter_installed, resolved_acp_command};
use crate::providers::amp_acp::{AMP_ACP_BINARY, AMP_ACP_PROVIDER_NAME};
use crate::providers::base::ProviderDef;
use crate::providers::chatgpt_codex::TokenCache as ChatGptCodexTokenCache;
use crate::providers::claude_acp::{CLAUDE_ACP_BINARY, CLAUDE_ACP_PROVIDER_NAME};
use crate::providers::codex_acp::CODEX_ACP_PROVIDER_NAME;
use crate::providers::copilot_acp::{COPILOT_ACP_BINARY, COPILOT_ACP_PROVIDER_NAME};
use crate::providers::formats::anthropic::ANTHROPIC_PROVIDER_NAME;
use crate::providers::google::{GOOGLE_API_HOST, GOOGLE_PROVIDER_NAME};
use crate::providers::huggingface::HuggingFaceProvider;
use crate::providers::huggingface_auth;
use crate::providers::ollama::{ollama_host_configured, OLLAMA_PROVIDER_NAME};
use crate::providers::openai::{OPEN_AI_DEFAULT_BASE_PATH, OPEN_AI_PROVIDER_NAME};
use crate::providers::pi_acp::{PI_ACP_BINARY, PI_ACP_PROVIDER_NAME};
use crate::providers::xai_oauth::TokenCache as XaiOAuthTokenCache;

pub fn openai_inventory() -> InventoryRegistration {
    InventoryRegistration::new(true, || {
        let config = Config::global();
        let mut identity =
            InventoryIdentityInput::new(OPEN_AI_PROVIDER_NAME, OPEN_AI_PROVIDER_NAME)
                .with_public(
                    "host",
                    config
                        .get_param::<String>("OPENAI_HOST")
                        .unwrap_or_else(|_| "https://api.openai.com".to_string()),
                )
                .with_public(
                    "base_path",
                    config
                        .get_param::<String>("OPENAI_BASE_PATH")
                        .unwrap_or_else(|_| OPEN_AI_DEFAULT_BASE_PATH.to_string()),
                );

        if let Ok(organization) = config.get_param::<String>("OPENAI_ORGANIZATION") {
            identity = identity.with_public("organization", organization);
        }
        if let Ok(project) = config.get_param::<String>("OPENAI_PROJECT") {
            identity = identity.with_public("project", project);
        }
        if let Some(api_key) = config_secret_value(config, "OPENAI_API_KEY") {
            identity = identity.with_secret("api_key", api_key);
        }
        if let Some(custom_headers) = config_secret_value(config, "OPENAI_CUSTOM_HEADERS") {
            identity = identity.with_secret("custom_headers", custom_headers);
        }

        Ok(identity)
    })
    .with_configured(|| {
        let config = Config::global();
        if let Ok(host) = config.get_param::<String>("OPENAI_HOST") {
            if host != "https://api.openai.com" {
                return true;
            }
        }
        config
            .get_secret::<serde_json::Value>("OPENAI_API_KEY")
            .is_ok()
    })
}

pub fn anthropic_inventory() -> InventoryRegistration {
    InventoryRegistration::new(true, || {
        let config = Config::global();
        let mut identity =
            InventoryIdentityInput::new(ANTHROPIC_PROVIDER_NAME, ANTHROPIC_PROVIDER_NAME)
                .with_public(
                    "host",
                    config
                        .get_param::<String>("ANTHROPIC_HOST")
                        .unwrap_or_else(|_| "https://api.anthropic.com".to_string()),
                );

        if let Some(api_key) = config_secret_value(config, "ANTHROPIC_API_KEY") {
            identity = identity.with_secret("api_key", api_key);
        }
        if let Ok(headers) = config
            .get_secret::<std::collections::HashMap<String, String>>("ANTHROPIC_CUSTOM_HEADERS")
        {
            identity = identity.with_secret("headers", serialize_string_map(&headers)?);
        }
        Ok(identity)
    })
}

pub fn google_inventory() -> InventoryRegistration {
    InventoryRegistration::new(true, || {
        let config = Config::global();
        let mut identity = InventoryIdentityInput::new(GOOGLE_PROVIDER_NAME, GOOGLE_PROVIDER_NAME)
            .with_public(
                "host",
                config
                    .get_param::<String>("GOOGLE_HOST")
                    .unwrap_or_else(|_| GOOGLE_API_HOST.to_string()),
            );
        if let Some(api_key) = config_secret_value(config, "GOOGLE_API_KEY") {
            identity = identity.with_secret("api_key", api_key);
        }
        Ok(identity)
    })
}

pub fn ollama_inventory() -> InventoryRegistration {
    InventoryRegistration::new(true, || {
        let config = Config::global();
        Ok(
            InventoryIdentityInput::new(OLLAMA_PROVIDER_NAME, OLLAMA_PROVIDER_NAME).with_public(
                "host",
                config
                    .get_param::<String>("OLLAMA_HOST")
                    .unwrap_or_else(|_| "http://localhost:11434".to_string()),
            ),
        )
    })
    .with_configured(|| ollama_host_configured(Config::global()))
}

pub fn huggingface_inventory() -> InventoryRegistration {
    InventoryRegistration::new(false, || {
        let metadata = HuggingFaceProvider::metadata();
        Ok(default_inventory_identity(
            &metadata.name,
            &metadata.name,
            &metadata.config_keys,
            Config::global(),
        ))
    })
    .with_configured(|| huggingface_auth::has_configured_token().unwrap_or(false))
}

pub fn refresh_only() -> InventoryRegistration {
    InventoryRegistration {
        supports_refresh: true,
        identity: default_inventory_identity_resolver(),
        configured: None,
    }
}

pub fn chatgpt_codex_inventory() -> InventoryRegistration {
    InventoryRegistration {
        supports_refresh: false,
        identity: default_inventory_identity_resolver(),
        configured: None,
    }
    .with_configured(|| ChatGptCodexTokenCache::new().has_token())
}

pub fn xai_oauth_inventory() -> InventoryRegistration {
    InventoryRegistration {
        supports_refresh: false,
        identity: default_inventory_identity_resolver(),
        configured: None,
    }
    .with_configured(|| XaiOAuthTokenCache::new().has_token())
}

pub fn acp_inventory(
    provider_id: &'static str,
    command: &'static str,
    supports_refresh: bool,
) -> InventoryRegistration {
    InventoryRegistration::new(supports_refresh, move || {
        let resolved_command = resolved_acp_command(command)?;
        Ok(InventoryIdentityInput::new(provider_id, provider_id)
            .with_public("command", resolved_command.display().to_string()))
    })
    .with_configured(move || acp_adapter_installed(command))
}

pub fn amp_acp_inventory() -> InventoryRegistration {
    acp_inventory(AMP_ACP_PROVIDER_NAME, AMP_ACP_BINARY, false)
}

pub fn claude_acp_inventory() -> InventoryRegistration {
    acp_inventory(CLAUDE_ACP_PROVIDER_NAME, CLAUDE_ACP_BINARY, true)
}

pub fn codex_acp_inventory() -> InventoryRegistration {
    acp_inventory(CODEX_ACP_PROVIDER_NAME, CODEX_ACP_PROVIDER_NAME, true)
}

pub fn copilot_acp_inventory() -> InventoryRegistration {
    acp_inventory(COPILOT_ACP_PROVIDER_NAME, COPILOT_ACP_BINARY, true)
}

pub fn pi_acp_inventory() -> InventoryRegistration {
    acp_inventory(PI_ACP_PROVIDER_NAME, PI_ACP_BINARY, false)
}
