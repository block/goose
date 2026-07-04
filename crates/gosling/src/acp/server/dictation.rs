use super::*;
use crate::dictation::providers::{
    all_providers, get_provider_def, is_configured, transcribe_with_provider, DictationProvider,
};

const OPENAI_TRANSCRIPTION_MODEL_CONFIG_KEY: &str = "OPENAI_TRANSCRIPTION_MODEL";
const GROQ_TRANSCRIPTION_MODEL_CONFIG_KEY: &str = "GROQ_TRANSCRIPTION_MODEL";
const ELEVENLABS_TRANSCRIPTION_MODEL_CONFIG_KEY: &str = "ELEVENLABS_TRANSCRIPTION_MODEL";
const OPENAI_TRANSCRIPTION_MODEL: &str = "whisper-1";
const GROQ_TRANSCRIPTION_MODEL: &str = "whisper-large-v3-turbo";
const ELEVENLABS_TRANSCRIPTION_MODEL: &str = "scribe_v1";

impl GoslingAcpAgent {
    pub(super) async fn on_dictation_transcribe(
        &self,
        req: DictationTranscribeRequest,
    ) -> Result<DictationTranscribeResponse, agent_client_protocol::Error> {
        use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
        let config = crate::config::Config::global();

        let provider: DictationProvider = serde_json::from_value(serde_json::Value::String(
            req.provider.clone(),
        ))
        .map_err(|_| {
            agent_client_protocol::Error::invalid_params()
                .data(format!("Unknown provider: {}", req.provider))
        })?;

        let audio_bytes = BASE64.decode(&req.audio).map_err(|_| {
            agent_client_protocol::Error::invalid_params().data("Invalid base64 audio data")
        })?;

        if audio_bytes.len() > 50 * 1024 * 1024 {
            return Err(
                agent_client_protocol::Error::invalid_params().data("Audio too large (max 50MB)")
            );
        }

        let extension = match req.mime_type.as_str() {
            "audio/webm" | "audio/webm;codecs=opus" => "webm",
            "audio/mp4" => "mp4",
            "audio/mpeg" | "audio/mpga" => "mp3",
            "audio/m4a" => "m4a",
            "audio/wav" | "audio/x-wav" => "wav",
            other => {
                return Err(agent_client_protocol::Error::invalid_params()
                    .data(format!("Unsupported format: {other}")));
            }
        };

        let (model_param, default_model) = dictation_transcribe_params(provider);
        let model =
            dictation_selected_model(config, provider).unwrap_or_else(|| default_model.to_string());
        let text = transcribe_with_provider(
            provider,
            model_param.to_string(),
            model,
            audio_bytes,
            extension,
            &req.mime_type,
        )
        .await
        .internal_err()?;

        Ok(DictationTranscribeResponse { text })
    }

    pub(super) async fn on_dictation_config(
        &self,
        _req: DictationConfigRequest,
    ) -> Result<DictationConfigResponse, agent_client_protocol::Error> {
        let config = crate::config::Config::global();
        let mut providers = std::collections::HashMap::new();

        for def in all_providers() {
            let provider = def.provider;
            let host = if let Some(host_key) = def.host_key {
                config
                    .get(host_key, false)
                    .ok()
                    .and_then(|v| v.as_str().map(|s| s.to_string()))
            } else {
                None
            };

            let provider_key = serde_json::to_value(provider)
                .ok()
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .unwrap_or_else(|| format!("{:?}", provider).to_lowercase());
            providers.insert(
                provider_key,
                DictationProviderStatusEntry {
                    configured: is_configured(provider),
                    host,
                    description: def.description.to_string(),
                    uses_provider_config: def.uses_provider_config,
                    settings_path: def.settings_path.map(|s| s.to_string()),
                    config_key: if !def.uses_provider_config {
                        Some(def.config_key.to_string())
                    } else {
                        None
                    },
                    model_config_key: dictation_model_config_key(provider),
                    default_model: dictation_default_model(provider),
                    selected_model: dictation_selected_model(config, provider),
                    available_models: dictation_available_models(provider),
                },
            );
        }

        Ok(DictationConfigResponse { providers })
    }

    pub(super) async fn on_dictation_secret_save(
        &self,
        req: DictationSecretSaveRequest,
    ) -> Result<EmptyResponse, agent_client_protocol::Error> {
        let provider = parse_dictation_provider(&req.provider)?;
        let key = dictation_secret_config_key(provider)?;
        let config = self.config()?;
        config.set_secret(key, &req.value).internal_err()?;
        Config::global().invalidate_secrets_cache();
        Ok(EmptyResponse {})
    }

