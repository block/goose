import {Model} from "./ModelContext"

// TODO: move into backends / fetch dynamically
export const goose_models: Model[] = [
    { id: 1, name: "gpt-4o-mini", provider: "OpenAI"},
    { id: 2, name: "gpt-4o", provider: "OpenAI"},
    { id: 3, name: "gpt-4-turbo", provider: "OpenAI"},
    { id: 4, name: "gpt-3.5-turbo", provider: "OpenAI"},
    { id: 5, name: "o1", provider: "OpenAI"},
    { id: 6, name: "o1-mini", provider: "OpenAI"},
    { id: 7, name: "claude-3-5-sonnet-latest", provider: "Anthropic" },
    { id: 8, name: "claude-3-5-haiku-latest", provider: "Anthropic" },
    { id: 9, name: "claude-3-opus-latest", provider: "Anthropic"},
]

export const openai_models = [
    "gpt-4o-mini",
    "gpt-4o",
    "gpt-4-turbo",
    "gpt-3.5-turbo",
    "o1",
    "o1-mini"
]

export const anthropic_models = [
    "claude-3-5-sonnet-latest",
    "claude-3-5-sonnet-2",
    "claude-3-5-haiku-latest",
    "claude-3-opus-latest"
]

export const default_models = {
    "openai": "gpt-4o",
    "anthropic": "claude-3-5-sonnet-2",
    "databricks": "claude-3-5-sonnet-2",
    "google": "gemini-2.0-flash-exp",
    "groq": "llama-3.3-70b-versatile",
    "openrouter": "anthropic/claude-3.5-sonnet",
    "ollama": "qwen2.5"
};

export function getDefaultModel(key: string): string | undefined {
    return default_models[key] || undefined;
}

export const short_list = [
    "gpt-4o",
    "claude-3-5-sonnet-latest",
]

export const required_keys = {
    "OpenAI": ["OPENAI_API_KEY"],
    "Anthropic": ["ANTHROPIC_API_KEY"],
    "Databricks": ["DATABRICKS_HOST"],
    "Groq": ["GROQ_API_KEY"],
    "Ollama": [],
    "Google": ["GOOGLE_API_KEY"],
    "OpenRouter": ["OPENROUTER_API_KEY"]
};

export const supported_providers = [
    "OpenAI", "Anthropic", "Databricks", "Groq", "Google", "Ollama", "OpenRouter"
]

// TODO: models -- update this with correct links and providers
const model_docs_link = [
    { name: "OpenAI", href: "https://platform.openai.com/docs/models" },
    { name: "Anthropic", href: "https://docs.anthropic.com/en/docs/about-claude/models" },
    { name: "Google", href: "https://cloud.google.com/vertex-ai" },
    { name: "Groq", href: "https://mistral.ai/models" },
    { name: "Databricks", href: "https://aws.amazon.com/bedrock/models" },
    { name: "OpenRouter", href: "https://azure.microsoft.com/en-us/products/cognitive-services/openai-service" },
    { name: "Ollama", href: "https://azure.microsoft.com/en-us/products/cognitive-services/openai-service" },
]

export const provider_aliases = [
    {provider: "OpenAI", alias: "openai"},
    {provider: "Anthropic", alias: "anthropic"},
    {provider: "Ollama", alias: "ollama"},
    {provider: "Groq", alias: "groq"},
    {provider: "Databricks", alias: "databricks"},
    {provider: "OpenRouter", alias: "openrouter"},
    {provider: "Google", alias: "google"}
]

export const recent_models = [
    { name: "gpt-4o", provider: "OpenAI", lastUsed: "2 hours ago" },
    { name: "claude-3-5-sonnet-latest", provider: "Anthropic", lastUsed: "Yesterday" },
    { name: "o1-mini", provider: "OpenAI", lastUsed: "3 days ago" },
]

export const model_env_vars = {
    "OpenAI": "OPENAI_MODEL",
    "Anthropic": "ANTHROPIC_MODEL",
    "Databricks": "DATABRICKS_MODEL",

};