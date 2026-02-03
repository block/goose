import httpx
import pytest
import respx

from src.integrations.github.graphql import GitHubGraphQLClient


@pytest.mark.asyncio
async def test_execute_query_success() -> None:
    token = "test_token"
    client = GitHubGraphQLClient(token)
    query = "query { viewer { login } }"
    variables = {}

    mock_response = {"data": {"viewer": {"login": "test_user"}}}

    async with respx.mock:
        respx.post("https://api.github.com/graphql").mock(return_value=httpx.Response(200, json=mock_response))

        result = await client.execute_query(query, variables)
        assert result == mock_response


@pytest.mark.asyncio
async def test_execute_query_unauthorized() -> None:
    token = "invalid_token"
    client = GitHubGraphQLClient(token)
    query = "query { viewer { login } }"
    variables: dict[str, str] = {}

    async with respx.mock:
        respx.post("https://api.github.com/graphql").mock(
            return_value=httpx.Response(401, json={"message": "Bad credentials"})
        )

        with pytest.raises(httpx.HTTPStatusError):
            await client.execute_query(query, variables)