    pub(super) async fn on_dictation_secret_delete(
        &self,
        req: DictationSecretDeleteRequest,
    ) -> Result<EmptyResponse, agent_client_protocol::Error> {
        let provider = parse_dictation_provider(&req.provider)?;
        let key = dictation_secret_config_key(provider)?;
        let config = self.config()?;
        config.delete_secret(key).internal_err()?;
        Config::global().invalidate_secrets_cache();
        Ok(EmptyResponse {})
    }

    pub(super) async fn on_dictation_model_select(
        &self,
        req: DictationModelSelectRequest,
    ) -> Result<EmptyResponse, agent_client_protocol::Error> {
        let provider: DictationProvider = serde_json::from_value(serde_json::Value::String(
            req.provider.clone(),
        ))
        .map_err(|_| {
            agent_client_protocol::Error::invalid_params()
                .data(format!("Unknown provider: {}", req.provider))
        })?;

        let key = match provider {
            DictationProvider::OpenAI => OPENAI_TRANSCRIPTION_MODEL_CONFIG_KEY,
            DictationProvider::Groq => GROQ_TRANSCRIPTION_MODEL_CONFIG_KEY,
            DictationProvider::ElevenLabs => ELEVENLABS_TRANSCRIPTION_MODEL_CONFIG_KEY,
        };

        crate::config::Config::global()
            .set_param(key, req.model_id)
            .internal_err()?;

        Ok(EmptyResponse {})
    }
}

fn parse_dictation_provider(
    provider: &str,
) -> Result<DictationProvider, agent_client_protocol::Error> {
    serde_json::from_value(serde_json::Value::String(provider.to_string())).map_err(|_| {
        agent_client_protocol::Error::invalid_params().data(format!("Unknown provider: {provider}"))
    })
}

fn dictation_secret_config_key(
    provider: DictationProvider,
) -> Result<&'static str, agent_client_protocol::Error> {
    let def = get_provider_def(provider);
    if def.uses_provider_config {
        return Err(agent_client_protocol::Error::invalid_params().data(
            "Dictation provider uses the main provider configuration. Configure its credentials in provider settings instead.",
        ));
    }

    Ok(def.config_key)
}

fn dictation_model_config_key(provider: DictationProvider) -> Option<String> {
    match provider {
        DictationProvider::OpenAI => Some(OPENAI_TRANSCRIPTION_MODEL_CONFIG_KEY.to_string()),
        DictationProvider::Groq => Some(GROQ_TRANSCRIPTION_MODEL_CONFIG_KEY.to_string()),
        DictationProvider::ElevenLabs => {
            Some(ELEVENLABS_TRANSCRIPTION_MODEL_CONFIG_KEY.to_string())
        }
    }
}

fn dictation_transcribe_params(provider: DictationProvider) -> (&'static str, &'static str) {
    match provider {
        DictationProvider::OpenAI => ("model", OPENAI_TRANSCRIPTION_MODEL),
        DictationProvider::Groq => ("model", GROQ_TRANSCRIPTION_MODEL),
        DictationProvider::ElevenLabs => ("model_id", ELEVENLABS_TRANSCRIPTION_MODEL),
    }
}

fn dictation_default_model(provider: DictationProvider) -> Option<String> {
    match provider {
        DictationProvider::OpenAI => Some(OPENAI_TRANSCRIPTION_MODEL.to_string()),
        DictationProvider::Groq => Some(GROQ_TRANSCRIPTION_MODEL.to_string()),
        DictationProvider::ElevenLabs => Some(ELEVENLABS_TRANSCRIPTION_MODEL.to_string()),
    }
}

fn dictation_selected_model(config: &Config, provider: DictationProvider) -> Option<String> {
    dictation_model_config_key(provider)
        .and_then(|key| {
            config
                .get(&key, false)
                .ok()
                .and_then(|value| value.as_str().map(str::to_owned))
        })
        .or_else(|| dictation_default_model(provider))
}

fn dictation_available_models(provider: DictationProvider) -> Vec<DictationModelOption> {
    match provider {
        DictationProvider::OpenAI => vec![DictationModelOption {
            id: OPENAI_TRANSCRIPTION_MODEL.to_string(),
            label: "Whisper-1".to_string(),
            description: "OpenAI's hosted Whisper transcription model.".to_string(),
        }],
        DictationProvider::Groq => vec![DictationModelOption {
            id: GROQ_TRANSCRIPTION_MODEL.to_string(),
            label: "Whisper Large V3 Turbo".to_string(),
            description: "Groq's fast hosted Whisper transcription model.".to_string(),
        }],
        DictationProvider::ElevenLabs => vec![DictationModelOption {
            id: ELEVENLABS_TRANSCRIPTION_MODEL.to_string(),
            label: "Scribe v1".to_string(),
            description: "ElevenLabs' hosted speech-to-text model.".to_string(),
        }],
    }
}
