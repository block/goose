from unittest.mock import AsyncMock, MagicMock

import pytest

from src.core.models import Violation
from src.event_processors.pull_request.enricher import PullRequestEnricher
from src.event_processors.pull_request.processor import PullRequestProcessor
from src.integrations.github.check_runs import CheckRunManager
from src.tasks.task_queue import Task


@pytest.fixture
def mock_agent():
    agent = AsyncMock()
    return agent


@pytest.fixture
def processor(monkeypatch, mock_agent):
    monkeypatch.setattr("src.event_processors.pull_request.processor.get_agent", lambda x: mock_agent)

    proc = PullRequestProcessor()

    # Create a mock for the GitHub client that returns a token
    mock_github_client = AsyncMock()
    mock_github_client.get_installation_access_token.return_value = "fake_token"

    # Patch the instance's github_client
    proc.github_client = mock_github_client

    proc.enricher = MagicMock(spec=PullRequestEnricher)
    proc.check_run_manager = AsyncMock(spec=CheckRunManager)
    return proc


@pytest.mark.asyncio
async def test_process_success(processor, mock_agent):
    task = MagicMock(spec=Task)
    task.repo_full_name = "owner/repo"
    task.installation_id = 1
    task.payload = {"pull_request": {"number": 1, "head": {"sha": "sha123"}}}

    processor.enricher.enrich_event_data.return_value = {"enriched": "data"}
    processor.enricher.fetch_acknowledgments.return_value = {}
    processor.rule_provider.get_rules = AsyncMock(return_value=[])

    mock_agent.execute.return_value = MagicMock(data={"evaluation_result": MagicMock(violations=[])})

    result = await processor.process(task)

    assert result.success is True
    assert result.violations == []
    processor.enricher.enrich_event_data.assert_called_once()
    mock_agent.execute.assert_called_once()
    # Ensure check run manager called for success
    processor.check_run_manager.create_check_run.assert_awaited_once()


@pytest.mark.asyncio
async def test_process_with_violations(processor, mock_agent):
    task = MagicMock(spec=Task)
    task.repo_full_name = "owner/repo"
    task.installation_id = 1
    task.payload = {"pull_request": {"number": 1, "head": {"sha": "sha123"}}}

    processor.enricher.enrich_event_data.return_value = {"enriched": "data"}
    processor.enricher.fetch_acknowledgments.return_value = {}
    processor.rule_provider.get_rules = AsyncMock(return_value=[])

    violation = Violation(rule_description="Rule 1", severity="high", message="Violation message")
    mock_agent.execute.return_value = MagicMock(data={"evaluation_result": MagicMock(violations=[violation])})

    result = await processor.process(task)

    assert result.success is False
    assert len(result.violations) == 1
    assert result.violations[0].rule_description == "Rule 1"
    # Ensure check run manager called for violation
    processor.check_run_manager.create_check_run.assert_awaited_once()
