"""
GitHub API adapter.

This package provides integrations for GitHub API interactions.
"""

from src.integrations.github.api import GitHubClient, github_client

__all__ = [
    "GitHubClient",
    "github_client",
]
