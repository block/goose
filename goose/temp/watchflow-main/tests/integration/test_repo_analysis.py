from unittest.mock import AsyncMock, patch

import pytest
import respx
from httpx import Response

from src.agents.repository_analysis_agent.agent import RepositoryAnalysisAgent
from src.agents.repository_analysis_agent.models import AnalysisState, HygieneMetrics

# Mock data for a repository
MOCK_REPO_URL = "https://github.com/mock/repo"
MOCK_REPO_FULL_NAME = "mock/repo"


@respx.mock
@pytest.mark.asyncio
async def test_agent_returns_enhanced_metrics():
    """
    Verifies that the RepositoryAnalysisAgent correctly populates and returns
    the new HygieneMetrics with their default values in the final report.
    """
    # Mock the LLM call to prevent actual network requests - proper OpenAI structure
    respx.post("https://api.openai.com/v1/chat/completions").mock(
        return_value=Response(
            200,
            json={
                "choices": [
                    {
                        "message": {
                            "role": "assistant",
                            "content": None,
                            "tool_calls": [
                                {
                                    "id": "call_123",
                                    "type": "function",
                                    "function": {
                                        "name": "RecommendationsList",
                                        "arguments": '{"recommendations": []}',
                                    },
                                }
                            ],
                        },
                        "finish_reason": "tool_calls",
                        "index": 0,
                    }
                ],
                "model": "gpt-4",
                "usage": {"prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15},
            },
        )
    )

    # Patch the github_client used in nodes to prevent actual network calls via aiohttp
    with patch("src.agents.repository_analysis_agent.nodes.github_client") as mock_github:
        # Configure metadata mocks
        mock_github.list_directory_any_auth = AsyncMock(return_value=[])
        mock_github.get_file_content = AsyncMock(return_value=None)

        # Configure PR signals mock
        mock_github.fetch_pr_hygiene_stats = AsyncMock(return_value=([], None))

        # 2. Action: Initialize the agent and invoke the graph directly to get the final state
        agent = RepositoryAnalysisAgent()
        graph = agent._build_graph()
        initial_state = AnalysisState(repo_full_name=MOCK_REPO_FULL_NAME, is_public=True)
        final_graph_state = await graph.ainvoke(initial_state)

        # 3. Assertion: Verify the HygieneMetrics in the final state
        assert final_graph_state is not None
        assert final_graph_state.get("hygiene_summary") is not None
        assert isinstance(final_graph_state["hygiene_summary"], HygieneMetrics)

        # Verify default values for enhanced hygiene metrics (Phase 2)
        metrics = final_graph_state["hygiene_summary"]
        assert metrics.issue_diff_mismatch_rate == 0.0
        assert metrics.ghost_contributor_rate == 0.0
        assert metrics.new_code_test_coverage == 0.0
        assert metrics.codeowner_bypass_rate == 0.0
        assert metrics.ai_generated_rate == 0.0

        # Verify no error occurred
        assert final_graph_state.get("error") is None
