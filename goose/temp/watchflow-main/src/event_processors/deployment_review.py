import logging
import time
from typing import Any

from src.agents import get_agent
from src.event_processors.base import BaseEventProcessor, ProcessingResult
from src.tasks.task_queue import Task

logger = logging.getLogger(__name__)


class DeploymentReviewProcessor(BaseEventProcessor):
    """Processor for deployment review events using hybrid agentic rule evaluation."""

    def __init__(self) -> None:
        # Call super class __init__ first
        super().__init__()

        # Create instance of hybrid RuleEngineAgent
        self.engine_agent = get_agent("engine")

    def get_event_type(self) -> str:
        return "deployment_review"

    async def process(self, task: Task) -> ProcessingResult:
        start_time = time.time()
        payload = task.payload
        deployment_review = payload.get("deployment_review", {})

        logger.info("=" * 80)
        logger.info(f"🚀 Processing DEPLOYMENT REVIEW event for {task.repo_full_name}")
        logger.info(f"   State: {deployment_review.get('state')}")
        logger.info(f"   Environment: {deployment_review.get('environment')}")
        logger.info("=" * 80)

        # Prepare event_data for the agent
        event_data = {
            "deployment_review": deployment_review,
            "deployment": payload.get("deployment", {}),  # Add deployment data
            "triggering_user": {"login": deployment_review.get("user", {}).get("login")},
            "repository": payload.get("repository", {}),
            "organization": payload.get("organization", {}),
            "event_id": payload.get("event_id"),
            "timestamp": payload.get("timestamp"),
        }

        # Enrich with additional deployment data if available
        deployment = payload.get("deployment", {})
        if deployment:
            event_data.update(
                {
                    "deployment_environment": deployment.get("environment"),
                    "deployment_ref": deployment.get("ref"),
                    "deployment_sha": deployment.get("sha"),
                    "deployment_task": deployment.get("task"),
                    "deployment_payload": deployment.get("payload", {}),
                }
            )

        # Fetch rules
        if not task.installation_id:
            logger.error("No installation ID found in task")
            return ProcessingResult(
                success=False,
                violations=[],
                api_calls_made=0,
                processing_time_ms=int((time.time() - start_time) * 1000),
                error="No installation ID found",
            )
        rules_optional = await self.rule_provider.get_rules(task.repo_full_name, task.installation_id)
        rules = rules_optional if rules_optional is not None else []

        # Filter rules for deployment_review events
        deployment_review_rules = []
        for r in rules:
            # Handle both Rule objects and dictionaries
            if hasattr(r, "event_types"):
                # Rule object
                event_types = [et.value if hasattr(et, "value") else et for et in r.event_types]
            elif isinstance(r, dict):
                # dictionary
                event_types = r.get("event_types", [])
            else:
                logger.error(f"Rule is not a dict or object: {r} (type: {type(r)})")
                continue

            if "deployment_review" in event_types:
                deployment_review_rules.append(r)

        if not deployment_review_rules:
            logger.info("📋 No deployment_review rules found")
            return ProcessingResult(
                success=True, violations=[], api_calls_made=1, processing_time_ms=int((time.time() - start_time) * 1000)
            )

        logger.info(f"📋 Found {len(deployment_review_rules)} applicable rules for deployment_review")

        # Convert rules to the new format expected by the agent
        formatted_rules = DeploymentReviewProcessor._convert_rules_to_new_format(deployment_review_rules)

        # Run agentic analysis using the instance
        result = await self.engine_agent.execute(
            event_type="deployment_review",
            event_data=event_data,
            rules=formatted_rules,
        )

        violations = result.data.get("violations", [])

        logger.info("=" * 80)
        logger.info(f"🏁 DEPLOYMENT REVIEW processing completed in {int((time.time() - start_time) * 1000)}ms")
        logger.info(f"   Violations: {len(violations)}")
        logger.info("=" * 80)

        return ProcessingResult(
            success=(not violations),
            violations=violations,
            api_calls_made=1,
            processing_time_ms=int((time.time() - start_time) * 1000),
        )

    async def prepare_webhook_data(self, task: Task) -> dict[str, Any]:
        """Extract data available in webhook payload."""
        return task.payload

    async def prepare_api_data(self, task: Task) -> dict[str, Any]:
        """Fetch data not available in webhook."""
        return {}

    @staticmethod
    def _convert_rules_to_new_format(rules: list[Any]) -> list[dict[str, Any]]:
        """Convert Rule objects to the new flat schema format."""
        formatted_rules = []

        for rule in rules:
            # Convert Rule object to dict format
            rule_dict = {
                "description": rule.description,
                "enabled": rule.enabled,
                "severity": rule.severity.value if hasattr(rule.severity, "value") else rule.severity,
                "event_types": [et.value if hasattr(et, "value") else et for et in rule.event_types],
                "parameters": rule.parameters if hasattr(rule, "parameters") else {},
            }

            # If no parameters field, try to extract from conditions (backward compatibility)
            if not rule_dict["parameters"] and hasattr(rule, "conditions"):
                for condition in rule.conditions:
                    rule_dict["parameters"].update(condition.parameters)

            formatted_rules.append(rule_dict)

        return formatted_rules

    @staticmethod
    def _format_violations_for_comment(violations: list[dict[str, Any]]) -> str:
        lines = []
        for v in violations:
            emoji = "⚠️" if v.get("severity") == "low" else "🚨"
            lines.append(
                f"{emoji} **Rule Violated:** `{v.get('rule_id', 'N/A')}`\n"
                f"**Severity:** {v.get('severity', 'high').capitalize()}\n"
                f"**Message:** {v.get('message', '')}\n"
                f"**How to fix:** {v.get('suggestion', 'See documentation.')}\n"
            )
        return "\n---\n".join(lines)
