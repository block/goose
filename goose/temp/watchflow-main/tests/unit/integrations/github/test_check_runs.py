from unittest.mock import AsyncMock

import pytest

from src.core.models import Severity, Violation
from src.integrations.github.api import GitHubClient
from src.integrations.github.check_runs import CheckRunManager


@pytest.fixture
def mock_github_client():
    return AsyncMock(spec=GitHubClient)


@pytest.fixture
def manager(mock_github_client):
    return CheckRunManager(mock_github_client)


@pytest.mark.asyncio
async def test_create_check_run_success(manager, mock_github_client):
    repo = "owner/repo"
    sha = "sha123"
    installation_id = 123
    violations = [
        Violation(
            rule_description="Rule 1", severity=Severity.HIGH, message="Violation 1", details={}, how_to_fix="Fix 1"
        )
    ]

    await manager.create_check_run(repo, sha, installation_id, violations)

    mock_github_client.create_check_run.assert_awaited_once()
    call_args = mock_github_client.create_check_run.call_args[1]
    assert call_args["repo"] == repo
    assert call_args["sha"] == sha
    assert call_args["installation_id"] == installation_id
    assert call_args["conclusion"] == "failure"
    assert "output" in call_args


@pytest.mark.asyncio
async def test_create_check_run_no_sha(manager, mock_github_client):
    await manager.create_check_run("owner/repo", None, 123, [])
    mock_github_client.create_check_run.assert_not_awaited()


@pytest.mark.asyncio
async def test_create_acknowledgment_check_run_success(manager, mock_github_client):
    repo = "owner/repo"
    sha = "sha123"
    installation_id = 123
    violations = [Violation(rule_description="Rule 1", severity=Severity.HIGH, message="V1")]
    acknowledgments = {"Rule 1": "Reason"}

    await manager.create_acknowledgment_check_run(repo, sha, installation_id, [], violations, acknowledgments)

    mock_github_client.create_check_run.assert_awaited_once()
    call_args = mock_github_client.create_check_run.call_args[1]
    assert call_args["conclusion"] == "failure"  # Because violations list is not empty


@pytest.mark.asyncio
async def test_create_acknowledgment_check_run_all_acked(manager, mock_github_client):
    repo = "owner/repo"
    sha = "sha123"
    installation_id = 123
    acked_violations = [Violation(rule_description="Rule 1", severity=Severity.HIGH, message="V1")]
    acknowledgments = {"Rule 1": "Reason"}

    await manager.create_acknowledgment_check_run(repo, sha, installation_id, acked_violations, [], acknowledgments)

    mock_github_client.create_check_run.assert_awaited_once()
    call_args = mock_github_client.create_check_run.call_args[1]
    assert call_args["conclusion"] == "success"
