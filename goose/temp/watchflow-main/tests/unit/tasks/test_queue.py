import asyncio
from unittest.mock import AsyncMock

import pytest

from src.tasks.task_queue import TaskQueue


class TestTaskQueue:
    """Test TaskQueue deduplication and execution."""

    @pytest.fixture
    def queue(self) -> TaskQueue:
        """Create a fresh TaskQueue instance for each test."""
        return TaskQueue()

    @pytest.fixture
    def sample_payload(self) -> dict[str, object]:
        """Sample GitHub webhook payload."""
        return {
            "action": "opened",
            "sender": {"login": "octocat", "id": 1},
            "repository": {"id": 123, "full_name": "octocat/test"},
            "pull_request": {"number": 42},
        }

    @pytest.mark.asyncio
    async def test_enqueue_success(self, queue: TaskQueue, sample_payload: dict[str, object]) -> None:
        """Test successful task enqueue."""
        handler = AsyncMock()

        result = await queue.enqueue(handler, "pull_request", sample_payload)

        assert result is True
        assert queue.queue.qsize() == 1
        assert len(queue.processed_hashes) == 1

    @pytest.mark.asyncio
    async def test_enqueue_deduplication(self, queue: TaskQueue, sample_payload: dict[str, object]) -> None:
        """Test that duplicate events are not enqueued twice."""
        handler = AsyncMock()

        # First enqueue should succeed
        result1 = await queue.enqueue(handler, "pull_request", sample_payload)
        assert result1 is True
        assert queue.queue.qsize() == 1

        # Second enqueue with same payload should be deduplicated
        result2 = await queue.enqueue(handler, "pull_request", sample_payload)
        assert result2 is False
        assert queue.queue.qsize() == 1  # Queue size unchanged
        assert len(queue.processed_hashes) == 1

    @pytest.mark.asyncio
    async def test_enqueue_different_events_not_deduplicated(
        self, queue: TaskQueue, sample_payload: dict[str, object]
    ) -> None:
        """Test that different events are not deduplicated."""
        handler = AsyncMock()

        # Enqueue first event
        result1 = await queue.enqueue(handler, "pull_request", sample_payload)
        assert result1 is True

        # Modify payload slightly
        different_payload = {**sample_payload, "action": "closed"}

        # Second enqueue with different payload should succeed
        result2 = await queue.enqueue(handler, "pull_request", different_payload)
        assert result2 is True
        assert queue.queue.qsize() == 2
        assert len(queue.processed_hashes) == 2

    @pytest.mark.asyncio
    async def test_enqueue_different_event_types_not_deduplicated(
        self, queue: TaskQueue, sample_payload: dict[str, object]
    ) -> None:
        """Test that same payload with different event types are not deduplicated."""
        handler = AsyncMock()

        # Enqueue as pull_request
        result1 = await queue.enqueue(handler, "pull_request", sample_payload)
        assert result1 is True

        # Enqueue same payload as push
        result2 = await queue.enqueue(handler, "push", sample_payload)
        assert result2 is True
        assert queue.queue.qsize() == 2
        assert len(queue.processed_hashes) == 2

    @pytest.mark.asyncio
    async def test_worker_processes_tasks(self, queue: TaskQueue, sample_payload: dict[str, object]) -> None:
        """Test that worker processes enqueued tasks."""
        handler = AsyncMock()

        # Start the worker
        await queue.start_workers()

        # Enqueue a task
        await queue.enqueue(handler, "pull_request", sample_payload)

        # Wait for worker to process
        await asyncio.sleep(0.1)
        await queue.queue.join()

        # Verify handler was called
        assert handler.called

    @pytest.mark.asyncio
    async def test_worker_handles_exceptions(self, queue: TaskQueue, sample_payload: dict[str, object]) -> None:
        """Test that worker continues after handler raises exception."""
        # Create a handler that raises an exception
        failing_handler = AsyncMock(side_effect=ValueError("Test error"))
        success_handler = AsyncMock()

        # Start the worker
        await queue.start_workers()

        # Enqueue failing task
        await queue.enqueue(failing_handler, "pull_request", sample_payload)

        # Enqueue successful task with different payload
        different_payload = {**sample_payload, "action": "closed"}
        await queue.enqueue(success_handler, "pull_request", different_payload)

        # Wait for worker to process both
        await asyncio.sleep(0.2)
        await queue.queue.join()

        # Verify both handlers were called despite first one failing
        assert failing_handler.called
        assert success_handler.called

    @pytest.mark.asyncio
    async def test_task_id_generation_deterministic(self, queue: TaskQueue, sample_payload: dict[str, object]) -> None:
        """Test that same payload generates same task_id."""
        task_id_1 = queue._generate_task_id("pull_request", sample_payload)
        task_id_2 = queue._generate_task_id("pull_request", sample_payload)

        assert task_id_1 == task_id_2

    @pytest.mark.asyncio
    async def test_task_id_generation_unique_for_different_payloads(
        self, queue: TaskQueue, sample_payload: dict[str, object]
    ) -> None:
        """Test that different payloads generate different task_ids."""
        task_id_1 = queue._generate_task_id("pull_request", sample_payload)

        different_payload = {**sample_payload, "action": "closed"}
        task_id_2 = queue._generate_task_id("pull_request", different_payload)

        assert task_id_1 != task_id_2

    @pytest.mark.asyncio
    async def test_task_id_with_delivery_id_unique_per_delivery(
        self, queue: TaskQueue, sample_payload: dict[str, object]
    ) -> None:
        """Test that with delivery_id, same payload but different delivery_id gets different task_ids (redeliveries processed)."""
        task_id_1 = queue._generate_task_id(
            "pull_request", sample_payload, delivery_id="delivery-abc", func=AsyncMock()
        )
        task_id_2 = queue._generate_task_id(
            "pull_request", sample_payload, delivery_id="delivery-xyz", func=AsyncMock()
        )
        assert task_id_1 != task_id_2
        # Same delivery_id + same func = same task_id
        task_id_3 = queue._generate_task_id(
            "pull_request", sample_payload, delivery_id="delivery-abc", func=AsyncMock()
        )
        assert task_id_1 == task_id_3

    @pytest.mark.asyncio
    async def test_enqueue_with_args_and_kwargs(self, queue: TaskQueue, sample_payload: dict[str, object]) -> None:
        """Test enqueue passes args and kwargs to handler."""
        handler = AsyncMock()

        # Start worker
        await queue.start_workers()

        # Enqueue with additional args and kwargs
        event_mock = {"test": "data"}
        await queue.enqueue(handler, "pull_request", sample_payload, event_mock, timeout=30)

        # Wait for processing
        await asyncio.sleep(0.1)
        await queue.queue.join()

        # Verify handler was called with correct args and kwargs
        assert handler.called
        call_args, call_kwargs = handler.call_args
        assert call_args[0] == event_mock
        assert call_kwargs["timeout"] == 30
