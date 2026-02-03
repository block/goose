import pytest
from pydantic import ValidationError

from src.webhooks.models import GitHubEventModel, WebhookRepository, WebhookResponse, WebhookSender


class TestWebhookSender:
    """Test WebhookSender model validation."""

    def test_valid_sender(self) -> None:
        """Test valid sender creation."""
        sender = WebhookSender(login="octocat", id=12345, type="User")

        assert sender.login == "octocat"
        assert sender.id == 12345
        assert sender.type == "User"

    def test_missing_required_fields(self) -> None:
        """Test validation fails when required fields are missing."""
        with pytest.raises(ValidationError) as exc_info:
            WebhookSender(login="octocat")  # type: ignore

        errors = exc_info.value.errors()
        error_fields = {err["loc"][0] for err in errors}
        assert "id" in error_fields
        assert "type" in error_fields


class TestWebhookRepository:
    """Test WebhookRepository model validation."""

    def test_valid_repository(self) -> None:
        """Test valid repository creation."""
        repo = WebhookRepository(
            id=123456,
            name="watchflow",
            full_name="octocat/watchflow",
            private=False,
            html_url="https://github.com/octocat/watchflow",
            default_branch="main",
        )

        assert repo.id == 123456
        assert repo.name == "watchflow"
        assert repo.full_name == "octocat/watchflow"
        assert repo.private is False
        assert repo.html_url == "https://github.com/octocat/watchflow"
        assert repo.default_branch == "main"

    def test_default_branch_defaults_to_main(self) -> None:
        """Test default_branch field has 'main' as default."""
        repo = WebhookRepository(
            id=123456,
            name="watchflow",
            full_name="octocat/watchflow",
            private=False,
            html_url="https://github.com/octocat/watchflow",
        )

        assert repo.default_branch == "main"

    def test_missing_required_fields(self) -> None:
        """Test validation fails when required fields are missing."""
        with pytest.raises(ValidationError) as exc_info:
            WebhookRepository(name="watchflow", full_name="octocat/watchflow")  # type: ignore

        errors = exc_info.value.errors()
        error_fields = {err["loc"][0] for err in errors}
        assert "id" in error_fields
        assert "private" in error_fields
        assert "html_url" in error_fields


class TestGitHubEventModel:
    """Test GitHubEventModel validation."""

    def test_valid_pull_request_event(self) -> None:
        """Test valid PR webhook payload."""
        payload = {
            "action": "opened",
            "sender": {"login": "octocat", "id": 1, "type": "User"},
            "repository": {
                "id": 123456,
                "name": "watchflow",
                "full_name": "octocat/watchflow",
                "private": False,
                "html_url": "https://github.com/octocat/watchflow",
            },
        }

        event = GitHubEventModel(**payload)

        assert event.action == "opened"
        assert event.sender.login == "octocat"
        assert event.repository.full_name == "octocat/watchflow"

    def test_valid_push_event_without_action(self) -> None:
        """Test push event doesn't require action field."""
        payload = {
            "sender": {"login": "octocat", "id": 1, "type": "User"},
            "repository": {
                "id": 123456,
                "name": "watchflow",
                "full_name": "octocat/watchflow",
                "private": False,
                "html_url": "https://github.com/octocat/watchflow",
            },
        }

        event = GitHubEventModel(**payload)

        assert event.action is None
        assert event.sender.login == "octocat"
        assert event.repository.full_name == "octocat/watchflow"

    def test_missing_sender_fails_validation(self) -> None:
        """Test validation fails when sender is missing."""
        payload = {
            "action": "opened",
            "repository": {
                "id": 123456,
                "name": "watchflow",
                "full_name": "octocat/watchflow",
                "private": False,
                "html_url": "https://github.com/octocat/watchflow",
            },
        }

        with pytest.raises(ValidationError) as exc_info:
            GitHubEventModel(**payload)

        errors = exc_info.value.errors()
        assert errors[0]["loc"][0] == "sender"

    def test_missing_repository_fails_validation(self) -> None:
        """Test validation fails when repository is missing."""
        payload = {
            "action": "opened",
            "sender": {"login": "octocat", "id": 1, "type": "User"},
        }

        with pytest.raises(ValidationError) as exc_info:
            GitHubEventModel(**payload)

        errors = exc_info.value.errors()
        assert errors[0]["loc"][0] == "repository"


class TestWebhookResponse:
    """Test WebhookResponse model."""

    def test_valid_success_response(self) -> None:
        """Test successful response creation."""
        response = WebhookResponse(status="success", detail="Event processed", event_type="pull_request")

        assert response.status == "success"
        assert response.detail == "Event processed"
        assert response.event_type == "pull_request"

    def test_minimal_response(self) -> None:
        """Test response with only required fields."""
        response = WebhookResponse(status="queued", detail="Event queued", event_type="pull_request")

        assert response.status == "queued"
        assert response.detail == "Event queued"
        assert response.event_type == "pull_request"

    def test_error_response(self) -> None:
        """Test error response with detail."""
        response = WebhookResponse(status="error", detail="Processing failed", event_type="push")

        assert response.status == "error"
        assert response.detail == "Processing failed"
        assert response.event_type == "push"
