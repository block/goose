"""Tests for workflow conditions.

Tests for WorkflowDurationCondition class.
"""

import pytest

from src.rules.conditions.workflow import WorkflowDurationCondition


class TestWorkflowDurationCondition:
    """Tests for WorkflowDurationCondition class."""

    @pytest.mark.asyncio
    async def test_validate_no_workflow_data(self) -> None:
        """Test that validate returns True when no workflow data is available."""
        condition = WorkflowDurationCondition()
        result = await condition.validate({"minutes": 5}, {})
        assert result is True

    @pytest.mark.asyncio
    async def test_validate_workflow_within_limit(self) -> None:
        """Test that validate returns True when workflow is within time limit."""
        condition = WorkflowDurationCondition()

        event = {
            "workflow_run": {
                "name": "CI Build",
                "run_started_at": "2026-01-28T10:00:00Z",
                "completed_at": "2026-01-28T10:02:00Z",  # 2 minutes
            }
        }

        result = await condition.validate({"minutes": 5}, event)
        assert result is True

    @pytest.mark.asyncio
    async def test_validate_workflow_exceeds_limit(self) -> None:
        """Test that validate returns False when workflow exceeds time limit."""
        condition = WorkflowDurationCondition()

        event = {
            "workflow_run": {
                "name": "CI Build",
                "run_started_at": "2026-01-28T10:00:00Z",
                "completed_at": "2026-01-28T10:10:00Z",  # 10 minutes
            }
        }

        result = await condition.validate({"minutes": 5}, event)
        assert result is False

    @pytest.mark.asyncio
    async def test_validate_missing_timestamps(self) -> None:
        """Test that validate returns True when timestamps are missing."""
        condition = WorkflowDurationCondition()

        event = {
            "workflow_run": {
                "name": "CI Build",
                "run_started_at": None,
            }
        }

        result = await condition.validate({"minutes": 5}, event)
        assert result is True

    @pytest.mark.asyncio
    async def test_evaluate_returns_violations_when_exceeded(self) -> None:
        """Test that evaluate returns violations when duration is exceeded."""
        condition = WorkflowDurationCondition()

        event = {
            "workflow_run": {
                "name": "Long Build",
                "run_started_at": "2026-01-28T10:00:00Z",
                "completed_at": "2026-01-28T10:15:00Z",  # 15 minutes
            }
        }
        context = {"parameters": {"minutes": 10}, "event": event}

        violations = await condition.evaluate(context)
        assert len(violations) == 1
        assert "exceeded duration threshold" in violations[0].message
        assert "Long Build" in violations[0].message

    @pytest.mark.asyncio
    async def test_evaluate_returns_empty_within_limit(self) -> None:
        """Test that evaluate returns empty list when within limit."""
        condition = WorkflowDurationCondition()

        event = {
            "workflow_run": {
                "name": "Quick Build",
                "run_started_at": "2026-01-28T10:00:00Z",
                "completed_at": "2026-01-28T10:01:00Z",  # 1 minute
            }
        }
        context = {"parameters": {"minutes": 5}, "event": event}

        violations = await condition.evaluate(context)
        assert len(violations) == 0

    @pytest.mark.asyncio
    async def test_evaluate_uses_updated_at_fallback(self) -> None:
        """Test that evaluate uses updated_at when completed_at is missing."""
        condition = WorkflowDurationCondition()

        event = {
            "workflow_run": {
                "name": "Build",
                "run_started_at": "2026-01-28T10:00:00Z",
                "updated_at": "2026-01-28T10:08:00Z",  # 8 minutes
            }
        }
        context = {"parameters": {"minutes": 5}, "event": event}

        violations = await condition.evaluate(context)
        assert len(violations) == 1
