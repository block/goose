# Integrations Module

This module contains integrations for external services and APIs. Integrations provide a clean interface between Watchflow and third-party services.

## Structure

```
src/integrations/
├── providers/          # Provider integrations (OpenAI, Bedrock, Vertex AI)
│   ├── base.py         # Base provider interface
│   ├── openai_provider.py
│   ├── bedrock_provider.py
│   ├── vertex_ai_provider.py
│   └── factory.py      # Provider factory functions
└── github/             # GitHub API adapter
    └── api.py          # GitHubClient implementation
```

## Providers

Provider integrations handle integration with model services. They implement a common interface defined in `base.py`.

### Usage

```python
from src.integrations.providers import get_provider, get_chat_model

# Get a provider instance
provider = get_provider(provider="openai", model="gpt-4")

# Or get a ready-to-use chat model
chat_model = get_chat_model(provider="openai", agent="engine_agent")
```

### Supported Providers

- **OpenAI** - Direct OpenAI API integration
- **AWS Bedrock** - AWS Bedrock with support for inference profiles
- **Vertex AI** - Google Cloud Vertex AI (Model Garden) supporting both Gemini and Claude models

## GitHub Adapter

The GitHub adapter provides a client for interacting with the GitHub API, handling authentication, token caching, and API operations.

### Usage

```python
from src.integrations.github import github_client

# Use the global instance
token = await github_client.get_installation_access_token(installation_id)
```

## Migration Notes

### Usage

All code should use the new import paths:

```python
# ✅ Use these imports
from src.integrations.providers import get_chat_model
from src.integrations.github import github_client
```

## Design Principles

1. **Separation of Concerns** - Integrations handle external service integration, not business logic
2. **Consistent Interface** - All providers implement the same base interface
3. **Flexible Configuration** - Providers support per-agent configuration
4. **Backward Compatible** - Old import paths continue to work during migration
