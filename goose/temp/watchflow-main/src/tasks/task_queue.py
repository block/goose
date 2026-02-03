import asyncio
import hashlib
import json
from collections import OrderedDict
from collections.abc import Callable, Coroutine
from typing import Any

import structlog
from pydantic import BaseModel, Field

logger = structlog.get_logger()

# Configuration constants
MAX_DEDUP_CACHE_SIZE = 10000  # Maximum entries in deduplication cache
MAX_RETRIES = 3  # Maximum retry attempts for failed tasks
INITIAL_BACKOFF_SECONDS = 1.0  # Initial backoff for exponential retry


class Task(BaseModel):
    """Strictly typed task container for the queue."""

    task_id: str = Field(..., description="Unique hash for deduplication")
    event_type: str = Field(..., description="GitHub event type")
    payload: dict[str, Any] = Field(..., description="Event payload for hash generation")
    func: Callable[..., Coroutine[Any, Any, Any]] | Any = Field(..., description="Handler function to execute")
    args: tuple[Any, ...] = Field(default_factory=tuple, description="Positional arguments")
    kwargs: dict[str, Any] = Field(default_factory=dict, description="Keyword arguments")
    retry_count: int = Field(default=0, description="Number of retry attempts")

    model_config = {"arbitrary_types_allowed": True}

    @property
    def repo_full_name(self) -> str:
        """Helper to extract repo full name from payload."""
        repo = self.payload.get("repository", {})
        if isinstance(repo, dict):
            return str(repo.get("full_name", ""))
        return ""

    @property
    def installation_id(self) -> int | None:
        """Helper to extract installation ID from payload."""
        installation = self.payload.get("installation")
        if isinstance(installation, dict):
            inst_id = installation.get("id")
            return int(inst_id) if inst_id is not None else None
        return None


def _is_transient_error(error: Exception) -> bool:
    """Determine if an error is transient and worth retrying."""
    transient_types = (
        ConnectionError,
        TimeoutError,
        asyncio.TimeoutError,
    )
    # Check exception type or message for transient indicators
    if isinstance(error, transient_types):
        return True
    error_msg = str(error).lower()
    return any(indicator in error_msg for indicator in ("timeout", "connection", "rate limit", "503", "429"))


