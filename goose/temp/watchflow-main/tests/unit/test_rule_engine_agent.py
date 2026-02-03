"""
Tests for the Rule Engine Agent with hybrid validation strategy.
"""

from unittest.mock import AsyncMock, MagicMock, patch

import pytest

from src.agents.base import AgentResult
from src.agents.engine_agent import RuleEngineAgent
from src.agents.engine_agent.models import (
    EngineState,
    HowToFixResponse,
    LLMEvaluationResponse,
    RuleDescription,
    RuleEvaluationResult,
    RuleViolation,
    StrategySelectionResponse,
    ValidationStrategy,
    ValidatorDescription,
)


class TestRuleEngineAgent:
    """Test engine agent functionality."""

    @patch("src.agents.base.BaseAgent.__init__")
    def test_engine_agent_initialization(self, mock_init):
        """Test engine agent initialization with hybrid strategy."""
        agent = RuleEngineAgent(max_retries=5, timeout=45.0)
        # Manually set the attributes since we mocked __init__
        agent.max_retries = 5
        agent.timeout = 45.0
        assert agent.max_retries == 5
        assert agent.timeout == 45.0

    @patch("src.agents.base.BaseAgent.__init__")
    def test_convert_rules_to_descriptions(self, mock_init):
        """Test conversion of rules to rule descriptions without id/name dependency."""
        agent = RuleEngineAgent()
        # Manually set the attributes since we mocked __init__
        agent.max_retries = 3

        rules = [
            {
                "description": "PRs must have security and review labels",
                "parameters": {"required_labels": ["security", "review"]},
                "event_types": ["pull_request"],
                "severity": "high",
            },
            {
                "description": "PRs need at least 2 approvals",
                "parameters": {"min_approvals": 2},
                "event_types": ["pull_request"],
                "severity": "medium",
            },
        ]

        rule_descriptions = agent._convert_rules_to_descriptions(rules)

        assert len(rule_descriptions) == 2
        assert rule_descriptions[0].description == "PRs must have security and review labels"
        assert rule_descriptions[0].parameters == {"required_labels": ["security", "review"]}
        assert rule_descriptions[0].validation_strategy == ValidationStrategy.HYBRID
        assert rule_descriptions[0].validator_name is None  # Will be selected by LLM

    @patch("src.agents.base.BaseAgent.__init__")
    def test_get_validator_descriptions(self, mock_init):
        """Test getting validator descriptions from validators themselves."""
        agent = RuleEngineAgent()
        # Manually set the attributes since we mocked __init__
        agent.max_retries = 3

        validator_descriptions = agent._get_validator_descriptions()

        assert len(validator_descriptions) > 0

        # Check that we have descriptions for common validators
        validator_names = [v.name for v in validator_descriptions]
        assert "required_labels" in validator_names
        # min_approvals is not currently implemented in registry
        assert "title_pattern" in validator_names

        # Check that descriptions have required fields
        for validator in validator_descriptions:
            assert validator.name
            assert validator.description
            assert isinstance(validator.parameter_patterns, list)
            assert isinstance(validator.event_types, list)
            assert isinstance(validator.examples, list)

    @patch("src.agents.base.BaseAgent.__init__")
    def test_validator_description_content(self, mock_init):
        """Test that validator descriptions have meaningful content."""
        agent = RuleEngineAgent()
        # Manually set the attributes since we mocked __init__
        agent.max_retries = 3

        validator_descriptions = agent._get_validator_descriptions()

        # Find required_labels validator
        required_labels_validator = next((v for v in validator_descriptions if v.name == "required_labels"), None)

        assert required_labels_validator is not None
        assert "required labels" in required_labels_validator.description.lower()
        assert "required_labels" in required_labels_validator.parameter_patterns
        assert "pull_request" in required_labels_validator.event_types
        assert len(required_labels_validator.examples) > 0

    @pytest.mark.asyncio
    @patch("src.agents.base.BaseAgent.__init__")
    async def test_execute_with_timeout(self, mock_init):
        """Test execute method with timeout handling."""
        agent = RuleEngineAgent(timeout=30.0)
        # Manually set the attributes since we mocked __init__
        agent.timeout = 30.0
        agent.graph = AsyncMock()

        # Mock the graph execution
        from src.agents.engine_agent.models import EngineState

        mock_state = EngineState(
            event_type="pull_request",
            event_data={},
            rules=[],
            available_validators=[],
            violations=[],
            validator_usage={"required_labels": 2, "min_approvals": 1},
            llm_usage=0,
            analysis_steps=["Rule analysis completed"],
        )

        agent.graph = AsyncMock()
        agent.graph.ainvoke.return_value = mock_state

        rules = [
            {
                "description": "Test rule description",
                "parameters": {"required_labels": ["security"]},
                "event_types": ["pull_request"],
                "severity": "medium",
            }
        ]

        event_data = {
            "pull_request": {"title": "Test PR", "labels": [{"name": "security"}]},
            "repository": {"full_name": "test/repo"},
        }

        result = await agent.execute(event_type="pull_request", event_data=event_data, rules=rules)

        assert result.success is True
        assert result.data["evaluation_result"].validator_usage == {"required_labels": 2, "min_approvals": 1}
        assert result.data["evaluation_result"].llm_usage == 0

    @pytest.mark.asyncio
    @patch("src.agents.base.BaseAgent.__init__")
    async def test_execute_with_violations(self, mock_init):
        """Test execute method with violations."""
        agent = RuleEngineAgent(timeout=30.0)
        # Manually set the attributes since we mocked __init__
        agent.timeout = 30.0
        agent.graph = AsyncMock()

        # Mock the graph execution with violations
        from src.agents.engine_agent.models import EngineState, ValidationStrategy

        violation_dict = {
            "rule_description": "PRs must have security and review labels",
            "severity": "high",
            "message": "Missing required labels",
            "details": {"validator_used": "required_labels"},
            "how_to_fix": "Add security and review labels",
            "validation_strategy": ValidationStrategy.VALIDATOR,
            "execution_time_ms": 150.0,
        }

        mock_state = EngineState(
            event_type="pull_request",
            event_data={},
            rules=[],
            available_validators=[],
            violations=[violation_dict],
            validator_usage={"required_labels": 1},
            llm_usage=0,
            analysis_steps=["Validator violation found"],
        )

        agent.graph = AsyncMock()
        agent.graph.ainvoke.return_value = mock_state

        rules = [
            {
                "description": "PRs must have security and review labels",
                "parameters": {"required_labels": ["security", "review"]},
                "event_types": ["pull_request"],
                "severity": "high",
            }
        ]

        event_data = {
            "pull_request": {
                "title": "Test PR",
                "labels": [],  # No labels
            },
            "repository": {"full_name": "test/repo"},
        }

        result = await agent.execute(event_type="pull_request", event_data=event_data, rules=rules)

        assert result.success is False
        assert len(result.data["evaluation_result"].violations) == 1
        violation = result.data["evaluation_result"].violations[0]
        assert violation.rule_description == "PRs must have security and review labels"
        assert violation.validation_strategy == ValidationStrategy.VALIDATOR
        assert violation.execution_time_ms == 150.0

    @pytest.mark.asyncio
    @patch("src.agents.base.BaseAgent.__init__")
    async def test_execute_with_timeout_error(self, mock_init):
        """Test execute method with timeout error."""
        agent = RuleEngineAgent(timeout=30.0)
        # Manually set the attributes since we mocked __init__
        agent.timeout = 30.0
        agent.graph = AsyncMock()

        # Mock timeout error
        agent.graph = AsyncMock()
        agent.graph.ainvoke.side_effect = TimeoutError()

        rules = [{"description": "Test Rule", "parameters": {}}]
        event_data = {"pull_request": {"title": "Test"}}

        result = await agent.execute(event_type="pull_request", event_data=event_data, rules=rules)

        assert result.success is False
        assert "timed out" in result.message
        assert result.metadata["error_type"] == "TimeoutError"

    @pytest.mark.asyncio
    @patch("src.agents.base.BaseAgent.__init__")
    async def test_legacy_evaluate_method(self, mock_init):
        """Test legacy evaluate method for backwards compatibility."""
        agent = RuleEngineAgent()
        # Manually set the attributes since we mocked __init__
        agent.max_retries = 3

        with patch.object(agent, "execute") as mock_execute:
            mock_execute.return_value = AgentResult(
                success=True, message="Success", data={"evaluation_result": MagicMock(violations=[])}
            )

            result = await agent.evaluate("pull_request", [], {})

            assert result["status"] == "success"
            assert result["violations"] == []

    @pytest.mark.asyncio
    @patch("src.agents.base.BaseAgent.__init__")
    async def test_legacy_evaluate_with_violations(self, mock_init):
        """Test legacy evaluate method with violations."""
        agent = RuleEngineAgent()
        # Manually set the attributes since we mocked __init__
        agent.max_retries = 3

        mock_violation = MagicMock()
        mock_violation.__dict__ = {"rule_description": "Test Rule", "severity": "high", "message": "Violation found"}

        mock_eval_result = MagicMock()
        mock_eval_result.violations = [mock_violation]

        with patch.object(agent, "execute") as mock_execute:
            mock_execute.return_value = AgentResult(
                success=False, message="Violations found", data={"evaluation_result": mock_eval_result}
            )

            result = await agent.evaluate("pull_request", [], {})

            assert result["status"] == "violations_found"
            assert len(result["violations"]) == 1
            assert result["violations"][0]["rule_description"] == "Test Rule"


