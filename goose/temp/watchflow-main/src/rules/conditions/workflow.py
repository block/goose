"""Workflow conditions for rule validation.

This module contains conditions that validate workflow-related aspects
such as workflow duration thresholds.
"""

from typing import Any

import structlog

from src.core.models import Severity, Violation
from src.rules.conditions.base import BaseCondition

logger = structlog.get_logger(__name__)


class WorkflowDurationCondition(BaseCondition):
    """Validates if a workflow run exceeded a time threshold."""

    name = "workflow_duration_exceeds"
    description = "Validates if a workflow run exceeded a time threshold"
    parameter_patterns = ["minutes"]
    event_types = ["workflow_run"]
    examples = [{"minutes": 3}, {"minutes": 5}]

    async def evaluate(self, context: Any) -> list[Violation]:
        """Evaluate workflow duration condition.

        Args:
            context: Dict with 'parameters' and 'event' keys.

        Returns:
            List of violations if workflow exceeded the duration threshold.
        """
        parameters = context.get("parameters", {})
        event = context.get("event", {})

        max_minutes = parameters.get("minutes", 3)

        workflow_run = event.get("workflow_run", {})
        if not workflow_run:
            logger.debug("WorkflowDurationCondition: No workflow_run data available")
            return []

        started_at = workflow_run.get("run_started_at")
        completed_at = workflow_run.get("completed_at") or workflow_run.get("updated_at")

        if not started_at or not completed_at:
            logger.debug("WorkflowDurationCondition: Missing timestamp data")
            return []

        try:
            from datetime import datetime

            start_time = datetime.fromisoformat(started_at.replace("Z", "+00:00"))
            end_time = datetime.fromisoformat(completed_at.replace("Z", "+00:00"))

            duration_seconds = (end_time - start_time).total_seconds()
            duration_minutes = duration_seconds / 60

            logger.debug(
                "WorkflowDurationCondition: Checking duration",
                duration_minutes=round(duration_minutes, 2),
                max_minutes=max_minutes,
            )

            if duration_minutes > max_minutes:
                workflow_name = workflow_run.get("name", "Unknown workflow")
                return [
                    Violation(
                        rule_description=self.description,
                        severity=Severity.MEDIUM,
                        message=f"Workflow '{workflow_name}' exceeded duration threshold ({duration_minutes:.1f} min > {max_minutes} min)",
                        details={
                            "workflow_name": workflow_name,
                            "duration_minutes": round(duration_minutes, 2),
                            "max_minutes": max_minutes,
                            "started_at": started_at,
                            "completed_at": completed_at,
                        },
                        how_to_fix="Optimize the workflow to run within the time limit or increase the threshold.",
                    )
                ]

        except Exception as e:
            logger.error("WorkflowDurationCondition: Error calculating duration", error=str(e))

        return []

    async def validate(self, parameters: dict[str, Any], event: dict[str, Any]) -> bool:
        """Legacy validation interface for backward compatibility.

        Note: This returns False (placeholder) as the original implementation did.
        Full implementation requires workflow_run event data.
        """
        # Placeholder logic - in production, this would check actual workflow duration
        context = {"parameters": parameters, "event": event}
        violations = await self.evaluate(context)
        return len(violations) == 0
