import logging
import time
from typing import Any

from src.agents import get_agent
from src.core.models import Severity, Violation
from src.event_processors.base import BaseEventProcessor, ProcessingResult
from src.integrations.github.check_runs import CheckRunManager
from src.tasks.task_queue import Task

logger = logging.getLogger(__name__)


class PushProcessor(BaseEventProcessor):
    """Processor for push events using hybrid agentic rule evaluation."""

    def __init__(self) -> None:
        super().__init__()

        self.engine_agent = get_agent("engine")

        self.check_run_manager = CheckRunManager(self.github_client)

    def get_event_type(self) -> str:
        return "push"

    async def process(self, task: Task) -> ProcessingResult:
        """Process a push event using the agentic approach."""
        start_time = time.time()
        payload = task.payload
        ref = payload.get("ref", "")
        commits = payload.get("commits", [])

        logger.info("=" * 80)
        logger.info(f"🚀 Processing PUSH event for {task.repo_full_name}")
        logger.info(f"   Ref: {ref}")
        logger.info(f"   Commits: {len(commits)}")
        logger.info("=" * 80)

        event_data = {
            "push": {
                "ref": ref,
                "commits": commits,
                "head_commit": payload.get("head_commit", {}),
                "before": payload.get("before"),
                "after": payload.get("after"),
            },
            "triggering_user": {"login": payload.get("pusher", {}).get("name")},
            "repository": payload.get("repository", {}),
            "organization": payload.get("organization", {}),
            "event_id": payload.get("event_id"),
            "timestamp": payload.get("timestamp"),
        }

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

        if not rules:
            logger.info("No rules found for this repository")
            return ProcessingResult(
                success=True, violations=[], api_calls_made=1, processing_time_ms=int((time.time() - start_time) * 1000)
            )

        logger.info(f"📋 Loaded {len(rules)} rules for evaluation")

        formatted_rules = self._convert_rules_to_new_format(rules)

        result = await self.engine_agent.execute(event_type="push", event_data=event_data, rules=formatted_rules)

        raw_violations = result.data.get("violations", [])
        violations: list[Violation] = []

        for v in raw_violations:
            try:
                # Map raw fields to Violation model
                severity_str = v.get("severity", "medium").lower()
                try:
                    severity = Severity(severity_str)
                except ValueError:
                    severity = Severity.MEDIUM

                violation = Violation(
                    rule_description=v.get("rule", "Unknown Rule"),
                    rule_id=v.get("rule_id"),
                    severity=severity,
                    message=v.get("message", "No message provided"),
                    how_to_fix=v.get("suggestion"),
                    details=v,
                )
                violations.append(violation)
            except Exception as e:
                logger.error(f"Error converting violation: {e}")

        processing_time = int((time.time() - start_time) * 1000)

        api_calls = 1

        sha = payload.get("after")
        if not sha or sha == "0000000000000000000000000000000000000000":
            logger.warning("No valid commit SHA found, skipping check run")
        else:
            # Ensure installation_id is not None before passing to check_run_manager
            if task.installation_id is None:
                logger.warning("Missing installation_id for push event, cannot create check run")
            else:
                if violations:
                    await self.check_run_manager.create_check_run(
                        repo=task.repo_full_name,
                        sha=sha,
                        installation_id=task.installation_id,
                        violations=violations,
                    )
                    api_calls += 1
                else:
                    # Create passing check run if no violations (optional but good practice)
                    await self.check_run_manager.create_check_run(
                        repo=task.repo_full_name,
                        sha=sha,
                        installation_id=task.installation_id,
                        violations=[],
                        conclusion="success",
                    )
                    api_calls += 1

        logger.info("=" * 80)

        logger.info(f"🏁 PUSH processing completed in {processing_time}ms")
        logger.info(f"   Rules evaluated: {len(formatted_rules)}")
        logger.info(f"   Violations found: {len(violations)}")
        logger.info(f"   API calls made: {api_calls}")
        logger.info("=" * 80)

        return ProcessingResult(
            success=True, violations=violations, api_calls_made=api_calls, processing_time_ms=processing_time
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
        return {
            "event_type": "push",
            "repo_full_name": task.repo_full_name,
            "ref": task.payload.get("ref"),
            "pusher": task.payload.get("pusher", {}),
            "commits": task.payload.get("commits", []),
            "head_commit": task.payload.get("head_commit", {}),
            "before": task.payload.get("before"),
            "after": task.payload.get("after"),
            "forced": task.payload.get("forced", False),
            "deleted": task.payload.get("deleted", False),
            "created": task.payload.get("created", False),
        }

    async def prepare_api_data(self, task: Task) -> dict[str, Any]:
        """Prepare data from GitHub API calls."""
        return {}
