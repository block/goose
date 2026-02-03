"""
Integration tests for the rules API endpoint.
These tests verify the complete HTTP stack but mock OpenAI calls by default.
Set INTEGRATION_TEST_REAL_API=true to make real OpenAI calls.
"""

import os
from unittest.mock import AsyncMock, MagicMock, patch

import pytest
from fastapi.testclient import TestClient

from src.agents.base import AgentResult
from src.main import app


class TestRulesAPIIntegration:
    """Integration test suite for the rules API with mocked external calls (safe for CI)."""

    @pytest.fixture
    def client(self):
        """Create test client."""
        return TestClient(app)

    def test_evaluate_feasible_rule_integration(self, client):
        """Test successful rule evaluation through the complete stack (mocked OpenAI)."""
        # Mock OpenAI unless real API testing is explicitly enabled
        if os.getenv("INTEGRATION_TEST_REAL_API", "false").lower() != "true":
            with patch("src.api.rules.get_agent") as mock_get_agent:
                # Mock the agent instance
                mock_agent = MagicMock()
                mock_get_agent.return_value = mock_agent

                # Mock the execute method as async
                mock_result = AgentResult(
                    success=True,
                    message="Rule is feasible and can be implemented.",
                    data={
                        "is_feasible": True,
                        "rule_type": "time_restriction",
                        "confidence_score": 0.9,
                        "yaml_content": """- description: "Prevent deployments on weekends"
  enabled: true
  severity: "high"
  event_types: ["deployment"]
  parameters:
    days: ["saturday", "sunday"]""",
                        "analysis_steps": ["Analyzed rule feasibility", "Generated YAML configuration"],
                    },
                )
                mock_agent.execute = AsyncMock(return_value=mock_result)

                response = client.post("/api/v1/rules/evaluate", json={"rule_text": "No deployments on weekends"})
        else:
            # Real API call - requires OPENAI_API_KEY
            if not os.getenv("OPENAI_API_KEY"):
                pytest.skip("Real API testing enabled but OPENAI_API_KEY not set")

            response = client.post("/api/v1/rules/evaluate", json={"rule_text": "No deployments on weekends"})

        assert response.status_code == 200
        data = response.json()
        assert data["data"]["supported"] is True
        assert len(data["data"]["snippet"]) > 0
        assert "weekend" in data["data"]["snippet"].lower() or "saturday" in data["data"]["snippet"].lower()
        assert len(data["message"]) > 0

    def test_evaluate_unfeasible_rule_integration(self, client):
        """Test unfeasible rule evaluation through the complete stack (mocked OpenAI)."""
        # Mock OpenAI unless real API testing is explicitly enabled
        if os.getenv("INTEGRATION_TEST_REAL_API", "false").lower() != "true":
            with patch("src.api.rules.get_agent") as mock_get_agent:
                # Mock the agent instance
                mock_agent = MagicMock()
                mock_get_agent.return_value = mock_agent

                # Mock the execute method as async
                mock_result = AgentResult(
                    success=False,
                    message="Rule is not feasible.",
                    data={
                        "is_feasible": False,
                        "rule_type": "undefined",
                        "confidence_score": 0.1,
                        "yaml_content": "",
                        "analysis_steps": ["Analyzed rule feasibility", "Determined rule is not implementable"],
                    },
                )
                mock_agent.execute = AsyncMock(return_value=mock_result)

                response = client.post(
                    "/api/v1/rules/evaluate", json={"rule_text": "This rule is completely impossible to implement"}
                )
        else:
            # Real API call - requires OPENAI_API_KEY
            if not os.getenv("OPENAI_API_KEY"):
                pytest.skip("Real API testing enabled but OPENAI_API_KEY not set")

            response = client.post(
                "/api/v1/rules/evaluate", json={"rule_text": "This rule is completely impossible to implement"}
            )

        assert response.status_code == 200
        data = response.json()
        # Note: For mocked tests, we control the response, for real API this might vary
        if os.getenv("INTEGRATION_TEST_REAL_API", "false").lower() != "true":
            assert data["data"]["supported"] is False
            assert data["data"]["snippet"] == ""
        assert len(data["message"]) > 0

    def test_evaluate_rule_missing_text_integration(self, client):
        """Test API validation for missing rule text (no external API calls needed)."""
        response = client.post("/api/v1/rules/evaluate", json={})

        assert response.status_code == 422  # Validation error