class TestEngineAgentModels:
    """Test engine agent data models."""

    def test_validation_strategy_enum(self):
        """Test validation strategy enum values."""
        assert ValidationStrategy.VALIDATOR == "validator"
        assert ValidationStrategy.LLM_REASONING == "llm_reasoning"
        assert ValidationStrategy.HYBRID == "hybrid"

    def test_rule_description_creation(self):
        """Test rule description model creation."""
        rule_desc = RuleDescription(
            description="Test rule description",
            parameters={"required_labels": ["security"]},
            event_types=["pull_request"],
            severity="high",
            validation_strategy=ValidationStrategy.VALIDATOR,
            validator_name="required_labels",
            fallback_to_llm=True,
        )

        assert rule_desc.description == "Test rule description"
        assert rule_desc.validation_strategy == ValidationStrategy.VALIDATOR
        assert rule_desc.validator_name == "required_labels"
        assert rule_desc.fallback_to_llm is True

    def test_validator_description_creation(self):
        """Test validator description model creation."""
        validator_desc = ValidatorDescription(
            name="required_labels",
            description="Validates that pull requests have required labels",
            parameter_patterns=["required_labels"],
            event_types=["pull_request"],
            examples=[{"required_labels": ["security", "review"]}],
        )

        assert validator_desc.name == "required_labels"
        assert "pull requests have required labels" in validator_desc.description.lower()
        assert "required_labels" in validator_desc.parameter_patterns
        assert "pull_request" in validator_desc.event_types
        assert len(validator_desc.examples) == 1

    def test_strategy_selection_response_creation(self):
        """Test strategy selection response model creation."""
        response = StrategySelectionResponse(
            strategy=ValidationStrategy.VALIDATOR,
            validator_name="required_labels",
            reasoning="This rule matches validator patterns",
        )

        assert response.strategy == ValidationStrategy.VALIDATOR
        assert response.validator_name == "required_labels"
        assert "matches validator patterns" in response.reasoning

    def test_llm_evaluation_response_creation(self):
        """Test LLM evaluation response model creation."""
        response = LLMEvaluationResponse(
            is_violated=True,
            message="Missing required labels",
            details={"reasoning": "No security label found"},
            how_to_fix="Add security and review labels",
        )

        assert response.is_violated is True
        assert "Missing required labels" in response.message
        assert response.how_to_fix == "Add security and review labels"

    def test_how_to_fix_response_creation(self):
        """Test how to fix response model creation."""
        response = HowToFixResponse(
            how_to_fix="Add the 'security' and 'review' labels to this pull request",
            steps=["Go to the PR page", "Click on 'Labels'", "Add 'security' and 'review'"],
            examples=["gh pr edit --add-label security,review"],
            context="These labels are required for security review process",
        )

        assert "security" in response.how_to_fix
        assert len(response.steps) == 3
        assert len(response.examples) == 1
        assert "security review" in response.context

    def test_engine_state_creation(self):
        """Test engine state model creation."""
        state = EngineState(
            event_type="pull_request",
            event_data={"test": "data"},
            rules=[{"description": "Test rule"}],
            rule_descriptions=[],
            available_validators=[],
            violations=[],
            evaluation_context={},
            analysis_steps=[],
            validator_usage={},
            llm_usage=0,
        )

        assert state.event_type == "pull_request"
        assert state.validator_usage == {}
        assert state.llm_usage == 0
        assert len(state.available_validators) == 0

    def test_rule_violation_creation(self):
        """Test rule violation model creation."""
        violation = RuleViolation(
            rule_description="Test rule description",
            severity="high",
            message="Violation found",
            details={"validator_used": "required_labels"},
            how_to_fix="Add required labels",
            docs_url="https://docs.example.com",
            validation_strategy=ValidationStrategy.VALIDATOR,
            execution_time_ms=150.0,
        )

        assert violation.rule_description == "Test rule description"
        assert violation.validation_strategy == ValidationStrategy.VALIDATOR
        assert violation.execution_time_ms == 150.0

    def test_rule_evaluation_result_creation(self):
        """Test rule evaluation result model creation."""
        result = RuleEvaluationResult(
            event_type="pull_request",
            repo_full_name="test/repo",
            violations=[],
            total_rules_evaluated=5,
            rules_triggered=0,
            total_rules=5,
            evaluation_time_ms=1000.0,
            validator_usage={"required_labels": 2},
            llm_usage=1,
        )

        assert result.event_type == "pull_request"
        assert result.validator_usage == {"required_labels": 2}
        assert result.llm_usage == 1
        assert result.evaluation_time_ms == 1000.0


