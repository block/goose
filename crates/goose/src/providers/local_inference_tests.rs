use super::local_inference::{
    select_chat_template, ChatTemplateFallbackWarning, ChatTemplateSelection,
    DEFAULT_FALLBACK_CHAT_TEMPLATE,
};

#[test]
fn embedded_chat_template_is_used_unchanged() {
    let selection = select_chat_template::<_, ()>(Ok("embedded-template"), Some("gemma"));

    assert_eq!(
        selection,
        ChatTemplateSelection::Embedded("embedded-template")
    );
}

#[test]
fn no_embedded_template_uses_architecture_fallbacks() {
    let cases = [
        (
            "gemma",
            "gemma",
            ChatTemplateFallbackWarning::KnownArchitecture,
        ),
        (
            "gemma2",
            "gemma",
            ChatTemplateFallbackWarning::KnownArchitecture,
        ),
        (
            "gemma3",
            "gemma",
            ChatTemplateFallbackWarning::KnownArchitecture,
        ),
        (
            "llama",
            "llama2",
            ChatTemplateFallbackWarning::KnownArchitecture,
        ),
        (
            "llama2",
            "llama2",
            ChatTemplateFallbackWarning::KnownArchitecture,
        ),
        (
            "llama3",
            "llama3",
            ChatTemplateFallbackWarning::KnownArchitecture,
        ),
        (
            "qwen2",
            "chatml",
            ChatTemplateFallbackWarning::KnownArchitecture,
        ),
        (
            "phi3",
            "phi3",
            ChatTemplateFallbackWarning::KnownArchitecture,
        ),
    ];

    for (architecture, expected_template, expected_warning) in cases {
        let selection = select_chat_template::<(), _>(Err(()), Some(architecture));
        let ChatTemplateSelection::BuiltIn(fallback) = selection else {
            panic!("expected fallback for {architecture}");
        };

        assert_eq!(fallback.template_name, expected_template);
        assert_eq!(fallback.architecture, Some(architecture));
        assert_eq!(fallback.warning, expected_warning);
    }
}

#[test]
fn architecture_fallback_is_case_and_whitespace_insensitive() {
    let selection = select_chat_template::<(), _>(Err(()), Some("  LLaMa3  "));
    let ChatTemplateSelection::BuiltIn(fallback) = selection else {
        panic!("expected fallback");
    };

    assert_eq!(fallback.template_name, "llama3");
    assert_eq!(fallback.architecture, Some("LLaMa3"));
    assert_eq!(
        fallback.warning,
        ChatTemplateFallbackWarning::KnownArchitecture
    );
}

#[test]
fn unknown_architecture_falls_back_to_chatml_with_warning() {
    let selection = select_chat_template::<(), _>(Err(()), Some("deepseek2"));
    let ChatTemplateSelection::BuiltIn(fallback) = selection else {
        panic!("expected fallback");
    };

    assert_eq!(fallback.template_name, DEFAULT_FALLBACK_CHAT_TEMPLATE);
    assert_eq!(fallback.architecture, Some("deepseek2"));
    assert_eq!(
        fallback.warning,
        ChatTemplateFallbackWarning::UnknownArchitecture
    );
}

#[test]
fn gemma4_falls_back_to_gemma_with_specific_warning() {
    let selection = select_chat_template::<(), _>(Err(()), Some("gemma4"));
    let ChatTemplateSelection::BuiltIn(fallback) = selection else {
        panic!("expected fallback");
    };

    assert_eq!(fallback.template_name, "gemma");
    assert_eq!(fallback.architecture, Some("gemma4"));
    assert_eq!(
        fallback.warning,
        ChatTemplateFallbackWarning::Gemma4Unsupported
    );
}

#[test]
fn missing_architecture_falls_back_to_chatml() {
    for architecture in [None, Some(""), Some("   ")] {
        let selection = select_chat_template::<(), _>(Err(()), architecture);
        let ChatTemplateSelection::BuiltIn(fallback) = selection else {
            panic!("expected fallback");
        };

        assert_eq!(fallback.template_name, DEFAULT_FALLBACK_CHAT_TEMPLATE);
        assert_eq!(fallback.architecture, None);
        assert_eq!(
            fallback.warning,
            ChatTemplateFallbackWarning::MissingArchitecture
        );
    }
}
