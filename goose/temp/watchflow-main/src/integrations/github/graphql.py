from typing import Any

import httpx
import structlog

logger = structlog.get_logger()


class GitHubGraphQLClient:
    def __init__(self, token: str):
        self.token = token
        self.endpoint = "https://api.github.com/graphql"

    async def execute_query(self, query: str, variables: dict[str, Any]) -> dict[str, Any]:
        """
        Executes a GraphQL query against the GitHub API.

        Args:
            query: The GraphQL query string.
            variables: A dictionary of variables for the query.

        Returns:
            The JSON response data as a dictionary.

        Raises:
            httpx.HTTPStatusError: If the request fails with a non-200 status code.
        """
        headers = {
            "Authorization": f"Bearer {self.token}",
            "Content-Type": "application/json",
        }

        try:
            async with httpx.AsyncClient(timeout=30.0) as client:
                response = await client.post(
                    self.endpoint, json={"query": query, "variables": variables}, headers=headers
                )
                response.raise_for_status()

                data = response.json()
                if not isinstance(data, dict):
                    raise TypeError("Expected a dictionary from GraphQL API")
                return data

        except httpx.HTTPStatusError as e:
            logger.error("graphql_request_failed", status=e.response.status_code)
            raise