class TaskQueue:
    """
    In-memory task queue with deduplication as per Blueprint 2.3.C.
    Prevents processing the same GitHub event multiple times.

    Open-source version: In-memory deduplication with LRU eviction.
    """

    def __init__(self, max_dedup_size: int = MAX_DEDUP_CACHE_SIZE) -> None:
        self.queue: asyncio.Queue[Task] = asyncio.Queue(maxsize=100)
        # LRU-based deduplication cache (prevents memory leaks)
        self._dedup_cache: OrderedDict[str, bool] = OrderedDict()
        self._max_dedup_size = max_dedup_size
        self.workers: list[asyncio.Task[None]] = []

    @property
    def processed_hashes(self) -> set[str]:
        """Backward compatibility: return set view of deduplication cache keys."""
        return set(self._dedup_cache.keys())

    def _add_to_dedup_cache(self, task_id: str) -> None:
        """Add task_id to deduplication cache with LRU eviction."""
        if task_id in self._dedup_cache:
            self._dedup_cache.move_to_end(task_id)

            return

        while len(self._dedup_cache) >= self._max_dedup_size:
            self._dedup_cache.popitem(last=False)

        self._dedup_cache[task_id] = True

    def _is_duplicate(self, task_id: str) -> bool:
        """Check if task_id is in deduplication cache."""
        return task_id in self._dedup_cache

    def _generate_task_id(
        self,
        event_type: str,
        payload: dict[str, Any],
        delivery_id: str | None = None,
        func: Any = None,
    ) -> str:
        """Creates a unique hash for deduplication.

        When delivery_id (X-GitHub-Delivery) is present, use it (plus func qualname
        so "run handler" and "run processor" get distinct IDs) so each webhook
        delivery is processed. Otherwise fall back to payload hash.
        """
        if delivery_id:
            qualname = getattr(func, "__qualname__", "") or ""
            raw_string = f"{event_type}:{delivery_id}:{qualname}"
        else:
            payload_str = json.dumps(payload, sort_keys=True)
            raw_string = f"{event_type}:{payload_str}"
        return hashlib.sha256(raw_string.encode()).hexdigest()

    def build_task(
        self,
        event_type: str,
        payload: dict[str, Any],
        func: Callable[..., Coroutine[Any, Any, Any]],
        delivery_id: str | None = None,
    ) -> Task:
        """Build a Task for a processor; pass as single arg to enqueue."""
        task_id = self._generate_task_id(event_type, payload, delivery_id=delivery_id, func=func)
        return Task(
            task_id=task_id,
            event_type=event_type,
            payload=payload,
            func=func,
            args=(),
            kwargs={},
        )

    async def enqueue(
        self,
        func: Callable[..., Coroutine[Any, Any, Any]],
        event_type: str,
        payload: dict[str, Any],
        *args: Any,
        delivery_id: str | None = None,
        **kwargs: Any,
    ) -> bool:
        """Adds a task to the queue if it is not a duplicate."""
        task_id = self._generate_task_id(event_type, payload, delivery_id=delivery_id, func=func)

        if self._is_duplicate(task_id):
            logger.info("task_skipped_duplicate", task_id=task_id, event_type=event_type)
            return False

        task = Task(task_id=task_id, event_type=event_type, payload=payload, func=func, args=args, kwargs=kwargs)
        await self.queue.put(task)
        self._add_to_dedup_cache(task_id)

        logger.info("task_enqueued", task_id=task_id, event_type=event_type)
        return True

    async def _execute_with_retry(self, task: Task) -> None:
        """Execute task with exponential backoff retry for transient failures."""
        last_error: Exception | None = None

        for attempt in range(MAX_RETRIES + 1):
            try:
                await task.func(*task.args, **task.kwargs)
                if attempt > 0:
                    logger.info("task_retry_succeeded", task_id=task.task_id, attempt=attempt + 1)
                return
            except Exception as e:
                last_error = e
                if attempt < MAX_RETRIES and _is_transient_error(e):
                    backoff = INITIAL_BACKOFF_SECONDS * (2**attempt)
                    logger.warning(
                        "task_retry_scheduled",
                        task_id=task.task_id,
                        attempt=attempt + 1,
                        backoff_seconds=backoff,
                        error=str(e),
                    )
                    await asyncio.sleep(backoff)
                else:
                    break

        logger.error(
            "task_failed",
            task_id=task.task_id,
            error=str(last_error),
            attempts=min(task.retry_count + 1, MAX_RETRIES + 1),
            exc_info=True,
        )

    async def _worker(self) -> None:
        """Background worker loop."""
        while True:
            task = await self.queue.get()
            try:
                logger.info("task_started", task_id=task.task_id, event_type=task.event_type)
                await self._execute_with_retry(task)
                logger.info("task_completed", task_id=task.task_id)
            except Exception as e:
                logger.error("task_worker_error", task_id=task.task_id, error=str(e), exc_info=True)

            finally:
                self.queue.task_done()

    async def start_workers(self, num_workers: int = 1) -> None:
        """Starts the background workers."""
        if not self.workers:
            for _ in range(num_workers):
                task = asyncio.create_task(self._worker())
                self.workers.append(task)
            logger.info("task_queue_workers_started", count=num_workers)

    async def stop_workers(self) -> None:
        """Stops the background workers."""
        if self.workers:
            for task in self.workers:
                task.cancel()
            await asyncio.gather(*self.workers, return_exceptions=True)
            self.workers.clear()
            logger.info("task_queue_workers_stopped")

    def get_stats(self) -> dict[str, Any]:
        """Get queue statistics for health checks."""
        return {
            "queue_size": self.queue.qsize(),
            "dedup_cache_size": len(self._dedup_cache),
            "dedup_cache_max": self._max_dedup_size,
            "worker_count": len(self.workers),
        }


# Global singleton for the application
task_queue = TaskQueue()
