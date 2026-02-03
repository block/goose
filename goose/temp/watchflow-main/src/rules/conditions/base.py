"""Base condition interface for rule validation.

This module defines the abstract base class that all conditions must implement.
"""

import logging
from abc import ABC, abstractmethod
from typing import Any

from src.core.models import Violation

logger = logging.getLogger(__name__)


class BaseCondition(ABC):
    """Abstract base class for all condition validators.

    All condition classes should inherit from this base and implement
    the evaluate method to perform their specific validation logic.

    Attributes:
        name: Unique identifier for the condition type.
        description: Human-readable description of what the condition validates.
        parameter_patterns: List of parameter keys this condition uses.
        event_types: List of event types this condition applies to.
        examples: Example parameter configurations for documentation.
    """

    name: str = ""
    description: str = ""
    parameter_patterns: list[str] = []
    event_types: list[str] = []
    examples: list[dict[str, Any]] = []

    @abstractmethod
    async def evaluate(self, context: Any) -> list[Violation]:
        """Evaluate the condition against the provided context.

        Args:
            context: The context data to evaluate against. This typically
                includes event data, parameters, and any other relevant info.

        Returns:
            A list of Violation objects if the condition is not met,
            or an empty list if the condition passes.
        """
        pass

    async def validate(self, parameters: dict[str, Any], event: dict[str, Any]) -> bool:
        """Legacy validation interface for backward compatibility.

        This method wraps the evaluate method to maintain compatibility
        with the existing validator interface.

        Args:
            parameters: The parameters for this condition type.
            event: The webhook event to validate against.

        Returns:
            True if the condition is met (no violations), False otherwise.
        """
        context = {"parameters": parameters, "event": event}
        violations = await self.evaluate(context)
        return len(violations) == 0

    def get_description(self) -> dict[str, Any]:
        """Get validator description for dynamic strategy selection."""
        return {
            "name": self.name,
            "description": self.description,
            "parameter_patterns": self.parameter_patterns,
            "event_types": self.event_types,
            "examples": self.examples,
        }
