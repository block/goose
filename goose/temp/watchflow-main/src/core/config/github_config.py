"""
GitHub configuration.
"""

from dataclasses import dataclass


@dataclass
class GitHubConfig:
    """GitHub configuration."""

    app_name: str
    app_id: str
    app_client_secret: str
    private_key: str
    webhook_secret: str
    api_base_url: str = "https://api.github.com"