class TestEngineAgentPerformance:
    """Test engine agent performance characteristics."""

    @pytest.mark.asyncio
    @patch("src.agents.base.BaseAgent.__init__")
    async def test_concurrent_validator_execution(self, mock_init):
        """Test concurrent validator execution performance."""
        agent = RuleEngineAgent()
        # Manually set the attributes since we mocked __init__
        agent.max_retries = 3
        agent.graph = AsyncMock()

        # Create multiple rules that can use validators
        rules = [
            {
                "description": f"Test rule {i}",
                "parameters": {"required_labels": ["security"]},
                "event_types": ["pull_request"],
                "severity": "medium",
            }
            for i in range(5)
        ]

        event_data = {
            "pull_request": {"title": "Test PR", "labels": [{"name": "security"}]},
            "repository": {"full_name": "test/repo"},
        }

        # Mock the graph execution
        from src.agents.engine_agent.models import EngineState

        mock_state = EngineState(
            event_type="pull_request",
            event_data={},
            rules=[],
            available_validators=[],
            violations=[],
            validator_usage={"required_labels": 5},
            llm_usage=0,
            analysis_steps=["All validators executed concurrently"],
        )

        agent.graph = AsyncMock()
        agent.graph.ainvoke.return_value = mock_state

        result = await agent.execute(event_type="pull_request", event_data=event_data, rules=rules)

        assert result.success is True
        assert result.data["evaluation_result"].validator_usage["required_labels"] == 5
        assert result.data["evaluation_result"].llm_usage == 0

    @pytest.mark.asyncio
    @patch("src.agents.base.BaseAgent.__init__")
    async def test_hybrid_strategy_performance(self, mock_init):
        """Test hybrid strategy performance with mixed validators and LLM."""
        agent = RuleEngineAgent()
        # Manually set the attributes since we mocked __init__
        agent.max_retries = 3
        agent.graph = AsyncMock()

        # Create mixed rules (some validators, some LLM)
        rules = [
            {
                "description": "Uses fast validator",
                "parameters": {"required_labels": ["security"]},
                "event_types": ["pull_request"],
                "severity": "medium",
            },
            {
                "description": "Complex rule requiring LLM reasoning",
                "parameters": {"custom_field": "value"},
                "event_types": ["pull_request"],
                "severity": "high",
            },
        ]

        event_data = {"pull_request": {"title": "Test PR"}, "repository": {"full_name": "test/repo"}}

        # Mock the graph execution
        from src.agents.engine_agent.models import EngineState

        mock_state = EngineState(
            event_type="pull_request",
            event_data={},
            rules=[],
            available_validators=[],
            violations=[],
            validator_usage={"required_labels": 1},
            llm_usage=1,
            analysis_steps=["Hybrid execution completed"],
        )

        agent.graph = AsyncMock()
        agent.graph.ainvoke.return_value = mock_state

        result = await agent.execute(event_type="pull_request", event_data=event_data, rules=rules)

        assert result.success is True
        assert result.data["evaluation_result"].validator_usage["required_labels"] == 1
        assert result.data["evaluation_result"].llm_usage == 1


