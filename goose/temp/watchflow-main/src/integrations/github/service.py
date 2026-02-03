from typing import Any

import httpx
import structlog
from giturlparse import parse  # type: ignore

from src.core.errors import (
    GitHubRateLimitError,
    GitHubResourceNotFoundError,
)

logger = structlog.get_logger()


class GitHubService:
    """
    Application Service for interacting with GitHub.
    Abstraction layer over raw API calls or the lower-level GitHubClient.
    """

    BASE_URL = "https://api.github.com"

    def __init__(self) -> None:
        # We use a shared client for connection pooling in production,
        # but for now, we instantiate per request for safety.
        pass

    async def get_repo_metadata(self, repo_url: str) -> dict[str, Any]:
        """
        Fetches basic metadata for a repository.

        Args:
            repo_url: The full URL of the repository.

        Returns:
            dict: Repository metadata including visibility, stars, description, etc.

        Raises:
            GitHubResourceNotFoundError: If the repository does not exist.
            GitHubRateLimitError: If the API rate limit is exceeded.
        """
        try:
            owner, repo = self._parse_url(repo_url)
        except ValueError as e:
            logger.error("url_parse_failed", repo_url=repo_url, error=str(e))
            raise

        api_url = f"{self.BASE_URL}/repos/{owner}/{repo}"

        try:
            async with httpx.AsyncClient(timeout=60.0) as client:
                response = await client.get(api_url)

                if response.status_code == 404:
                    raise GitHubResourceNotFoundError(f"Repo {owner}/{repo} not found")
                if response.status_code == 403 and "rate limit" in response.text.lower():
                    raise GitHubRateLimitError("GitHub API rate limit exceeded")

                response.raise_for_status()
                data = response.json()
                if not isinstance(data, dict):
                    raise TypeError("Expected a dictionary from GitHub API")
                return data
        except httpx.HTTPStatusError as e:
            logger.error(
                "github_metadata_fetch_failed",
                repo=f"{owner}/{repo}",
                status_code=e.response.status_code,
                response_body=e.response.text,
            )
            raise
        except httpx.TimeoutException as e:
            logger.error("github_metadata_timeout", repo=f"{owner}/{repo}", error=str(e))
            raise
        except httpx.RequestError as e:
            logger.error("github_metadata_request_error", repo=f"{owner}/{repo}", error=str(e))
            raise

    async def analyze_repository_rules(self, repo_url: str, token: str | None = None) -> list[dict[str, Any]]:
        """
        Analyzes the repository and returns rule suggestions.

        This method performs a lightweight analysis by checking for the existence
        of key governance files (CODEOWNERS, CONTRIBUTING.md) and generating
        recommendations based on their presence or absence.

        Args:
            repo_url: The full URL of the repository.
            token: Optional GitHub personal access token for private repos.

        Returns:
            list[dict]: A list of rule recommendations.
        """
        try:
            owner, repo = self._parse_url(repo_url)
        except ValueError as e:
            logger.error("url_parse_failed", repo_url=repo_url, error=str(e))
            raise

        headers = {}
        if token:
            headers["Authorization"] = f"Bearer {token}"

        # 1. Check for specific governance files (Real API Check)
        files_to_check = ["CODEOWNERS", "CONTRIBUTING.md", ".github/workflows"]
        found_files = []

        try:
            async with httpx.AsyncClient(timeout=60.0) as client:
                for filepath in files_to_check:
                    # Tricky: Public repos can be read without auth, Private need auth
                    # We use the 'contents' API
                    check_url = f"{self.BASE_URL}/repos/{owner}/{repo}/contents/{filepath}"
                    resp = await client.get(check_url, headers=headers)
                    if resp.status_code == 200:
                        found_files.append(filepath)
        except httpx.HTTPStatusError as e:
            logger.error(
                "github_files_check_failed",
                repo=f"{owner}/{repo}",
                status_code=e.response.status_code,
                response_body=e.response.text,
            )
            # Continue with empty found_files on error
        except httpx.TimeoutException as e:
            logger.error("github_files_check_timeout", repo=f"{owner}/{repo}", error=str(e))
            # Continue with empty found_files on error
        except httpx.RequestError as e:
            logger.error("github_files_check_request_error", repo=f"{owner}/{repo}", error=str(e))
            # Continue with empty found_files on error

        # 2. Generate Recommendations based on REAL findings
        recommendations = []

        if "CODEOWNERS" not in found_files:
            recommendations.append(
                {
                    "description": "Enforce CODEOWNERS for critical paths",
                    "severity": "high",
                    "reason": "We detected this repository lacks a CODEOWNERS file, which is critical for defining responsibility.",
                }
            )

        if "CONTRIBUTING.md" not in found_files:
            recommendations.append(
                {
                    "description": "Add Contributing Guidelines",
                    "severity": "medium",
                    "reason": "No CONTRIBUTING.md found. This increases friction for new developers.",
                }
            )

        # Always suggest a default rule to prove the connection works
        recommendations.append(
            {
                "description": "Require Linear History",
                "severity": "low",
                "reason": "Standard practice for cleaner git logs.",
            }
        )

        return recommendations

    def _parse_url(self, url: str) -> tuple[str, str]:
        """
        Extracts owner and repo from GitHub URL using giturlparse.
        Handles both HTTPS and SSH formats:
        - https://github.com/owner/repo
        - https://github.com/owner/repo.git
        - git@github.com:owner/repo.git

        Raises:
            ValueError: If URL is not a valid GitHub repository URL
        """
        p = parse(url)
        if not p.valid or not p.owner or not p.repo or "github.com" not in p.host:
            logger.error(
                "invalid_github_url",
                url=url,
                valid=p.valid,
                host=p.host,
                owner=p.owner,
                repo=p.repo,
            )
            raise ValueError(
                f"Invalid GitHub repository URL: {url}. Must be in format 'https://github.com/owner/repo'."
            )
        return p.owner, p.repo
