use super::{quality, transport::Hub, Quality};
use goose_sdk_types::custom_requests::LocalInferenceSuitability as Suitability;
use serde_json::Value;

pub const CONVERSATIONAL_TASKS: &[&str] = &[
    "text-generation",
    "image-text-to-text",
    "video-text-to-text",
    "audio-text-to-text",
    "text2text-generation",
];

/// Classify purpose independently of architecture, memory, and benchmark rank.
pub fn classify(
    info: &Value,
    _quality: Option<&Quality>,
    chat_template: bool,
) -> (Suitability, String) {
    let task = info["pipeline_tag"]
        .as_str()
        .or_else(|| info["cardData"]["pipeline_tag"].as_str());
    if let Some(task) = task {
        if !CONVERSATIONAL_TASKS.contains(&task) {
            return (
                Suitability::Unsuitable,
                format!("The declared task is {task}, rather than conversational text generation."),
            );
        }
    }
    let tags: Vec<_> = info["tags"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    if tags.iter().any(|tag| {
        matches!(
            *tag,
            "reward-model"
                | "reward-modeling"
                | "reranker"
                | "reranking"
                | "sentence-similarity"
                | "text-embeddings-inference"
                | "embedding"
                | "embeddings"
                | "base"
                | "pretrained"
        )
    }) || info["config"]["architectures"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .any(|name| {
            name.ends_with("ForSequenceClassification") || name.ends_with("ForTokenClassification")
        })
    {
        return (
            Suitability::Unsuitable,
            "Metadata identifies a base model or a non-conversational specialist.".into(),
        );
    }
    let instruction_tag = tags.iter().any(|tag| {
        matches!(
            *tag,
            "conversational"
                | "chat"
                | "instruct"
                | "instruction-tuned"
                | "instruction-following"
                | "tool-use"
                | "function-calling"
        )
    });
    if instruction_tag {
        return (
            Suitability::Suitable,
            "Conversational use is indicated by instruction metadata.".into(),
        );
    }
    if chat_template {
        return (Suitability::Suitable, "Text generation with a declared chat template; conversational quality and tool readiness are not guaranteed.".into());
    }
    (
        Suitability::Unknown,
        "Conversational suitability has not been established from model metadata.".into(),
    )
}

pub fn has_chat_template(info: &Value) -> bool {
    fn nonempty(value: &Value) -> bool {
        value.as_str().is_some_and(|s| !s.trim().is_empty())
            || value.as_array().is_some_and(|templates| {
                templates
                    .iter()
                    .any(|template| nonempty(&template["template"]))
            })
    }
    nonempty(&info["chat_template"])
        || nonempty(&info["config"]["tokenizer_config"]["chat_template"])
}

pub(super) async fn assess(
    hub: &Hub,
    info: &Value,
    quality: Option<&Quality>,
) -> (Suitability, String) {
    assess_checked(hub, info, quality)
        .await
        .unwrap_or_else(|_| classify(info, quality, has_chat_template(info)))
}

pub(super) async fn assess_checked(
    hub: &Hub,
    info: &Value,
    quality: Option<&Quality>,
) -> anyhow::Result<(Suitability, String)> {
    let direct = classify(info, quality, has_chat_template(info));
    if direct.0 == Suitability::Unsuitable {
        return Ok(direct);
    }
    if let Some(parent) = quality::quantized_parent(info) {
        {
            let parent_info = hub.model(parent).await?;
            let inherited = classify(&parent_info, quality, has_chat_template(&parent_info));
            if inherited.0 == Suitability::Unsuitable
                || (direct.0 == Suitability::Unknown && inherited.0 == Suitability::Suitable)
            {
                return Ok((
                    inherited.0,
                    format!("{} Original quantized checkpoint: {parent}.", inherited.1),
                ));
            }
        }
    }
    Ok(direct)
}

#[cfg(all(feature = "eredu", target_os = "macos", target_arch = "aarch64"))]
pub(super) async fn fetch_chat_template_checked(
    hub: &Hub,
    repo: &str,
    revision: &str,
    info: &Value,
) -> anyhow::Result<bool> {
    let names: Vec<_> = info["siblings"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|file| file["rfilename"].as_str())
        .collect();
    if names.contains(&"chat_template.jinja") {
        return Ok(!hub
            .file(repo, revision, "chat_template.jinja")
            .await?
            .is_empty());
    }
    if !names.contains(&"tokenizer_config.json") {
        return Ok(false);
    }
    let config = hub
        .json(
            &format!("{repo}/resolve/{revision}/tokenizer_config.json"),
            &[],
        )
        .await?;
    Ok(has_chat_template(&config))
}

#[cfg(all(feature = "eredu", target_os = "macos", target_arch = "aarch64"))]
pub(super) async fn fetch_chat_template(
    hub: &Hub,
    repo: &str,
    revision: &str,
    info: &Value,
) -> bool {
    fetch_chat_template_checked(hub, repo, revision, info)
        .await
        .unwrap_or(false)
}
