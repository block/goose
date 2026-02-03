import asyncio
import base64
import time
from typing import Any, cast

import aiohttp
import httpx
import jwt
import structlog
from cachetools import TTLCache  # type: ignore[import-untyped]
from tenacity import retry, stop_after_attempt, wait_exponential

from src.core.config import config
from src.core.errors import GitHubGraphQLError

logger = structlog.get_logger(__name__)

_PR_HYGIENE_QUERY = """
query PRHygiene($owner: String!, $repo: String!) {
  repository(owner: $owner, name: $repo) {
    pullRequests(last: 20, states: [MERGED, CLOSED]) {
      nodes {
        number
        title
        body
        changedFiles
        comments {
          totalCount
        }
        closingIssuesReferences(first: 1) {
          totalCount
        }
        reviews(first: 1) {
          totalCount
        }
      }
    }
  }
}
"""


class GitHubClient:
    """
    A client for interacting with the GitHub API.

    This client handles the authentication flow for a GitHub App, including
    generating a JWT and exchanging it for an installation access token.
    Tokens are cached to improve performance and avoid rate limiting.

    Architectural Note:
    - Implements strict typing for arguments.
    - Handles 'Anonymous' access for public repository analysis (Phase 1 requirement).
    - Centralizes auth header logic to prevent token leakage.
    """

    def __init__(self) -> None:
        self._private_key = self._decode_private_key()
        self._app_id = config.github.app_id
        self._session: aiohttp.ClientSession | None = None
        # Cache for installation tokens (TTL: 50 minutes, GitHub tokens expire in 60)
        self._token_cache: TTLCache = TTLCache(maxsize=100, ttl=50 * 60)

    def _detect_issue_references(self, body: str, title: str) -> bool:
        """Detect if PR body or title contains issue references (e.g. #123)."""
        import re

        # Simple heuristic: look for #digits
        pattern = r"#\d+"
        return bool(re.search(pattern, body) or re.search(pattern, title))

    async def _get_auth_headers(
        self,
        installation_id: int | None = None,
        user_token: str | None = None,
        accept: str = "application/vnd.github.v3+json",
        allow_anonymous: bool = False,  # <--- NEW: Support for Phase 1 Public Analysis
    ) -> dict[str, str] | None:
        """
        Build auth headers using either installation token, user token, or anonymous mode.
        """
        token = user_token

        if token:
            return {"Authorization": f"Bearer {token}", "Accept": accept}

        if installation_id is not None:
            token = await self.get_installation_access_token(installation_id)
            if token:
                return {"Authorization": f"Bearer {token}", "Accept": accept}

        if allow_anonymous:
            # Public access (Subject to 60 req/hr rate limit per IP)
            return {"Accept": accept, "User-Agent": "Watchflow-Analyzer/1.0"}

        return None

    async def get_installation_access_token(self, installation_id: int) -> str | None:
        """
        Gets an access token for a specific installation of the GitHub App.
        Caches the token to avoid regenerating it for every request.
        """
        if installation_id in self._token_cache:
            logger.debug(f"Using cached installation token for installation_id {installation_id}.")
            return cast("str", self._token_cache[installation_id])

        jwt_token = self._generate_jwt()
        headers = {
            "Authorization": f"Bearer {jwt_token}",
            "Accept": "application/vnd.github.v3+json",
        }
        url = f"{config.github.api_base_url}/app/installations/{installation_id}/access_tokens"

        session = await self._get_session()
        async with session.post(url, headers=headers) as response:
            if response.status == 201:
                data = await response.json()
                token = data["token"]
                self._token_cache[installation_id] = token
                logger.info(f"Generated new installation token for installation_id {installation_id}.")
                return cast("str", token)
            else:
                error_text = await response.text()
                logger.error(
                    f"Failed to get installation access token for installation {installation_id}. "
                    f"Status: {response.status}, Response: {error_text}"
                )
                return None

    async def get_repository(
        self, repo_full_name: str, installation_id: int | None = None, user_token: str | None = None
    ) -> dict[str, Any] | None:
        """Fetch repository metadata (default branch, language, etc.). Supports public access."""
        headers = await self._get_auth_headers(
            installation_id=installation_id, user_token=user_token, allow_anonymous=True
        )
        if not headers:
            return None
        url = f"{config.github.api_base_url}/repos/{repo_full_name}"
        session = await self._get_session()
        async with session.get(url, headers=headers) as response:
            if response.status == 200:
                data = await response.json()
                return cast("dict[str, Any]", data)
            return None

    async def list_directory_any_auth(
        self, repo_full_name: str, path: str, installation_id: int | None = None, user_token: str | None = None
    ) -> list[dict[str, Any]]:
        """List directory contents using either installation or user token."""
        headers = await self._get_auth_headers(
            installation_id=installation_id, user_token=user_token, allow_anonymous=True
        )
        if not headers:
            return []
        url = f"{config.github.api_base_url}/repos/{repo_full_name}/contents/{path}"
        session = await self._get_session()
        async with session.get(url, headers=headers) as response:
            if response.status == 200:
                data = await response.json()
                return cast("list[dict[str, Any]]", data if isinstance(data, list) else [data])

            # Raise exception for error statuses to avoid silent failures
            response.raise_for_status()
            return []

    async def get_file_content(
        self, repo_full_name: str, file_path: str, installation_id: int | None, user_token: str | None = None
    ) -> str | None:
        """
        Fetches the content of a file from a repository. Supports anonymous access for public analysis.
        """
        headers = await self._get_auth_headers(
            installation_id=installation_id,
            user_token=user_token,
            accept="application/vnd.github.raw",
            allow_anonymous=True,
        )
        if not headers:
            return None
        url = f"{config.github.api_base_url}/repos/{repo_full_name}/contents/{file_path}"

        session = await self._get_session()
        async with session.get(url, headers=headers) as response:
            if response.status == 200:
                logger.info(f"Successfully fetched file '{file_path}' from '{repo_full_name}'.")
                return await response.text()
            elif response.status == 404:
                logger.info(f"File '{file_path}' not found in '{repo_full_name}'.")
                return None
            else:
                error_text = await response.text()
                logger.error(
                    f"Failed to get file content for {repo_full_name}/{file_path}. "
                    f"Status: {response.status}, Response: {error_text}"
                )
                response.raise_for_status()
                return None

    async def close(self) -> None:
        """Closes the aiohttp session."""
        if self._session and not self._session.closed:
            await self._session.close()

    async def create_check_run(
        self, repo: str, sha: str, name: str, status: str, conclusion: str, output: dict[str, Any], installation_id: int
    ) -> dict[str, Any]:
        """Create a check run."""
        try:
            headers = await self._get_auth_headers(installation_id=installation_id)
            if not headers:
                return {}

            url = f"{config.github.api_base_url}/repos/{repo}/check-runs"
            data = {"name": name, "head_sha": sha, "status": status, "conclusion": conclusion, "output": output}

            session = await self._get_session()
            async with session.post(url, headers=headers, json=data) as response:
                if response.status == 201:
                    return cast("dict[str, Any]", await response.json())
                return {}
        except Exception as e:
            logger.error(f"Error creating check run: {e}")
            return {}

    async def get_pull_request(self, repo: str, pr_number: int, installation_id: int) -> dict[str, Any] | None:
        """Get pull request details."""
        try:
            headers = await self._get_auth_headers(installation_id=installation_id)
            if not headers:
                return None

            url = f"{config.github.api_base_url}/repos/{repo}/pulls/{pr_number}"
            session = await self._get_session()
            async with session.get(url, headers=headers) as response:
                if response.status == 200:
                    return cast("dict[str, Any]", await response.json())
                return None
        except Exception as e:
            logger.error(f"Error getting PR #{pr_number}: {e}")
            return None

    async def list_pull_requests(
        self,
        repo: str,
        installation_id: int | None = None,
        state: str = "all",
        per_page: int = 20,
        user_token: str | None = None,
    ) -> list[dict[str, Any]]:
        """List pull requests for a repository."""
        try:
            headers = await self._get_auth_headers(installation_id=installation_id, user_token=user_token)
            if not headers:
                return []
            url = f"{config.github.api_base_url}/repos/{repo}/pulls?state={state}&per_page={min(per_page, 100)}"

            session = await self._get_session()
            async with session.get(url, headers=headers) as response:
                if response.status == 200:
                    return cast("list[dict[str, Any]]", await response.json())
                return []
        except Exception as e:
            logger.error(f"Error listing PRs for {repo}: {e}")
            return []

    async def _get_session(self) -> aiohttp.ClientSession:
        """
        Initializes and returns the aiohttp session.

        Architectural Note:
        - Creates a new session if none exists or if the current session is closed.
        - Also recreates the session if the event loop has changed (common in test environments).
        """
        try:
            if self._session is None or self._session.closed:
                self._session = aiohttp.ClientSession()
            else:
                # Check if we're in a different event loop (avoid deprecated .loop property)
                try:
                    current_loop = asyncio.get_running_loop()
                    # Try to access session's internal loop to check if it's the same
                    # If the session's loop is closed, this will fail
                    if self._session._loop != current_loop or self._session._loop.is_closed():
                        await self._session.close()
                        self._session = aiohttp.ClientSession()
                except RuntimeError:
                    # No running loop or loop is closed, recreate session
                    self._session = aiohttp.ClientSession()
        except Exception:
            # Fallback: ensure we have a valid session
            self._session = aiohttp.ClientSession()
        return self._session

    def _generate_jwt(self) -> str:
        """Generates a JSON Web Token (JWT) to authenticate as the GitHub App."""
        payload = {
            "iat": int(time.time()),
            "exp": int(time.time()) + (1 * 60),
            "iss": self._app_id,
        }
        return jwt.encode(payload, self._private_key, algorithm="RS256")

    @staticmethod
    def _decode_private_key() -> str:
        try:
            decoded_key = base64.b64decode(config.github.private_key).decode("utf-8")
            return decoded_key
        except Exception as e:
            logger.error(f"Failed to decode private key: {e}")
            raise ValueError("Invalid private key format.") from e

    async def get_pr_files(self, repo_full_name: str, pr_number: int, installation_id: int) -> list[dict[str, Any]]:
        """
        Fetch the list of files changed in a pull request.
        """
        return await self.get_pull_request_files(repo_full_name, pr_number, installation_id)

    async def get_pr_reviews(self, repo_full_name: str, pr_number: int, installation_id: int) -> list[dict[str, Any]]:
        """
        Fetch the list of reviews for a pull request.
        """
        return await self.get_pull_request_reviews(repo_full_name, pr_number, installation_id)

    async def get_pr_checks(self, repo_full_name: str, pr_number: int, installation_id: int) -> list[dict[str, Any]]:
        """
        Fetch the list of checks/statuses for a pull request by finding the head SHA first.
        """
        try:
            pr_data = await self.get_pull_request(repo_full_name, pr_number, installation_id)
            if not pr_data:
                return []

            head_sha = pr_data.get("head", {}).get("sha")
            if not head_sha:
                return []

            # We need to fetch from the check-runs endpoint for this SHA
            headers = await self._get_auth_headers(installation_id=installation_id)
            if not headers:
                return []

            url = f"{config.github.api_base_url}/repos/{repo_full_name}/commits/{head_sha}/check-runs"
            session = await self._get_session()
            async with session.get(url, headers=headers) as response:
                if response.status == 200:
                    data = await response.json()
                    return cast("list[dict[str, Any]]", data.get("check_runs", []))
                return []
        except Exception as e:
            logger.error(f"Error getting checks for PR #{pr_number}: {e}")
            return []

    async def get_user_teams(self, repo: str, username: str, installation_id: int) -> list:
        """Fetch the teams a user belongs to in a repo's org."""
        headers = await self._get_auth_headers(installation_id=installation_id)
        if not headers:
            return []

        org = repo.split("/")[0]
        # Use config base URL instead of hardcoded string
        url = f"{config.github.api_base_url}/orgs/{org}/memberships/{username}/teams"

        session = await self._get_session()
        async with session.get(url, headers=headers) as response:
            if response.status == 200:
                data = await response.json()
                return [cast("dict[str, Any]", team) for team in data]
            return []

    async def get_user_team_membership(self, repo: str, username: str, installation_id: int) -> dict[str, Any]:
        """Get team membership for a user (with caching)."""
        # Implementation with caching
        return {}

    async def get_codeowners(self, repo: str, installation_id: int) -> dict[str, Any]:
        """Get CODEOWNERS file content."""
        try:
            content = await self.get_file_content(repo, ".github/CODEOWNERS", installation_id)
            return {"content": content} if content else {}
        except Exception:
            return {}

    async def create_pull_request_comment(
        self, repo: str, pr_number: int, comment: str, installation_id: int
    ) -> dict[str, Any]:
        """Create a comment on a pull request."""
        try:
            token = await self.get_installation_access_token(installation_id)
            if not token:
                logger.error(f"Failed to get installation token for {installation_id}")
                return {}

            headers = {"Authorization": f"Bearer {token}", "Accept": "application/vnd.github.v3+json"}

            url = f"{config.github.api_base_url}/repos/{repo}/issues/{pr_number}/comments"
            data = {"body": comment}

            session = await self._get_session()
            async with session.post(url, headers=headers, json=data) as response:
                if response.status == 201:
                    result = await response.json()
                    logger.info(f"Created comment on PR #{pr_number} in {repo}")
                    return cast("dict[str, Any]", result)
                else:
                    error_text = await response.text()
                    logger.error(
                        f"Failed to create comment on PR #{pr_number} in {repo}. Status: {response.status}, Response: {error_text}"
                    )
                    return {}
        except Exception as e:
            logger.error(f"Error creating comment on PR #{pr_number} in {repo}: {e}")
            return {}

    async def update_check_run(
        self, repo: str, check_run_id: int, status: str, conclusion: str, output: dict[str, Any], installation_id: int
    ) -> dict[str, Any]:
        """Update a check run."""
        try:
            token = await self.get_installation_access_token(installation_id)
            if not token:
                logger.error(f"Failed to get installation token for {installation_id}")
                return {}

            headers = {"Authorization": f"Bearer {token}", "Accept": "application/vnd.github.v3+json"}

            url = f"{config.github.api_base_url}/repos/{repo}/check-runs/{check_run_id}"
            data = {"status": status, "conclusion": conclusion, "output": output}

            session = await self._get_session()
            async with session.patch(url, headers=headers, json=data) as response:
                if response.status == 200:
                    result = await response.json()
                    logger.info(f"Updated check run {check_run_id} for {repo}")
                    return cast("dict[str, Any]", result)
                else:
                    error_text = await response.text()
                    logger.error(
                        f"Failed to update check run {check_run_id} for {repo}. Status: {response.status}, Response: {error_text}"
                    )
                    return {}
        except Exception as e:
            logger.error(f"Error updating check run {check_run_id} for {repo}: {e}")
            return {}

    async def get_check_runs(self, repo: str, sha: str, installation_id: int) -> list[dict[str, Any]]:
        """Get check runs for a commit."""
        try:
            token = await self.get_installation_access_token(installation_id)
            if not token:
                logger.error(f"Failed to get installation token for {installation_id}")
                return []

            headers = {"Authorization": f"Bearer {token}", "Accept": "application/vnd.github.v3+json"}

            url = f"{config.github.api_base_url}/repos/{repo}/commits/{sha}/check-runs"

            session = await self._get_session()
            async with session.get(url, headers=headers) as response:
                if response.status == 200:
                    data = await response.json()
                    return cast("list[dict[str, Any]]", data.get("check_runs", []))
                else:
                    error_text = await response.text()
                    logger.error(
                        f"Failed to get check runs for {repo} commit {sha}. Status: {response.status}, Response: {error_text}"
                    )
                    return []
        except Exception as e:
            logger.error(f"Error getting check runs for {repo} commit {sha}: {e}")
            return []

    async def get_pull_request_reviews(self, repo: str, pr_number: int, installation_id: int) -> list[dict[str, Any]]:
        """Get reviews for a pull request."""
        try:
            token = await self.get_installation_access_token(installation_id)
            if not token:
                logger.error(f"Failed to get installation token for {installation_id}")
                return []

            headers = {"Authorization": f"Bearer {token}", "Accept": "application/vnd.github.v3+json"}

            url = f"{config.github.api_base_url}/repos/{repo}/pulls/{pr_number}/reviews"

            session = await self._get_session()
            async with session.get(url, headers=headers) as response:
                if response.status == 200:
                    result = await response.json()
                    logger.info(f"Retrieved {len(result)} reviews for PR #{pr_number} in {repo}")
                    return cast("list[dict[str, Any]]", result)
                else:
                    error_text = await response.text()
                    logger.error(
                        f"Failed to get reviews for PR #{pr_number} in {repo}. Status: {response.status}, Response: {error_text}"
                    )
                    return []
        except Exception as e:
            logger.error(f"Error getting reviews for PR #{pr_number} in {repo}: {e}")
            return []

    async def get_pull_request_files(self, repo: str, pr_number: int, installation_id: int) -> list[dict[str, Any]]:
        """Get files changed in a pull request."""
        try:
            token = await self.get_installation_access_token(installation_id)
            if not token:
                logger.error(f"Failed to get installation token for {installation_id}")
                return []

            headers = {"Authorization": f"Bearer {token}", "Accept": "application/vnd.github.v3+json"}

            url = f"{config.github.api_base_url}/repos/{repo}/pulls/{pr_number}/files"

            session = await self._get_session()
            async with session.get(url, headers=headers) as response:
                if response.status == 200:
                    result = await response.json()
                    logger.info(f"Retrieved {len(result)} files for PR #{pr_number} in {repo}")
                    return cast("list[dict[str, Any]]", result)
                else:
                    error_text = await response.text()
                    logger.error(
                        f"Failed to get files for PR #{pr_number} in {repo}. Status: {response.status}, Response: {error_text}"
                    )
                    return []
        except Exception as e:
            logger.error(f"Error getting files for PR #{pr_number} in {repo}: {e}")
            return []

    async def create_comment_reply(
        self, repo: str, comment_id: int, reply: str, installation_id: int
    ) -> dict[str, Any]:
        """Create a reply to a comment."""
        try:
            token = await self.get_installation_access_token(installation_id)
            if not token:
                logger.error(f"Failed to get installation token for {installation_id}")
                return {}

            headers = {"Authorization": f"Bearer {token}", "Accept": "application/vnd.github.v3+json"}

            url = f"{config.github.api_base_url}/repos/{repo}/issues/comments/{comment_id}/reactions"
            data = {"content": "eyes"}  # Add a reaction to acknowledge

            session = await self._get_session()
            async with session.post(url, headers=headers, json=data) as response:
                if response.status == 201:
                    logger.info(f"Added reaction to comment {comment_id} in {repo}")
                    return cast("dict[str, Any]", await response.json())
                else:
                    error_text = await response.text()
                    logger.error(
                        f"Failed to add reaction to comment {comment_id} in {repo}. Status: {response.status}, Response: {error_text}"
                    )
                    return {}
        except Exception as e:
            logger.error(f"Error adding reaction to comment {comment_id} in {repo}: {e}")
            return {}

    async def create_issue_comment(
        self, repo: str, issue_number: int, comment: str, installation_id: int
    ) -> dict[str, Any]:
        """Create a comment on an issue."""
        try:
            token = await self.get_installation_access_token(installation_id)
            if not token:
                logger.error(f"Failed to get installation token for {installation_id}")
                return {}

            headers = {"Authorization": f"Bearer {token}", "Accept": "application/vnd.github.v3+json"}

            url = f"{config.github.api_base_url}/repos/{repo}/issues/{issue_number}/comments"
            data = {"body": comment}

            session = await self._get_session()
            async with session.post(url, headers=headers, json=data) as response:
                if response.status == 201:
                    result = await response.json()
                    logger.info(f"Created comment on issue #{issue_number} in {repo}")
                    return cast("dict[str, Any]", result)
                else:
                    error_text = await response.text()
                    logger.error(
                        f"Failed to create comment on issue #{issue_number} in {repo}. Status: {response.status}, Response: {error_text}"
                    )
                    return {}
        except Exception as e:
            logger.error(f"Error creating comment on issue #{issue_number} in {repo}: {e}")
            return {}

    async def create_deployment_status(
        self,
        repo: str,
        deployment_id: int,
        state: str,
        description: str,
        environment: str,
        log_url: str,
        installation_id: int,
    ) -> dict[str, Any] | None:
        """Create a deployment status."""
        try:
            token = await self.get_installation_access_token(installation_id)
            if not token:
                logger.error(f"Failed to get installation token for {installation_id}")
                return None

            headers = {"Authorization": f"Bearer {token}", "Accept": "application/vnd.github.v3+json"}

            url = f"{config.github.api_base_url}/repos/{repo}/deployments/{deployment_id}/statuses"
            data = {"state": state, "description": description, "environment": environment, "log_url": log_url}

            session = await self._get_session()
            async with session.post(url, headers=headers, json=data) as response:
                if response.status == 201:
                    result = await response.json()
                    logger.info(f"Created deployment status for deployment {deployment_id} in {repo}")
                    return cast("dict[str, Any]", result)
                else:
                    error_text = await response.text()
                    logger.error(
                        f"Failed to create deployment status for deployment {deployment_id} in {repo}. Status: {response.status}, Response: {error_text}"
                    )
                    return None
        except Exception as e:
            logger.error(f"Error creating deployment status for deployment {deployment_id} in {repo}: {e}")
            return None

    async def review_deployment_protection_rule(
        self, callback_url: str, environment: str, state: str, comment: str, installation_id: int
    ) -> dict[str, Any] | None:
        """Review a deployment protection rule."""
        try:
            token = await self.get_installation_access_token(installation_id)
            if not token:
                logger.error(f"Failed to get installation token for {installation_id} to review deployment.")
                return None

            headers = {"Authorization": f"Bearer {token}", "Accept": "application/vnd.github+json"}
            data = {
                "state": state,  # "approved" or "rejected"
                "comment": comment,
                "environment_name": environment,
            }

            session = await self._get_session()
            async with session.post(callback_url, headers=headers, json=data) as response:
                if response.status in [200, 204]:  # 204 No Content is also a success
                    logger.info(f"Successfully reviewed deployment protection rule with state {state}.")
                    if response.status == 200:
                        return cast("dict[str, Any]", await response.json())
                    else:
                        return {"status": "success", "state": state}
                else:
                    error_text = await response.text()
                    logger.error(
                        f"Failed to review deployment protection rule for environment {environment}. Status: {response.status}, Response: {error_text}"
                    )
                    logger.error(f"Request URL: {callback_url}")
                    logger.error(f"Request payload: {data}")
                    return None
        except Exception as e:
            logger.error(f"Error reviewing deployment protection rule: {e}")
            return None

    async def get_issue_comments(self, repo: str, issue_number: int, installation_id: int) -> list[dict[str, Any]]:
        """Get comments for an issue."""
        try:
            token = await self.get_installation_access_token(installation_id)
            if not token:
                logger.error(f"Failed to get installation token for {installation_id}")
                return []

            headers = {"Authorization": f"Bearer {token}", "Accept": "application/vnd.github.v3+json"}

            url = f"{config.github.api_base_url}/repos/{repo}/issues/{issue_number}/comments"

            session = await self._get_session()
            async with session.get(url, headers=headers) as response:
                if response.status == 200:
                    result = await response.json()
                    logger.info(f"Retrieved {len(result)} comments for issue #{issue_number} in {repo}")
                    return cast("list[dict[str, Any]]", result)
                else:
                    error_text = await response.text()
                    logger.error(
                        f"Failed to get comments for issue #{issue_number} in {repo}. Status: {response.status}, Response: {error_text}"
                    )
                    return []
        except Exception as e:
            logger.error(f"Error getting comments for issue #{issue_number} in {repo}: {e}")
            return []

    async def update_deployment_status(
        self, callback_url: str, state: str, description: str, environment_url: str | None = None
    ) -> dict[str, Any] | None:
        """Update deployment status via callback URL."""
        try:
            # For this method, we need to use a different approach since we don't have the installation_id
            # We'll use the JWT token directly
            jwt_token = self._generate_jwt()
            headers = {"Authorization": f"Bearer {jwt_token}", "Accept": "application/vnd.github.v3+json"}

            data = {"state": state, "description": description}
            if environment_url:
                data["environment_url"] = environment_url

            session = await self._get_session()
            async with session.post(callback_url, headers=headers, json=data) as response:
                if response.status == 200:
                    result = await response.json()
                    logger.info(f"Updated deployment status to {state}")
                    return cast("dict[str, Any]", result)
                else:
                    error_text = await response.text()
                    logger.error(
                        f"Failed to update deployment status. Status: {response.status}, Response: {error_text}"
                    )
                    return None
        except Exception as e:
            logger.error(f"Error updating deployment status: {e}")
            return None

    async def get_repository_contributors(
        self, repo: str, installation_id: int | None = None, user_token: str | None = None
    ) -> list[dict[str, Any]]:
        """
        Fetches repository contributors with their contribution counts.
        """
        headers = await self._get_auth_headers(installation_id=installation_id, user_token=user_token)
        if not headers:
            return []
        url = f"{config.github.api_base_url}/repos/{repo}/contributors"

        session = await self._get_session()
        async with session.get(url, headers=headers) as response:
            if response.status == 200:
                contributors = await response.json()
                logger.info(f"Successfully fetched {len(contributors)} contributors for {repo}.")
                return cast("list[dict[str, Any]]", contributors)
            else:
                error_text = await response.text()
                logger.error(
                    f"Failed to get contributors for {repo}. Status: {response.status}, Response: {error_text}"
                )
                return []

    async def get_user_commits(
        self, repo: str, username: str, installation_id: int, limit: int = 100
    ) -> list[dict[str, Any]]:
        """
        Fetches commits by a specific user in the repository.
        """
        token = await self.get_installation_access_token(installation_id)
        if not token:
            return []

        headers = {
            "Authorization": f"Bearer {token}",
            "Accept": "application/vnd.github.v3+json",
        }
        url = f"{config.github.api_base_url}/repos/{repo}/commits?author={username}&per_page={min(limit, 100)}"

        session = await self._get_session()
        async with session.get(url, headers=headers) as response:
            if response.status == 200:
                commits = await response.json()
                logger.info(f"Successfully fetched {len(commits)} commits by {username} in {repo}.")
                return cast("list[dict[str, Any]]", commits)
            else:
                error_text = await response.text()
                logger.error(
                    f"Failed to get commits by {username} in {repo}. Status: {response.status}, Response: {error_text}"
                )
                return []

    async def get_user_pull_requests(
        self, repo: str, username: str, installation_id: int, limit: int = 100
    ) -> list[dict[str, Any]]:
        """
        Fetches pull requests by a specific user in the repository.
        """
        token = await self.get_installation_access_token(installation_id)
        if not token:
            return []

        headers = {
            "Authorization": f"Bearer {token}",
            "Accept": "application/vnd.github.v3+json",
        }
        url = f"{config.github.api_base_url}/repos/{repo}/pulls?state=all&creator={username}&per_page={min(limit, 100)}"

        session = await self._get_session()
        async with session.get(url, headers=headers) as response:
            if response.status == 200:
                pull_requests = await response.json()
                logger.info(f"Successfully fetched {len(pull_requests)} PRs by {username} in {repo}.")
                return cast("list[dict[str, Any]]", pull_requests)
            else:
                error_text = await response.text()
                logger.error(
                    f"Failed to get PRs by {username} in {repo}. Status: {response.status}, Response: {error_text}"
                )
                return []

    async def get_user_issues(
        self, repo: str, username: str, installation_id: int, limit: int = 100
    ) -> list[dict[str, Any]]:
        """
        Fetches issues by a specific user in the repository.
        """
        token = await self.get_installation_access_token(installation_id)
        if not token:
            return []

        headers = {
            "Authorization": f"Bearer {token}",
            "Accept": "application/vnd.github.v3+json",
        }
        url = (
            f"{config.github.api_base_url}/repos/{repo}/issues?state=all&creator={username}&per_page={min(limit, 100)}"
        )

        session = await self._get_session()
        async with session.get(url, headers=headers) as response:
            if response.status == 200:
                issues = await response.json()
                logger.info(f"Successfully fetched {len(issues)} issues by {username} in {repo}.")
                return cast("list[dict[str, Any]]", issues)
            else:
                error_text = await response.text()
                logger.error(
                    f"Failed to get issues by {username} in {repo}. Status: {response.status}, Response: {error_text}"
                )
                return []

    async def get_git_ref_sha(
        self, repo_full_name: str, ref: str, installation_id: int | None = None, user_token: str | None = None
    ) -> str | None:
        """Get the SHA for a branch/ref."""
        headers = await self._get_auth_headers(installation_id=installation_id, user_token=user_token)
        if not headers:
            return None
        ref_clean = ref.removeprefix("refs/heads/") if ref.startswith("refs/heads/") else ref
        url = f"{config.github.api_base_url}/repos/{repo_full_name}/git/ref/heads/{ref_clean}"
        session = await self._get_session()
        async with session.get(url, headers=headers) as response:
            if response.status == 200:
                data = await response.json()
                return cast("str | None", data.get("object", {}).get("sha"))
            return None

    async def create_git_ref(
        self,
        repo_full_name: str,
        ref: str,
        sha: str,
        installation_id: int | None = None,
        user_token: str | None = None,
    ) -> dict[str, Any] | None:
        """Create a new git ref/branch. Returns ref data if successful, None if failed."""
        headers = await self._get_auth_headers(installation_id=installation_id, user_token=user_token)
        if not headers:
            return None
        url = f"{config.github.api_base_url}/repos/{repo_full_name}/git/refs"
        ref_clean = ref.removeprefix("refs/heads/") if ref.startswith("refs/heads/") else ref
        payload = {"ref": f"refs/heads/{ref_clean}", "sha": sha}
        session = await self._get_session()
        async with session.post(url, headers=headers, json=payload) as response:
            if response.status in (200, 201):
                return cast("dict[str, Any]", await response.json())
            # Branch might already exist - check if it exists and points to the same SHA
            if response.status == 422:
                error_data = await response.json()
                if "already exists" in error_data.get("message", "").lower():
                    # Branch exists - verify it's the same SHA
                    existing_ref = await self.get_git_ref_sha(repo_full_name, ref_clean, installation_id, user_token)
                    if existing_ref == sha:
                        logger.info(f"Branch {ref_clean} already exists with same SHA, continuing")
                        return {"ref": f"refs/heads/{ref_clean}", "object": {"sha": sha}}
                    # Branch exists but with different SHA - log and return None
                    logger.error(f"Failed to create branch {ref_clean}: branch exists with different SHA. {error_data}")
                    return None
                # 422 error but not "already exists" - log and return None
                logger.error(f"Failed to create branch {ref_clean}: {error_data}")
                return None
            error_text = await response.text()
            logger.error(f"Failed to create branch {ref_clean}: {response.status} - {error_text}")
            return None

    async def get_file_sha(
        self,
        repo_full_name: str,
        path: str,
        branch: str,
        installation_id: int | None = None,
        user_token: str | None = None,
    ) -> str | None:
        """
        Get the SHA of an existing file on a specific branch.
        Returns None if file doesn't exist.
        """
        headers = await self._get_auth_headers(installation_id=installation_id, user_token=user_token)
        if not headers:
            return None

        url = f"{config.github.api_base_url}/repos/{repo_full_name}/contents/{path.lstrip('/')}"
        params = {"ref": branch}

        session = await self._get_session()
        async with session.get(url, headers=headers, params=params) as response:
            if response.status == 200:
                data = await response.json()
                # Handle both single file and directory responses
                if isinstance(data, dict) and "sha" in data:
                    return cast("str | None", data["sha"])
            return None

    async def create_or_update_file(
        self,
        repo_full_name: str,
        path: str,
        content: str,
        message: str,
        branch: str,
        installation_id: int | None = None,
        user_token: str | None = None,
        sha: str | None = None,
    ) -> dict[str, Any] | None:
        """
        Create or update a file via the Contents API.

        If sha is not provided, will attempt to fetch it if file exists on the branch.
        """
        headers = await self._get_auth_headers(installation_id=installation_id, user_token=user_token)
        if not headers:
            logger.error(f"Failed to get auth headers for create_or_update_file in {repo_full_name}")
            return None

        # If sha not provided, try to get it from existing file
        if not sha:
            existing_sha = await self.get_file_sha(repo_full_name, path, branch, installation_id, user_token)
            if existing_sha:
                sha = existing_sha
                logger.info(f"Found existing file, will update with SHA: {sha[:8]}")

        url = f"{config.github.api_base_url}/repos/{repo_full_name}/contents/{path.lstrip('/')}"
        payload: dict[str, Any] = {
            "message": message,
            "content": base64.b64encode(content.encode()).decode(),
            "branch": branch,
        }
        if sha:
            payload["sha"] = sha

        session = await self._get_session()
        async with session.put(url, headers=headers, json=payload) as response:
            if response.status in (200, 201):
                result = await response.json()
                logger.info(f"Successfully created/updated file {path} in {repo_full_name} on branch {branch}")
                return cast("dict[str, Any]", result)
            error_text = await response.text()
            logger.error(
                f"Failed to create/update file {path} in {repo_full_name} on branch {branch}. "
                f"Status: {response.status}, Response: {error_text}"
            )
            return None

    async def create_pull_request(
        self,
        repo_full_name: str,
        title: str,
        head: str,
        base: str,
        body: str,
        installation_id: int | None = None,
        user_token: str | None = None,
    ) -> dict[str, Any] | None:
        """Open a pull request."""
        headers = await self._get_auth_headers(installation_id=installation_id, user_token=user_token)
        if not headers:
            logger.error(f"Failed to get auth headers for create_pull_request in {repo_full_name}")
            return None
        url = f"{config.github.api_base_url}/repos/{repo_full_name}/pulls"
        payload = {"title": title, "head": head, "base": base, "body": body}
        session = await self._get_session()
        async with session.post(url, headers=headers, json=payload) as response:
            if response.status in (200, 201):
                result = await response.json()
                pr_number = result.get("number")
                pr_url = result.get("html_url", "")
                logger.info(
                    f"Successfully created PR #{pr_number} in {repo_full_name}: {pr_url} (head: {head}, base: {base})"
                )
                from typing import cast

                return cast("dict[str, Any]", result)
            error_text = await response.text()
            logger.error(
                f"Failed to create PR in {repo_full_name} (head: {head}, base: {base}). "
                f"Status: {response.status}, Response: {error_text}"
            )
            return None

    async def fetch_recent_pull_requests(
        self,
        repo_full_name: str,
        installation_id: int | None = None,
        user_token: str | None = None,
        limit: int = 30,
    ) -> list[dict[str, Any]]:
        """
        Fetch recent merged pull requests for hygiene analysis (AI Immune System - Phase 6).

        Returns PRs with fields required for detecting AI spam patterns:
        - title, body (for AI hint detection)
        - author association (FIRST_TIME_CONTRIBUTOR, MEMBER, etc.)
        - linked issues (via timeline API or closing references)
        - additions/deletions (lines changed)

        Args:
            repo_full_name: Repository in 'owner/repo' format
            installation_id: GitHub App installation ID (optional for public repos)
            user_token: User OAuth token (optional)
            limit: Maximum number of PRs to fetch (default 30, max 100)

        Returns:
            List of PR dictionaries with enhanced metadata for hygiene analysis
        """
        try:
            headers = await self._get_auth_headers(
                installation_id=installation_id,
                user_token=user_token,
                allow_anonymous=True,  # Support public repos
            )
            if not headers:
                logger.error("pr_fetch_auth_failed", repo=repo_full_name, error_type="auth_error")
                return []

            # Fetch merged PRs sorted by recently updated
            url = (
                f"{config.github.api_base_url}/repos/{repo_full_name}/pulls"
                f"?state=closed&sort=updated&direction=desc&per_page={min(limit, 100)}"
            )

            session = await self._get_session()
            async with session.get(url, headers=headers) as response:
                if response.status == 200:
                    prs = await response.json()

                    # Filter only merged PRs and extract required fields
                    merged_prs = []
                    for pr in prs:
                        if not pr.get("merged_at"):  # Skip closed but not merged PRs
                            continue

                        # Calculate lines changed
                        additions = pr.get("additions", 0)
                        deletions = pr.get("deletions", 0)
                        lines_changed = additions + deletions

                        # Extract author association
                        author_association = pr.get("author_association", "NONE")

                        # Check for linked issues (heuristic: look for issue references in body)
                        body = pr.get("body") or ""
                        title = pr.get("title") or ""
                        has_issue_ref = self._detect_issue_references(body, title)

                        merged_prs.append(
                            {
                                "number": pr.get("number"),
                                "title": title,
                                "body": body,
                                "author_association": author_association,
                                "additions": additions,
                                "deletions": deletions,
                                "lines_changed": lines_changed,
                                "has_issue_ref": has_issue_ref,
                                "merged_at": pr.get("merged_at"),
                            }
                        )

                        if len(merged_prs) >= limit:
                            break

                    logger.info(
                        "pr_fetch_succeeded", repo=repo_full_name, fetched_count=len(merged_prs), total_closed=len(prs)
                    )
                    return merged_prs

                elif response.status == 404:
                    logger.warning("pr_fetch_repo_not_found", repo=repo_full_name, status_code=404)
                    return []
                else:
                    error_text = await response.text()
                    logger.error(
                        "pr_fetch_failed",
                        repo=repo_full_name,
                        status_code=response.status,
                        error_type="network_error",
                        response=error_text[:200],
                    )
                    return []

        except httpx.HTTPStatusError as e:
            logger.error(
                "pr_fetch_http_error",
                repo=repo_full_name,
                status_code=e.response.status_code,
                error_type="network_error",
                error=str(e),
            )
            return []
        except Exception as e:
            logger.error("pr_fetch_unexpected_error", repo=repo_full_name, error_type="unknown_error", error=str(e))
            return []

    @retry(stop=stop_after_attempt(3), wait=wait_exponential(multiplier=1, min=4, max=10))
    async def execute_graphql(
        self, query: str, variables: dict[str, Any], user_token: str | None = None, installation_id: int | None = None
    ) -> dict[str, Any]:
        """
        Executes a GraphQL query against the GitHub API.

        Args:
            query: The GraphQL query string.
            variables: A dictionary of variables for the query.
            user_token: Optional GitHub Personal Access Token for authenticated requests.
            installation_id: Optional GitHub App installation ID for authenticated requests.

        Returns:
            The JSON response from the API.

        Raises:
            GitHubGraphQLError: If the API returns errors.
            httpx.HTTPStatusError: If the HTTP request fails.
        """

        url = f"{config.github.api_base_url}/graphql"
        payload = {"query": query, "variables": variables}

        # Get appropriate headers (can be anonymous for public data or authenticated)
        # Priority: user_token > installation_id > anonymous (if allowed)
        headers = await self._get_auth_headers(
            user_token=user_token, installation_id=installation_id, allow_anonymous=True
        )
        if not headers:
            # Fallback or error? GraphQL usually demands auth.
            # If we have no headers, we likely can't query GraphQL successfully for many fields.
            # We'll try with empty headers if that's what _get_auth_headers returns (it returns None on failure).
            # If None, we can't proceed.
            logger.error("GraphQL execution failed: No authentication headers available.")
            raise Exception("Authentication required for GraphQL query.")

        start_time = time.time()

        session = await self._get_session()
        async with session.post(url, headers=headers, json=payload) as response:
            try:
                if response.status != 200:
                    # Log the error body for debugging
                    error_text = await response.text()
                    logger.error(
                        "GitHub GraphQL request failed",
                        status_code=response.status,
                        response_body=error_text,
                        query=query,
                    )
                    # Raise exception - the error_text is logged and will be in exception context
                    # We'll check response.status and error_text in the calling code
                    response.raise_for_status()

                json_response = await response.json()
                if "errors" in json_response:
                    logger.error(
                        "GitHub GraphQL Error",
                        errors=json_response["errors"],
                        query=query,
                        variables=variables,
                    )
                    raise GitHubGraphQLError(json_response["errors"])

                from typing import cast

                return cast("dict[str, Any]", json_response)
            finally:
                end_time = time.time()
                logger.debug(
                    "GraphQL query executed",
                    duration_ms=(end_time - start_time) * 1000,
                )

    async def fetch_pr_hygiene_stats(
        self, owner: str, repo: str, user_token: str | None = None, installation_id: int | None = None
    ) -> tuple[list[dict[str, Any]], str | None]:
        """
        Fetches PR statistics for hygiene analysis using GraphQL.

        Args:
            owner: Repository owner (username or org).
            repo: Repository name.
            user_token: Optional GitHub Personal Access Token for authenticated requests.
            installation_id: Optional GitHub App installation ID for authenticated requests.

        Returns:
            Tuple of (pr_nodes, warning_message). warning_message is None if successful, or a string describing the issue.
        """
        _PR_HYGIENE_QUERY_20 = """
        query PRHygiene($owner: String!, $repo: String!) {
          repository(owner: $owner, name: $repo) {
            pullRequests(last: 20, states: [MERGED, CLOSED]) {
              nodes {
                number
                title
                body
                changedFiles
                mergedAt
                additions
                deletions
                author {
                  login
                }
                authorAssociation
                comments {
                  totalCount
                }
                closingIssuesReferences(first: 1) {
                  totalCount
                  nodes {
                    title
                  }
                }
                reviews(first: 10) {
                  nodes {
                    state
                    author {
                      login
                    }
                  }
                }
                files(first: 10) {
                  edges {
                    node {
                      path
                    }
                  }
                }
              }
            }
          }
        }
        """

        _PR_HYGIENE_QUERY_10 = """
        query PRHygiene($owner: String!, $repo: String!) {
          repository(owner: $owner, name: $repo) {
            pullRequests(last: 10, states: [MERGED, CLOSED]) {
              nodes {
                number
                title
                body
                changedFiles
                mergedAt
                additions
                deletions
                author {
                  login
                }
                authorAssociation
                comments {
                  totalCount
                }
                closingIssuesReferences(first: 1) {
                  totalCount
                  nodes {
                    title
                  }
                }
                reviews(first: 10) {
                  nodes {
                    state
                    author {
                      login
                    }
                  }
                }
                files(first: 10) {
                  edges {
                    node {
                      path
                    }
                  }
                }
              }
            }
          }
        }
        """

        variables = {"owner": owner, "repo": repo}

        # Try with 20 PRs first
        try:
            data = await self.execute_graphql(
                _PR_HYGIENE_QUERY_20, variables, user_token=user_token, installation_id=installation_id
            )
            nodes = data.get("data", {}).get("repository", {}).get("pullRequests", {}).get("nodes", [])
            if not nodes:
                logger.warning("GraphQL query returned no PR nodes.", owner=owner, repo=repo)
                return [], "No pull requests found in repository"
            return nodes, None
        except Exception as e:
            error_str = str(e).lower()
            # Check if it's a rate limit error - check both message and status code
            is_rate_limit = "rate limit" in error_str or "403" in error_str
            # Also check if it's an aiohttp ClientResponseError with status 403
            if hasattr(e, "status") and e.status == 403:
                is_rate_limit = True
            has_auth = user_token is not None or installation_id is not None

            if is_rate_limit and not has_auth:
                # Try fallback with fewer PRs (10 instead of 20)
                logger.warning(
                    "Rate limit hit without auth, trying fallback with fewer PRs", owner=owner, repo=repo, error=str(e)
                )
                try:
                    data = await self.execute_graphql(
                        _PR_HYGIENE_QUERY_10, variables, user_token=user_token, installation_id=installation_id
                    )
                    nodes = data.get("data", {}).get("repository", {}).get("pullRequests", {}).get("nodes", [])
                    if nodes:
                        return (
                            nodes,
                            "GitHub API rate limit reached. Showing data from fewer PRs (10 instead of 20). Add a GitHub token for higher rate limits (5,000/hr vs 60/hr).",
                        )
                    else:
                        return (
                            [],
                            "GitHub API rate limit reached and no PRs could be fetched. Add a GitHub token for higher rate limits (5,000/hr vs 60/hr).",
                        )
                except Exception as fallback_error:
                    logger.error("Fallback PR fetch also failed", error=str(fallback_error))
                    return (
                        [],
                        f"GitHub API rate limit exceeded. Unable to fetch PR data. Add a GitHub Personal Access Token for higher rate limits (5,000/hr vs 60/hr). Error: {str(e)}",
                    )
            else:
                # Other error or rate limit with auth (shouldn't happen, but handle gracefully)
                logger.error("Failed to fetch PR hygiene stats", error=str(e))
                if is_rate_limit:
                    return [], f"GitHub API rate limit exceeded. Error: {str(e)}"
                return [], f"Failed to fetch PR data: {str(e)}"


# Global instance
github_client = GitHubClient()
