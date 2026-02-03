from unittest.mock import AsyncMock, MagicMock

import pytest

from src.event_processors.pull_request.enricher import PullRequestEnricher


@pytest.fixture
def mock_github_client():
    return AsyncMock()


@pytest.fixture
def enricher(mock_github_client):
    return PullRequestEnricher(mock_github_client)


@pytest.fixture
def mock_task():
    task = MagicMock()
    task.repo_full_name = "owner/repo"
    task.installation_id = 12345
    task.payload = {
        "pull_request": {"number": 1, "user": {"login": "author"}},
        "repository": {"full_name": "owner/repo"},
        "organization": {"login": "org"},
        "event_id": "evt_123",
        "timestamp": "2024-01-01T00:00:00Z",
    }
    return task


@pytest.mark.asyncio
async def test_fetch_api_data_success(enricher, mock_github_client):
    mock_github_client.get_pull_request_reviews.return_value = [{"id": 1}]
    mock_github_client.get_pull_request_files.return_value = [{"filename": "f1.txt"}]

    data = await enricher.fetch_api_data("owner/repo", 1, 12345)

    assert data["reviews"] == [{"id": 1}]
    assert data["files"] == [{"filename": "f1.txt"}]
    mock_github_client.get_pull_request_reviews.assert_called_once()
    mock_github_client.get_pull_request_files.assert_called_once()


@pytest.mark.asyncio
async def test_enrich_event_data(enricher, mock_task, mock_github_client):
    mock_github_client.get_pull_request_reviews.return_value = []
    mock_github_client.get_pull_request_files.return_value = [
        {"filename": "test.py", "status": "added", "additions": 10, "deletions": 0, "patch": "+print('hello')"}
    ]

    event_data = await enricher.enrich_event_data(mock_task, "fake_token")

    assert event_data["pull_request_details"]["number"] == 1
    assert event_data["triggering_user"]["login"] == "author"
    assert "reviews" in event_data
    assert "files" in event_data
    assert len(event_data["changed_files"]) == 1
    assert event_data["changed_files"][0]["filename"] == "test.py"
    assert "diff_summary" in event_data


@pytest.mark.asyncio
async def test_fetch_acknowledgments(enricher, mock_github_client):
    mock_github_client.get_issue_comments.return_value = [
        {
            "body": "🚨 Watchflow Rule Violations Detected\n\n**Reason:** valid reason\n\n---\nThe following violations have been overridden:\n• **Rule** - Pull request does not have the minimum required approvals\n",
            "user": {"login": "reviewer"},
        }
    ]

    acks = await enricher.fetch_acknowledgments("owner/repo", 1, 12345)

    assert "min-pr-approvals" in acks
    assert acks["min-pr-approvals"].reason == "valid reason"
    assert acks["min-pr-approvals"].commenter == "reviewer"


def test_summarize_files_truncates(enricher):
    files = [
        {
            "filename": "large.py",
            "status": "modified",
            "additions": 100,
            "deletions": 50,
            "patch": "\n".join([f"+line {i}" for i in range(20)]),
        }
    ]

    summary = enricher.summarize_files(files, max_files=1, max_patch_lines=5)

    assert "- large.py (modified, +100/-50)" in summary
    assert "line 4" in summary
    assert "line 5" not in summary
    assert "... (diff truncated)" in summary


def test_summarize_files_empty(enricher):
    assert enricher.summarize_files([]) == ""


def test_prepare_webhook_data(enricher, mock_task):
    data = enricher.prepare_webhook_data(mock_task)

    assert data["event_type"] == "pull_request"
    assert data["pull_request"]["number"] == 1
    assert data["pull_request"]["user"] == "author"
