"""
Main configuration class that composes all configs.
"""

import json
import os

from dotenv import load_dotenv

from src.core.config.cors_config import CORSConfig
from src.core.config.github_config import GitHubConfig
from src.core.config.langsmith_config import LangSmithConfig
from src.core.config.logging_config import LoggingConfig
from src.core.config.provider_config import AgentConfig, ProviderConfig
from src.core.config.repo_config import RepoConfig

# Load environment variables from a .env file
load_dotenv()


class Config:
    """Main configuration class."""

    def __init__(self) -> None:
        self.github = GitHubConfig(
            app_name=os.getenv("APP_NAME_GITHUB", ""),
            app_id=os.getenv("APP_CLIENT_ID_GITHUB", ""),
            app_client_secret=os.getenv("APP_CLIENT_SECRET_GITHUB", ""),
            private_key=os.getenv("PRIVATE_KEY_BASE64_GITHUB", ""),
            webhook_secret=os.getenv("WEBHOOK_SECRET_GITHUB", ""),
        )

        self.ai = ProviderConfig(
            provider=os.getenv("AI_PROVIDER", "openai"),
            api_key=os.getenv("OPENAI_API_KEY", ""),
            max_tokens=int(os.getenv("AI_MAX_TOKENS", "4096")),
            temperature=float(os.getenv("AI_TEMPERATURE", "0.1")),
            # Provider-specific model fields
            openai_model=os.getenv("OPENAI_MODEL"),
            bedrock_model_id=os.getenv("BEDROCK_MODEL_ID"),
            vertex_ai_model=os.getenv("VERTEX_AI_MODEL") or os.getenv("MODEL_GARDEN_MODEL"),  # Support legacy name
            # AWS Bedrock configuration
            bedrock_region=os.getenv("BEDROCK_REGION"),
            aws_access_key_id=os.getenv("AWS_ACCESS_KEY_ID"),
            aws_secret_access_key=os.getenv("AWS_SECRET_ACCESS_KEY"),
            aws_profile=os.getenv("AWS_PROFILE"),
            # GCP Model Garden configuration
            gcp_project=os.getenv("GCP_PROJECT_ID"),
            gcp_location=os.getenv("GCP_LOCATION"),
            gcp_service_account_key_base64=os.getenv("GCP_SERVICE_ACCOUNT_KEY_BASE64"),
            # Per-agent configurations
            engine_agent=AgentConfig(
                max_tokens=int(os.getenv("AI_ENGINE_MAX_TOKENS", "8000")),
                temperature=float(os.getenv("AI_ENGINE_TEMPERATURE", "0.1")),
            ),
            feasibility_agent=AgentConfig(
                max_tokens=int(os.getenv("AI_FEASIBILITY_MAX_TOKENS", "4096")),
                temperature=float(os.getenv("AI_FEASIBILITY_TEMPERATURE", "0.1")),
            ),
            acknowledgment_agent=AgentConfig(
                max_tokens=int(os.getenv("AI_ACKNOWLEDGMENT_MAX_TOKENS", "2000")),
                temperature=float(os.getenv("AI_ACKNOWLEDGMENT_TEMPERATURE", "0.1")),
            ),
        )

        # LangSmith configuration
        self.langsmith = LangSmithConfig(
            tracing_v2=os.getenv("LANGCHAIN_TRACING_V2", "false").lower() == "true",
            endpoint=os.getenv("LANGCHAIN_ENDPOINT", "https://api.smith.langchain.com"),
            api_key=os.getenv("LANGCHAIN_API_KEY", ""),
            project=os.getenv("LANGCHAIN_PROJECT", "watchflow-dev"),
        )

        # CORS configuration
        cors_headers = os.getenv("CORS_HEADERS", '["*"]')
        cors_origins = os.getenv(
            "CORS_ORIGINS",
            '["http://localhost:3000", "http://127.0.0.1:3000", "http://localhost:5500", "https://warestack.github.io", "https://watchflow.dev"]',
        )

        try:
            self.cors = CORSConfig(
                headers=json.loads(cors_headers),
                origins=json.loads(cors_origins),
            )
        except json.JSONDecodeError:
            # Fallback to default values if JSON parsing fails
            self.cors = CORSConfig(
                headers=["*"],
                origins=[
                    "http://localhost:3000",
                    "http://127.0.0.1:3000",
                    "http://localhost:5500",
                    "https://warestack.github.io",
                    "https://watchflow.dev",
                ],
            )

        self.repo_config = RepoConfig(
            base_path=os.getenv("REPO_CONFIG_BASE_PATH", ".watchflow"),
            rules_file=os.getenv("REPO_CONFIG_RULES_FILE", "rules.yaml"),
        )

        self.logging = LoggingConfig(
            level=os.getenv("LOG_LEVEL", "INFO"),
            format=os.getenv("LOG_FORMAT", "%(asctime)s - %(name)s - %(levelname)s - %(message)s"),
            file_path=os.getenv("LOG_FILE_PATH"),
        )

        # Development settings
        self.debug = os.getenv("DEBUG", "false").lower() == "true"
        self.environment = os.getenv("ENVIRONMENT", "development")

        # Repository Analysis Feature Settings (AI Immune System)
        # CRITICAL: USE_MOCK_DATA must be False for CEO demo (Phase 5)
        self.use_mock_data = os.getenv("USE_MOCK_DATA", "false").lower() == "true"
        self.anonymous_rate_limit = int(os.getenv("ANONYMOUS_RATE_LIMIT", "5"))  # Per hour
        self.authenticated_rate_limit = int(os.getenv("AUTHENTICATED_RATE_LIMIT", "100"))  # Per hour
        self.analysis_timeout = int(os.getenv("ANALYSIS_TIMEOUT", "60"))  # Seconds

    def validate(self) -> bool:
        """Validate configuration."""
        errors = []

        if not self.github.app_name:
            errors.append("APP_NAME_GITHUB is required")

        if not self.github.app_id:
            errors.append("CLIENT_ID_GITHUB is required")

        if not self.github.app_client_secret:
            errors.append("APP_CLIENT_SECRET is required")

        if not self.github.private_key:
            errors.append("PRIVATE_KEY_BASE64_GITHUB is required")

        if not self.github.webhook_secret:
            errors.append("WEBHOOK_SECRET_GITHUB is required")

        if self.ai.provider == "openai" and not self.ai.api_key:
            errors.append("OPENAI_API_KEY is required for OpenAI provider")

        # Bedrock credentials are read from AWS environment/IMDS; encourage region/model hints
        if self.ai.provider == "bedrock" and not self.ai.bedrock_model_id:
            errors.append("BEDROCK_MODEL_ID is required for Bedrock provider")

        # Vertex AI typically uses ADC; project/location optional but recommended
        vertex_aliases = {
            "vertex_ai",
            "garden",
            "vertex",
            "vertexai",
            "model_garden",
            "gcp",
        }
        if self.ai.provider in vertex_aliases and not self.ai.vertex_ai_model:
            errors.append("VERTEX_AI_MODEL is required for Google Vertex AI provider")

        if errors:
            raise ValueError(f"Configuration errors: {', '.join(errors)}")

        return True


# Global config instance
config = Config()
