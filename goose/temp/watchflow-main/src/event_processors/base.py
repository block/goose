import logging
from abc import ABC, abstractmethod
from typing import Any

from pydantic import BaseModel, Field

from src.core.models import Violation, WebhookEvent
from src.integrations.github import github_client
from src.rules.interface import RuleLoader
from src.rules.loaders.github_loader import GitHubRuleLoader
from src.tasks.task_queue import Task

logger = logging.getLogger(__name__)


class ProcessingResult(BaseModel):
    """Result of event processing."""

    success: bool
    violations: list[Violation] = Field(default_factory=list)
    api_calls_made: int
    processing_time_ms: int
    error: str | None = None


class BaseEventProcessor(ABC):
    """Base class for all event processors."""

    def __init__(self) -> None:
        self.rule_provider = self._get_rule_provider()
        self.github_client = github_client

    @abstractmethod
    async def process(self, task: Task) -> ProcessingResult:
        """Process the event task."""
        pass

    @abstractmethod
    def get_event_type(self) -> str:
        """Get the event type this processor handles."""
        pass

    @abstractmethod
    async def prepare_webhook_data(self, task: Task) -> dict[str, Any]:
        """Prepare data from webhook payload."""
        pass

    @abstractmethod
    async def prepare_api_data(self, task: Task) -> dict[str, Any]:
        """Prepare data from GitHub API calls."""
        pass

    def _get_rule_provider(self) -> RuleLoader:
        """Get the rule provider for this processor."""
        return GitHubRuleLoader(github_client)

    async def _create_webhook_event(self, task: Task) -> WebhookEvent:
        """Create a WebhookEvent from the task."""
        from src.core.models import EventType

        return WebhookEvent(event_type=EventType(task.event_type), payload=task.payload)
