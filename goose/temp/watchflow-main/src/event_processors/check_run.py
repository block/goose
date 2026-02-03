import logging
import time
from typing import Any

from src.agents import get_agent
from src.event_processors.base import BaseEventProcessor, ProcessingResult
from src.tasks.task_queue import Task

logger = logging.getLogger(__name__)


class CheckRunProcessor(BaseEventProcessor):
    """Processor for check run events using hybrid agentic rule evaluation."""

    def __init__(self) -> None:
        # Call super class __init__ first
        super().__init__()

        # Create instance of hybrid RuleEngineAgent
        self.engine_agent = get_agent("engine")

    def get_event_type(self) -> str:
        return "check_run"

    async def process(self, task: Task) -> ProcessingResult:
        start_time = time.time()
        payload = task.payload
        check_run = payload.get("check_run", {})

        # Ignore our own check runs to prevent infinite loops
        if "watchflow" in check_run.get("name", "").lower():
            logger.info("Ignoring Watchflow's own check run to prevent recursive loops.")
            return ProcessingResult(
                success=True, violations=[], api_calls_made=0, processing_time_ms=int((time.time() - start_time) * 1000)
            )

        logger.info("=" * 80)
        logger.info(f"🚀 Processing CHECK RUN event for {task.repo_full_name}")
        logger.info(f"   Name: {check_run.get('name')}")
        logger.info(f"   Status: {check_run.get('status')}")
        logger.info(f"   Conclusion: {check_run.get('conclusion')}")
        logger.info("=" * 80)

        # Prepare event_data for the agent
        event_data = {
            "check_run": check_run,
            "triggering_user": {"login": check_run.get("app", {}).get("owner", {}).get("login")},
            "repository": payload.get("repository", {}),
            "organization": payload.get("organization", {}),
            "event_id": payload.get("event_id"),
            "timestamp": payload.get("timestamp"),
        }

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

        # Convert rules to the new format expected by the agent
        formatted_rules = self._convert_rules_to_new_format(rules)

        # Run agentic analysis using the instance
        result = await self.engine_agent.execute(
            event_type="check_run",
            event_data=event_data,
            rules=formatted_rules,
        )

        violations = result.data.get("violations", [])

        logger.info("=" * 80)
        logger.info(f"🏁 CHECK RUN processing completed in {int((time.time() - start_time) * 1000)}ms")
        logger.info(f"   Violations: {len(violations)}")
        logger.info("=" * 80)

        return ProcessingResult(
            success=(not violations),
            violations=violations,
            api_calls_made=1,
            processing_time_ms=int((time.time() - start_time) * 1000),
        )

    def _convert_rules_to_new_format(self, rules: list[Any]) -> list[dict[str, Any]]:
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

    async def prepare_webhook_data(self, task: Task) -> dict[str, Any]:
        """Prepare data from webhook payload."""
        check_run = task.payload.get("check_run", {})
        return {
            "event_type": "check_run",
            "repo_full_name": task.repo_full_name,
            "action": task.payload.get("action"),
            "check_run": {
                "id": check_run.get("id"),
                "name": check_run.get("name"),
                "status": check_run.get("status"),
                "conclusion": check_run.get("conclusion"),
                "started_at": check_run.get("started_at"),
                "completed_at": check_run.get("completed_at"),
                "head_sha": check_run.get("head_sha"),
                "check_suite": check_run.get("check_suite", {}),
            },
        }

    async def prepare_api_data(self, task: Task) -> dict[str, Any]:
        """Prepare data from GitHub API calls."""
        return {}
