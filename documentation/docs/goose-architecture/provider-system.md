---
title: Provider System Architecture
sidebar_position: 6
---

# Provider System Architecture

The Provider system in Goose abstracts interactions with different LLM providers (OpenAI, Anthropic, Google, etc.). This document explains the provider system architecture, implementation, and supported providers.

## Core Concepts

### Provider Trait

The core of the provider system is the `Provider` trait:

```rust
#[async_trait]
pub trait Provider: Send + Sync {
    fn metadata() -> ProviderMetadata
    where
        Self: Sized;

    async fn complete(
        &self,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<(Message, ProviderUsage), ProviderError>;

    fn get_model_config(&self) -> ModelConfig;
}
```

This trait defines the core functionality that all providers must implement:

1. Providing metadata about the provider
2. Completing prompts with the LLM
3. Providing model configuration

### Provider Metadata

Provider metadata includes information about the provider's capabilities and configuration requirements:

```rust
pub struct ProviderMetadata {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub default_model: String,
    pub known_models: Vec<String>,
    pub model_doc_link: String,
    pub config_keys: Vec<ConfigKey>,
}
```

### Model Configuration

Model configuration includes settings specific to each model:

```rust
pub struct ModelConfig {
    pub name: String,
    pub context_limit: usize,
    pub tokenizer_name: Option<String>,
    pub toolshim: bool,
}
```

## Provider Factory

Providers are created through a factory pattern:

```rust
pub fn create(provider_name: &str, model_name: &str) -> Result<Box<dyn Provider>>;
```

This allows for dynamic provider selection based on configuration.

## Supported Providers

Goose supports multiple LLM providers:

1. **OpenAI**: Supports GPT-3.5, GPT-4, and other OpenAI models
2. **Anthropic**: Supports Claude models
3. **Google**: Supports Gemini models
4. **Azure OpenAI**: Supports Azure-hosted OpenAI models
5. **Ollama**: Supports locally-hosted models through Ollama
6. **Groq**: Supports Groq-hosted models
7. **Bedrock**: Supports AWS Bedrock models
8. **OpenRouter**: Supports models through OpenRouter

## Provider Implementation

Each provider implements the `Provider` trait with specific logic for interacting with the provider's API:

```rust
pub struct OpenAiProvider {
    client: Client,
    host: String,
    base_path: String,
    api_key: String,
    organization: Option<String>,
    project: Option<String>,
    model: ModelConfig,
}

#[async_trait]
impl Provider for OpenAiProvider {
    // Implementation details
}
```

## Configuration

Providers are configured through environment variables or configuration files:

```rust
pub struct ConfigKey {
    pub name: String,
    pub required: bool,
    pub secret: bool,
    pub default: Option<String>,
}
```

Each provider defines its required configuration keys in its metadata.

## Usage Tracking

Providers track token usage for each interaction:

```rust
pub struct Usage {
    pub input_tokens: Option<i32>,
    pub output_tokens: Option<i32>,
    pub total_tokens: Option<i32>,
}

pub struct ProviderUsage {
    pub model: String,
    pub usage: Usage,
}
```

This information is used for monitoring and optimization.

## Error Handling

Provider errors are handled through the `ProviderError` enum:

```rust
pub enum ProviderError {
    Configuration(String),
    Network(String),
    Authentication(String),
    RateLimit(String),
    ContextLengthExceeded(String),
    Timeout(String),
    Other(String),
}
```

Special handling is provided for context length exceeded errors to enable context revision.

## Tool Support

Providers have varying levels of tool support:

1. **Native Tool Support**: Some providers (like OpenAI) support tools natively
2. **Tool Shim**: For providers without native tool support, Goose uses a tool shim to parse tool calls from text

```rust
pub struct ToolShim {
    // Implementation details
}
```

## Best Practices

1. **Provider Abstraction**: Use the `Provider` trait to abstract provider-specific details
2. **Configuration Management**: Use environment variables or configuration files for provider settings
3. **Error Handling**: Handle provider-specific errors and translate them to common error types
4. **Usage Tracking**: Monitor token usage for optimization