class TestStructuredResponses:
    """Test structured response models for LLM interactions."""

    def test_strategy_selection_response_validation(self):
        """Test strategy selection response validation."""
        # Valid response
        response = StrategySelectionResponse(
            strategy=ValidationStrategy.VALIDATOR,
            validator_name="required_labels",
            reasoning="Rule matches validator patterns",
        )
        assert response.strategy == ValidationStrategy.VALIDATOR

        # Test with None validator
        response = StrategySelectionResponse(
            strategy=ValidationStrategy.LLM_REASONING, validator_name=None, reasoning="Complex rule requiring LLM"
        )
        assert response.strategy == ValidationStrategy.LLM_REASONING
        assert response.validator_name is None

    def test_llm_evaluation_response_validation(self):
        """Test LLM evaluation response validation."""
        # Violation response
        response = LLMEvaluationResponse(
            is_violated=True,
            message="Missing required labels",
            details={"reasoning": "No security label found"},
            how_to_fix="Add security labels",
        )
        assert response.is_violated is True
        assert response.how_to_fix == "Add security labels"

        # Pass response
        response = LLMEvaluationResponse(
            is_violated=False, message="All requirements met", details={"reasoning": "All labels present"}
        )
        assert response.is_violated is False
        assert response.how_to_fix is None

    def test_how_to_fix_response_validation(self):
        """Test how to fix response validation."""
        # Complete response
        response = HowToFixResponse(
            how_to_fix="Add the 'security' and 'review' labels to this pull request",
            steps=["Go to PR page", "Click Labels", "Add security and review"],
            examples=["gh pr edit --add-label security,review"],
            context="Labels required for security review",
        )
        assert "security" in response.how_to_fix
        assert len(response.steps) == 3
        assert len(response.examples) == 1

        # Minimal response
        response = HowToFixResponse(how_to_fix="Update the PR title to match the pattern")
        assert response.how_to_fix == "Update the PR title to match the pattern"
        assert response.steps == []
        assert response.examples == []


if __name__ == "__main__":
    pytest.main([__file__])
