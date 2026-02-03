from unittest.mock import AsyncMock, MagicMock, patch

import pytest

from src.core.models import Severity
from src.event_processors.push import PushProcessor
from src.integrations.github.check_runs import CheckRunManager
from src.tasks.task_queue import Task


@pytest.fixture
def mock_agent():
    return AsyncMock()


@pytest.fixture
def mock_github_client():
    return AsyncMock()


@pytest.fixture
def mock_rule_provider():
    provider = AsyncMock()
    provider.get_rules.return_value = []
    return provider


@pytest.fixture
def processor(mock_agent, mock_github_client, mock_rule_provider):
    with (
        patch("src.event_processors.push.get_agent", return_value=mock_agent),
        patch("src.event_processors.base.github_client", mock_github_client),
    ):
        proc = PushProcessor()
        proc.rule_provider = mock_rule_provider
        proc.engine_agent = mock_agent
        proc.check_run_manager = AsyncMock(spec=CheckRunManager)
        return proc


@pytest.fixture
def task():
    task = MagicMock(spec=Task)
    task.repo_full_name = "owner/repo"
    task.installation_id = 123
    task.payload = {
        "ref": "refs/heads/main",
        "commits": [{"id": "sha1"}],
        "after": "sha123",
        "pusher": {"name": "user"},
    }
    return task


@pytest.mark.asyncio
async def test_process_no_rules(processor, task, mock_rule_provider):
    mock_rule_provider.get_rules.return_value = []

    result = await processor.process(task)

    assert result.success is True
    assert result.violations == []
    processor.engine_agent.execute.assert_not_called()


@pytest.mark.asyncio
async def test_process_success_no_violations(processor, task, mock_rule_provider):
    # Setup rules
    rule = MagicMock()
    rule.description = "Test Rule"
    rule.event_types = ["push"]
    mock_rule_provider.get_rules.return_value = [rule]

    # Setup agent response (raw dicts)
    processor.engine_agent.execute.return_value = MagicMock(data={"violations": []})

    result = await processor.process(task)

    assert result.success is True
    assert result.violations == []

    # Verify check run created with success
    processor.check_run_manager.create_check_run.assert_awaited_once()
    call_args = processor.check_run_manager.create_check_run.call_args[1]
    assert call_args["conclusion"] == "success"
    assert call_args["violations"] == []


@pytest.mark.asyncio
async def test_process_with_violations(processor, task, mock_rule_provider):
    # Setup rules
    rule = MagicMock()
    rule.description = "Test Rule"
    mock_rule_provider.get_rules.return_value = [rule]

    # Setup agent response with raw dict violations
    raw_violation = {"rule": "Test Rule", "severity": "high", "message": "Bad code", "suggestion": "Fix it"}
    processor.engine_agent.execute.return_value = MagicMock(data={"violations": [raw_violation]})

    result = await processor.process(task)

    assert result.success is True
    assert len(result.violations) == 1
    assert result.violations[0].rule_description == "Test Rule"
    assert result.violations[0].severity == Severity.HIGH

    # Verify check run created with violations
    processor.check_run_manager.create_check_run.assert_awaited_once()
    call_args = processor.check_run_manager.create_check_run.call_args[1]
    assert call_args["repo"] == "owner/repo"
    assert call_args["sha"] == "sha123"
    assert len(call_args["violations"]) == 1
