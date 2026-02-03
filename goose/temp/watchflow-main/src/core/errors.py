"""
Core error classes for the Watchflow application.
"""

from typing import Any


class GitHubGraphQLError(Exception):
    """Raised when GitHub GraphQL API returns errors in the response."""

    def __init__(self, errors: list[dict[str, Any]]) -> None:
        self.errors = errors
        super().__init__(f"GraphQL errors: {errors}")


class RepositoryNotFoundError(Exception):
    """Raised when a repository is not found or inaccessible."""

    pass


class GitHubRateLimitError(Exception):
    """Raised when GitHub API rate limit is exceeded."""

    pass


class GitHubResourceNotFoundError(Exception):
    """Raised when a specific GitHub resource is not found."""

    pass
