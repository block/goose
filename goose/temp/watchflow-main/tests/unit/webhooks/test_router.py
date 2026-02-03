from unittest.mock import AsyncMock, patch

import pytest
from fastapi import FastAPI
from httpx import ASGITransport, AsyncClient

from src.webhooks.router import router


async def mock_verify_signature() -> bool:
    """Mock verification dependency that always returns True."""
    return True


@pytest.fixture
def app() -> FastAPI:
    """Create FastAPI test app with webhook router."""
    from src.webhooks.auth import verify_github_signature

    test_app = FastAPI()
    test_app.include_router(router, prefix="/webhooks")
    # Override the dependency for testing
    test_app.dependency_overrides[verify_github_signature] = lambda: True
    return test_app


@pytest.fixture
def valid_pr_payload() -> dict[str, object]:
    """Valid pull request webhook payload."""
    return {
        "action": "opened",
        "sender": {"login": "octocat", "id": 1, "type": "User"},
        "repository": {
            "id": 123456,
            "name": "watchflow",
            "full_name": "octocat/watchflow",
            "private": False,
            "html_url": "https://github.com/octocat/watchflow",
        },
        "pull_request": {"number": 42, "title": "Test PR"},
    }


@pytest.fixture
def valid_headers() -> dict[str, str]:
    """Valid GitHub webhook headers."""
    return {
        "X-GitHub-Event": "pull_request",
        "X-Hub-Signature-256": "sha256=mock_signature",
        "Content-Type": "application/json",
    }


class TestWebhookRouter:
    """Test webhook router endpoint."""

    @pytest.mark.asyncio
    async def test_github_webhook_success(
        self, app: FastAPI, valid_pr_payload: dict[str, object], valid_headers: dict[str, str]
    ) -> None:
        """Test successful webhook processing."""
        with patch("src.webhooks.router.dispatcher.dispatch", new_callable=AsyncMock) as mock_dispatch:
            mock_dispatch.return_value = {"status": "queued", "event_type": "pull_request"}

            async with AsyncClient(transport=ASGITransport(app=app), base_url="http://test") as client:
                response = await client.post("/webhooks/github", json=valid_pr_payload, headers=valid_headers)

                assert response.status_code == 200
                result = response.json()
                assert result["status"] == "ok"
                assert result["event_type"] == "pull_request"

                # Verify dispatcher was called
                assert mock_dispatch.called
                call_args = mock_dispatch.call_args
                event_arg = call_args[0][0]
                assert event_arg.event_type.value == "pull_request"
                assert event_arg.payload == valid_pr_payload

    @pytest.mark.asyncio
    async def test_missing_event_header(self, app: FastAPI, valid_pr_payload: dict[str, object]) -> None:
        """Test webhook fails without X-GitHub-Event header."""
        headers = {
            "X-Hub-Signature-256": "sha256=mock_signature",
            "Content-Type": "application/json",
        }

        async with AsyncClient(transport=ASGITransport(app=app), base_url="http://test") as client:
            response = await client.post("/webhooks/github", json=valid_pr_payload, headers=headers)

        assert response.status_code == 400
        assert "Missing X-GitHub-Event header" in response.json()["detail"]

    @pytest.mark.asyncio
    async def test_invalid_payload_structure(self, app: FastAPI, valid_headers: dict[str, str]) -> None:
        """Test webhook fails with malformed payload."""
        invalid_payload = {
            "action": "opened",
            # Missing required 'sender' and 'repository' fields
        }

        async with AsyncClient(transport=ASGITransport(app=app), base_url="http://test") as client:
            response = await client.post("/webhooks/github", json=invalid_payload, headers=valid_headers)

        assert response.status_code == 400
        assert "Invalid webhook payload structure" in response.json()["detail"]

    @pytest.mark.asyncio
    async def test_unsupported_event_type(self, app: FastAPI, valid_pr_payload: dict[str, object]) -> None:
        """Test webhook handles unsupported event types gracefully."""
        headers = {
            "X-GitHub-Event": "unsupported_event_type",
            "X-Hub-Signature-256": "sha256=mock_signature",
            "Content-Type": "application/json",
        }

        async with AsyncClient(transport=ASGITransport(app=app), base_url="http://test") as client:
            response = await client.post("/webhooks/github", json=valid_pr_payload, headers=headers)

        # Should return 202 for unsupported events per router logic
        assert response.status_code == 200
        result = response.json()
        assert result["status"] == "ignored"

    @pytest.mark.asyncio
    async def test_push_event_without_action(self, app: FastAPI, valid_headers: dict[str, str]) -> None:
        """Test push events work without action field."""
        push_payload = {
            "sender": {"login": "octocat", "id": 1, "type": "User"},
            "repository": {
                "id": 123456,
                "name": "watchflow",
                "full_name": "octocat/watchflow",
                "private": False,
                "html_url": "https://github.com/octocat/watchflow",
            },
            "ref": "refs/heads/main",
            "commits": [],
        }

        push_headers = {**valid_headers, "X-GitHub-Event": "push"}

        with patch("src.webhooks.router.dispatcher.dispatch", new_callable=AsyncMock) as mock_dispatch:
            mock_dispatch.return_value = {"status": "queued", "event_type": "push"}

            async with AsyncClient(transport=ASGITransport(app=app), base_url="http://test") as client:
                response = await client.post("/webhooks/github", json=push_payload, headers=push_headers)

            assert response.status_code == 200
            result = response.json()
            assert result["status"] == "ok"
            assert result["event_type"] == "push"

            # Verify dispatcher was called
            assert mock_dispatch.called

    @pytest.mark.asyncio
    async def test_structured_logging_on_validation(
        self, app: FastAPI, valid_pr_payload: dict[str, object], valid_headers: dict[str, str]
    ) -> None:
        """Test that structured logging captures webhook validation."""
        with (
            patch("src.webhooks.router.dispatcher.dispatch", new_callable=AsyncMock) as mock_dispatch,
            patch("src.webhooks.router.logger") as mock_logger,
        ):
            mock_dispatch.return_value = {"status": "queued", "event_type": "pull_request"}

            async with AsyncClient(transport=ASGITransport(app=app), base_url="http://test") as client:
                response = await client.post("/webhooks/github", json=valid_pr_payload, headers=valid_headers)

            assert response.status_code == 200

            # Verify structured logging was called
            assert mock_logger.info.called
            # Check that webhook_validated was logged
            calls = [call for call in mock_logger.info.call_args_list if "webhook_validated" in str(call)]
            assert len(calls) > 0
