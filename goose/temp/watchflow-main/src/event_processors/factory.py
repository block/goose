from src.event_processors.base import BaseEventProcessor
from src.event_processors.check_run import CheckRunProcessor
from src.event_processors.deployment import DeploymentProcessor
from src.event_processors.deployment_protection_rule import DeploymentProtectionRuleProcessor
from src.event_processors.deployment_review import DeploymentReviewProcessor
from src.event_processors.deployment_status import DeploymentStatusProcessor
from src.event_processors.pull_request import PullRequestProcessor
from src.event_processors.push import PushProcessor
from src.event_processors.rule_creation import RuleCreationProcessor
from src.event_processors.violation_acknowledgment import ViolationAcknowledgmentProcessor


class EventProcessorFactory:
    """Factory for creating event processors."""

    _processors: dict[str, type[BaseEventProcessor]] = {
        "pull_request": PullRequestProcessor,
        "push": PushProcessor,
        "check_run": CheckRunProcessor,
        "deployment_review": DeploymentReviewProcessor,
        "deployment_status": DeploymentStatusProcessor,
        "deployment": DeploymentProcessor,  # Process deployment events for rule checking
        "deployment_protection_rule": DeploymentProtectionRuleProcessor,
        "rule_creation": RuleCreationProcessor,
        "violation_acknowledgment": ViolationAcknowledgmentProcessor,
    }

    @classmethod
    def create_processor(cls, event_type: str) -> BaseEventProcessor:
        """Create a processor for the given event type."""
        processor_class = cls._processors.get(event_type)
        if not processor_class:
            raise ValueError(f"No processor found for event type: {event_type}")

        return processor_class()

    @classmethod
    def register_processor(cls, event_type: str, processor_class: type[BaseEventProcessor]) -> None:
        """Register a new processor."""
        cls._processors[event_type] = processor_class

    @classmethod
    def get_processor(cls, event_type: str) -> BaseEventProcessor:
        """Get processor for the given event type."""
        processor_class = cls._processors.get(event_type)
        if not processor_class:
            raise ValueError(f"No processor found for event type: {event_type}")
        return processor_class()

    @classmethod
    def get_supported_event_types(cls) -> list[str]:
        """Get list of supported event types."""
        return list(cls._processors.keys())
