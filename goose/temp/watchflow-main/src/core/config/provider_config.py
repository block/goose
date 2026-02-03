"""
Provider configuration.
"""

from dataclasses import dataclass
from typing import cast


@dataclass
class AgentConfig:
    """Per-agent configuration."""

    max_tokens: int = 4096
    temperature: float = 0.1


@dataclass
class ProviderConfig:
    """Provider configuration."""

    api_key: str
    provider: str = "openai"
    max_tokens: int = 4096
    temperature: float = 0.1
    # Provider-specific model fields
    openai_model: str | None = None
    bedrock_model_id: str | None = None
    vertex_ai_model: str | None = None
    # Optional provider-specific fields
    # AWS Bedrock
    bedrock_region: str | None = None
    aws_access_key_id: str | None = None
    aws_secret_access_key: str | None = None
    aws_profile: str | None = None
    # GCP Model Garden
    gcp_project: str | None = None
    gcp_location: str | None = None
    gcp_service_account_key_base64: str | None = None
    # Per-agent configurations
    engine_agent: AgentConfig | None = None
    feasibility_agent: AgentConfig | None = None
    acknowledgment_agent: AgentConfig | None = None

    def get_model_for_provider(self, provider: str) -> str:
        """Get the appropriate model for the given provider with fallbacks."""
        provider = provider.lower()

        if provider == "openai":
            return self.openai_model or "gpt-4.1-mini"
        elif provider == "bedrock":
            return self.bedrock_model_id or "anthropic.claude-3-sonnet-20240229-v1:0"
        elif provider in ["vertex_ai", "garden", "model_garden", "gcp", "vertex", "vertexai"]:
            # Support both Gemini and Claude models in Vertex AI
            return self.vertex_ai_model or "gemini-pro"
        else:
            return "gpt-4.1-mini"  # Ultimate fallback

    def get_max_tokens_for_agent(self, agent: str | None = None) -> int:
        """Get max tokens for agent with fallback to global config."""
        if agent and hasattr(self, agent):
            agent_config = getattr(self, agent)
            if agent_config and isinstance(agent_config, AgentConfig) and hasattr(agent_config, "max_tokens"):
                return int(cast("int", agent_config.max_tokens))
        return int(self.max_tokens)

    def get_temperature_for_agent(self, agent: str | None = None) -> float:
        """Get temperature for agent with fallback to global config."""
        if agent and hasattr(self, agent):
            agent_config = getattr(self, agent)
            if agent_config and isinstance(agent_config, AgentConfig) and hasattr(agent_config, "temperature"):
                return float(cast("float", agent_config.temperature))
        return float(self.temperature)


# Backward compatibility aliases
AgentAIConfig = AgentConfig
AIConfig = ProviderConfig
