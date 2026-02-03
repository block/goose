import asyncio
from unittest.mock import AsyncMock, patch

import pytest
import respx
from fastapi import FastAPI
from httpx import ASGITransport, AsyncClient

from src.tasks.task_queue import TaskQueue
from src.webhooks.auth import verify_github_signature
from src.webhooks.dispatcher import WebhookDispatcher
from src.webhooks.router import router


@pytest.fixture
def app() -> FastAPI:
    """Create FastAPI test app with webhook router."""
    test_app = FastAPI()
    test_app.include_router(router, prefix="/webhooks")
    # Bypass GitHub signature verification for integration tests
    test_app.dependency_overrides[verify_github_signature] = lambda: True
    return test_app


@pytest.fixture
def fresh_dispatcher(fresh_queue: TaskQueue) -> WebhookDispatcher:
    """Create a fresh dispatcher instance for testing with injected queue."""
    return WebhookDispatcher(queue=fresh_queue)


@pytest.fixture
def fresh_queue() -> TaskQueue:
    """Create a fresh task queue for testing."""
    return TaskQueue()


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
        "pull_request": {"number": 42, "title": "Test PR", "body": "Test body"},
    }


@pytest.fixture
def valid_headers() -> dict[str, str]:
    """Valid GitHub webhook headers."""
    return {
        "X-GitHub-Event": "pull_request",
        "X-Hub-Signature-256": "sha256=mock_signature",
        "Content-Type": "application/json",
    }


class TestWebhookFlowIntegration:
    """Integration tests for Router -> Dispatcher -> TaskQueue flow."""

    @pytest.mark.asyncio
    @respx.mock
    async def test_end_to_end_webhook_flow(
        self,
        app: FastAPI,
        fresh_dispatcher: WebhookDispatcher,
        fresh_queue: TaskQueue,
        valid_pr_payload: dict[str, object],
        valid_headers: dict[str, str],
    ) -> None:
        """Test complete flow from webhook ingress to task queue."""
        # Create a mock handler
        mock_handler = AsyncMock()

        # Register handler with dispatcher
        fresh_dispatcher.register_handler("pull_request", mock_handler)

        # Start the task queue worker
        await fresh_queue.start_workers()

        with patch("src.webhooks.router.dispatcher", fresh_dispatcher):
            async with AsyncClient(transport=ASGITransport(app=app), base_url="http://test") as client:
                response = await client.post("/webhooks/github", json=valid_pr_payload, headers=valid_headers)

            assert response.status_code == 200
            result = response.json()
            assert result["status"] == "ok"

            # Wait for task queue to process
            await asyncio.sleep(0.2)
            await fresh_queue.queue.join()

            # Verify handler was called via task queue
            assert mock_handler.called

    @pytest.mark.asyncio
    @respx.mock
    async def test_webhook_deduplication_across_flow(
        self,
        app: FastAPI,
        fresh_dispatcher: WebhookDispatcher,
        fresh_queue: TaskQueue,
        valid_pr_payload: dict[str, object],
        valid_headers: dict[str, str],
    ) -> None:
        """Test that duplicate webhooks are deduplicated in task queue."""
        mock_handler = AsyncMock()
        fresh_dispatcher.register_handler("pull_request", mock_handler)
        await fresh_queue.start_workers()

        with patch("src.webhooks.router.dispatcher", fresh_dispatcher):
            async with AsyncClient(transport=ASGITransport(app=app), base_url="http://test") as client:
                # Send same webhook twice
                response1 = await client.post("/webhooks/github", json=valid_pr_payload, headers=valid_headers)
                response2 = await client.post("/webhooks/github", json=valid_pr_payload, headers=valid_headers)

            assert response1.status_code == 200
            assert response2.status_code == 200

            # Wait for processing
            await asyncio.sleep(0.2)
            await fresh_queue.queue.join()

            # Handler should only be called once due to deduplication
            assert mock_handler.call_count == 1

    @pytest.mark.asyncio
    @respx.mock
    async def test_multiple_event_types_flow(
        self,
        app: FastAPI,
        fresh_dispatcher: WebhookDispatcher,
        fresh_queue: TaskQueue,
        valid_pr_payload: dict[str, object],
    ) -> None:
        """Test handling multiple event types through the flow."""
        pr_handler = AsyncMock()
        push_handler = AsyncMock()

        fresh_dispatcher.register_handler("pull_request", pr_handler)
        fresh_dispatcher.register_handler("push", push_handler)
        await fresh_queue.start_workers()

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

        with patch("src.webhooks.router.dispatcher", fresh_dispatcher):
            async with AsyncClient(transport=ASGITransport(app=app), base_url="http://test") as client:
                # Send PR event
                pr_response = await client.post(
                    "/webhooks/github",
                    json=valid_pr_payload,
                    headers={
                        "X-GitHub-Event": "pull_request",
                        "X-Hub-Signature-256": "sha256=mock",
                    },
                )

                # Send push event
                push_response = await client.post(
                    "/webhooks/github",
                    json=push_payload,
                    headers={
                        "X-GitHub-Event": "push",
                        "X-Hub-Signature-256": "sha256=mock",
                    },
                )

            assert pr_response.status_code == 200
            assert push_response.status_code == 200

            # Wait for processing
            await asyncio.sleep(0.2)
            await fresh_queue.queue.join()

            # Both handlers should be called
            assert pr_handler.called
            assert push_handler.called

    @pytest.mark.asyncio
    @respx.mock
    async def test_handler_exception_doesnt_break_flow(
        self,
        app: FastAPI,
        fresh_dispatcher: WebhookDispatcher,
        fresh_queue: TaskQueue,
        valid_pr_payload: dict[str, object],
        valid_headers: dict[str, str],
    ) -> None:
        """Test that handler exceptions are caught and don't break the flow."""
        # Create a handler that raises an exception
        failing_handler = AsyncMock(side_effect=ValueError("Test error"))
        fresh_dispatcher.register_handler("pull_request", failing_handler)
        await fresh_queue.start_workers()

        with patch("src.webhooks.router.dispatcher", fresh_dispatcher):
            async with AsyncClient(transport=ASGITransport(app=app), base_url="http://test") as client:
                response = await client.post("/webhooks/github", json=valid_pr_payload, headers=valid_headers)

            # Webhook should still be accepted
            assert response.status_code == 200

            # Wait for processing
            await asyncio.sleep(0.2)
            await fresh_queue.queue.join()

            # Handler was called and exception was caught
            assert failing_handler.called

    @pytest.mark.asyncio
    @respx.mock
    async def test_no_handler_registered_flow(
        self,
        app: FastAPI,
        fresh_dispatcher: WebhookDispatcher,
        fresh_queue: TaskQueue,
        valid_pr_payload: dict[str, object],
        valid_headers: dict[str, str],
    ) -> None:
        """Test flow when no handler is registered for event type."""
        # Don't register any handlers
        await fresh_queue.start_workers()

        with patch("src.webhooks.router.dispatcher", fresh_dispatcher):
            async with AsyncClient(transport=ASGITransport(app=app), base_url="http://test") as client:
                response = await client.post("/webhooks/github", json=valid_pr_payload, headers=valid_headers)

            # Should still return success (webhook accepted)
            assert response.status_code == 200

            # Wait briefly
            await asyncio.sleep(0.1)

            # Queue should be empty (nothing to process)
            assert fresh_queue.queue.qsize() == 0
