from unittest.mock import AsyncMock, MagicMock, patch

import aiohttp
import pytest
import respx
from fastapi import status
from httpx import ASGITransport, AsyncClient, Response

from src.main import app

# Example repo URLs for test cases
github_public_repo = "https://github.com/pallets/flask"
github_private_repo = "https://github.com/example/private-repo"


def mock_analysis_report_response():
    return {
        "id": "chatcmpl-report",
        "object": "chat.completion",
        "created": 1234567890,
        "model": "gpt-4",
        "choices": [
            {
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": None,
                    "tool_calls": [
                        {
                            "id": "call_report",
                            "type": "function",
                            "function": {
                                "name": "AnalysisReport",
                                "arguments": '{"report": "## Analysis Report\\n\\nFindings..."}',
                            },
                        }
                    ],
                },
                "finish_reason": "tool_calls",
            }
        ],
        "usage": {"prompt_tokens": 100, "completion_tokens": 50, "total_tokens": 150},
    }


def mock_recommendations_response():
    return {
        "id": "chatcmpl-recs",
        "object": "chat.completion",
        "created": 1234567890,
        "model": "gpt-4",
        "choices": [
            {
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": None,
                    "tool_calls": [
                        {
                            "id": "call_recs",
                            "type": "function",
                            "function": {
                                "name": "RecommendationsList",
                                "arguments": '{"recommendations": [{"key": "require_pr_reviews", "name": "Require Pull Request Reviews", "description": "Ensure all PRs are reviewed before merging", "severity": "high", "category": "quality", "event_types": ["pull_request"], "parameters": {}}]}',
                            },
                        }
                    ],
                },
                "finish_reason": "tool_calls",
            }
        ],
        "usage": {"prompt_tokens": 100, "completion_tokens": 50, "total_tokens": 150},
    }


def mock_rule_reasoning_response():
    return {
        "id": "chatcmpl-reasoning",
        "object": "chat.completion",
        "created": 1234567890,
        "model": "gpt-4",
        "choices": [
            {
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": None,
                    "tool_calls": [
                        {
                            "id": "call_reasoning",
                            "type": "function",
                            "function": {
                                "name": "RuleReasoning",
                                "arguments": '{"reasoning": "This rule is recommended because..."}',
                            },
                        }
                    ],
                },
                "finish_reason": "tool_calls",
            }
        ],
        "usage": {"prompt_tokens": 100, "completion_tokens": 50, "total_tokens": 150},
    }


@pytest.mark.asyncio
@respx.mock
async def test_anonymous_access_public_repo():
    # Mock OpenAI API call (httpx) - Sequence: Report -> Recommendations -> Reasoning
    respx.post("https://api.openai.com/v1/chat/completions").side_effect = [
        Response(200, json=mock_analysis_report_response()),
        Response(200, json=mock_recommendations_response()),
        Response(200, json=mock_rule_reasoning_response()),
    ]

    # Patch global github_client for metadata
    with patch("src.agents.repository_analysis_agent.nodes.github_client") as mock_github:
        # Configure metadata mocks
        mock_github.list_directory_any_auth = AsyncMock(
            side_effect=[
                # Root directory
                [
                    {"name": "README.md", "type": "file"},
                    {"name": "pyproject.toml", "type": "file"},
                    {"name": ".github", "type": "dir"},
                ],
                # .github/workflows directory
                [],
            ]
        )

        mock_github.get_file_content = AsyncMock(
            side_effect=[
                # README.md
                "Test content",
                # CODEOWNERS check (root) - return None for not found
                None,
                # .github/CODEOWNERS check - return None for not found
                None,
                # docs/CODEOWNERS check - return None for not found
                None,
            ]
        )

        # Configure PR signals mock - return ([], None) which is (pr_nodes, warning)
        mock_github.fetch_pr_hygiene_stats = AsyncMock(return_value=([], None))

        async with AsyncClient(transport=ASGITransport(app=app), base_url="http://test") as ac:
            payload = {"repo_url": github_public_repo, "force_refresh": False}
            response = await ac.post("/api/v1/rules/recommend", json=payload)

            assert response.status_code == status.HTTP_200_OK
            data = response.json()
            assert "rules_yaml" in data and "pr_plan" in data and "analysis_summary" in data
            assert isinstance(data["rules_yaml"], str)


@pytest.mark.asyncio
@respx.mock
async def test_anonymous_access_private_repo():
    # Mock OpenAI API call - Sequence: Report -> Recommendations -> Reasoning
    respx.post("https://api.openai.com/v1/chat/completions").side_effect = [
        Response(200, json=mock_analysis_report_response()),
        Response(200, json=mock_recommendations_response()),
        Response(200, json=mock_rule_reasoning_response()),
    ]

    with patch("src.agents.repository_analysis_agent.nodes.github_client") as mock_github:
        # Create a proper ClientResponseError for list_directory_any_auth
        req_info = MagicMock()
        error = aiohttp.ClientResponseError(
            request_info=req_info, history=(), status=404, message="Not Found", headers=None
        )

        mock_github.list_directory_any_auth = AsyncMock(side_effect=error)

        # CRITICAL: get_file_content must be AsyncMock to avoid "await MagicMock" error
        # during the CODEOWNERS check loop which happens even if file_tree failed
        mock_github.get_file_content = AsyncMock(return_value=None)

        # Configure PR signals mock
        mock_github.fetch_pr_hygiene_stats = AsyncMock(return_value=([], None))

        async with AsyncClient(transport=ASGITransport(app=app), base_url="http://test") as ac:
            payload = {"repo_url": github_private_repo, "force_refresh": False}
            response = await ac.post("/api/v1/rules/recommend", json=payload)

            # When GitHub returns 404, the agent returns success with fallback recommendation
            assert response.status_code == status.HTTP_200_OK
            data = response.json()
            assert "rules_yaml" in data and "pr_plan" in data and "analysis_summary" in data


@pytest.mark.asyncio
@respx.mock
async def test_authenticated_access_private_repo():
    # Mock OpenAI API call - Sequence: Report -> Recommendations -> Reasoning
    respx.post("https://api.openai.com/v1/chat/completions").side_effect = [
        Response(200, json=mock_analysis_report_response()),
        Response(200, json=mock_recommendations_response()),
        Response(200, json=mock_rule_reasoning_response()),
    ]

    with patch("src.agents.repository_analysis_agent.nodes.github_client") as mock_github:
        # Mock fetch_repository_metadata calls
        mock_github.list_directory_any_auth = AsyncMock(
            side_effect=[
                # Root directory
                [{"name": "README.md", "type": "file"}, {"name": "package.json", "type": "file"}],
                # .github/workflows directory
                [],
            ]
        )

        mock_github.get_file_content = AsyncMock(
            side_effect=[
                # README.md
                "Private repo",
                # CODEOWNERS checks
                None,
                None,
                None,
            ]
        )

        # Configure PR signals mock
        mock_github.fetch_pr_hygiene_stats = AsyncMock(return_value=([], None))

        async with AsyncClient(transport=ASGITransport(app=app), base_url="http://test") as ac:
            payload = {"repo_url": github_private_repo, "force_refresh": False}
            headers = {"Authorization": "Bearer testtoken"}
            response = await ac.post("/api/v1/rules/recommend", json=payload, headers=headers)

            assert response.status_code == status.HTTP_200_OK
            data = response.json()
            assert "rules_yaml" in data and "pr_plan" in data and "analysis_summary" in data
            assert isinstance(data["rules_yaml"], str)
