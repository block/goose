import asyncio
import contextlib
from datetime import UTC, datetime, timedelta
from typing import TYPE_CHECKING, Any

import structlog

from src.agents import get_agent

if TYPE_CHECKING:
    from src.agents.base import BaseAgent
from src.integrations.github import github_client

logger = structlog.get_logger(__name__)


class DeploymentScheduler:
    """Scheduler for re-evaluating time-based deployment rules."""

    def __init__(self) -> None:
        self.running = False
        self.pending_deployments: list[dict[str, Any]] = []
        self.scheduler_task: asyncio.Task[None] | None = None
        # Lazy-load engine agent to avoid API key validation at import time
        self._engine_agent: BaseAgent | None = None

    @property
    def engine_agent(self) -> Any:
        """Lazy-load the engine agent to avoid API key validation at import time."""
        if self._engine_agent is None:
            self._engine_agent = get_agent("engine")
        return self._engine_agent

    async def start(self) -> None:
        """Start the scheduler."""
        if self.running:
            logger.warning("Deployment scheduler is already running")
            return

        self.running = True
        self.scheduler_task = asyncio.create_task(self._scheduler_loop())
        logger.info("🕒 Deployment scheduler started - checking every 15 minutes")

    async def stop(self) -> None:
        """Stop the scheduler."""
        self.running = False
        if self.scheduler_task:
            self.scheduler_task.cancel()
            # SIM105: Use contextlib.suppress instead of try-except-pass
            with contextlib.suppress(asyncio.CancelledError):
                await self.scheduler_task
        logger.info("🛑 Deployment scheduler stopped")

    async def add_pending_deployment(self, deployment_data: dict[str, Any]) -> None:
        """
        Add a deployment to the pending list for future re-evaluation.

        Args:
            deployment_data: dictionary containing:
                - deployment_id: GitHub deployment ID
                - repo: Repository full name
                - installation_id: GitHub App installation ID
                - environment: Deployment environment
                - event_data: Original event data
                - rules: Rules that were evaluated
                - violations: Current violations
                - time_based_violations: Time-based violations
                - created_at: Timestamp when added
        """
        try:
            # Validate required fields
            required_fields = [
                "deployment_id",
                "repo",
                "installation_id",
                "environment",
                "event_data",
                "rules",
                "violations",
                "time_based_violations",
                "created_at",
            ]
            missing_fields = [field for field in required_fields if not deployment_data.get(field)]
            if missing_fields:
                logger.error(f"Missing required fields for deployment scheduler: {missing_fields}")
                return

            logger.info(f"⏰ Adding deployment {deployment_data['deployment_id']} to scheduler")
            logger.info(f"    Repo: {deployment_data['repo']}")
            logger.info(f"    Installation: {deployment_data['installation_id']}")
            logger.info(f"    Time-based violations: {len(deployment_data.get('time_based_violations', []))}")

            self.pending_deployments.append(deployment_data)
        except Exception as e:
            logger.error(f"Error adding deployment to scheduler: {e}")

    async def _scheduler_loop(self) -> None:
        """Main scheduler loop - runs every 15 minutes."""
        while self.running:
            try:
                await self._check_pending_deployments()
                # Wait 15 minutes (900 seconds)
                await asyncio.sleep(900)
            except asyncio.CancelledError:
                logger.info("Scheduler loop cancelled")
                break
            except Exception as e:
                logger.error(f"Scheduler error: {e}")
                # Wait 1 minute on error before retrying
                await asyncio.sleep(60)

    async def _check_pending_deployments(self) -> None:
        """Check and re-evaluate pending deployments."""
        if not self.pending_deployments:
            return

        current_time = datetime.now(UTC)
        logger.info(
            f"🔍 Checking {len(self.pending_deployments)} pending deployments at {current_time.strftime('%Y-%m-%d %H:%M:%S')} UTC"
        )

        deployments_to_remove = []

        for i, deployment in enumerate(self.pending_deployments):
            try:
                # Check if deployment is too old (remove after 7 days)
                created_at = deployment.get("created_at")
                if created_at:
                    if isinstance(created_at, int | float):
                        # Convert timestamp to datetime
                        created_at = datetime.fromtimestamp(created_at)
                    age = current_time - created_at
                    if age > timedelta(days=7):
                        logger.info(
                            f"⏰ Removing expired deployment for {deployment.get('repo')} (age: {age.days} days)"
                        )
                        deployments_to_remove.append(i)
                        continue
                else:
                    # If no created_at timestamp, remove the deployment as it's invalid
                    logger.warning(f"⏰ Removing deployment for {deployment.get('repo')} with no created_at timestamp")
                    deployments_to_remove.append(i)
                    continue

                # Update last checked time
                deployment["last_checked"] = current_time

                # Re-evaluate the deployment
                should_approve = await self._re_evaluate_deployment(deployment)

                if should_approve:
                    # Approve the deployment
                    await self._approve_deployment(deployment)
                    deployments_to_remove.append(i)
                else:
                    logger.info(f"⏳ Deployment for {deployment.get('repo')} still blocked by time-based rules")

            except Exception as e:
                logger.error(f"Error re-evaluating deployment {deployment.get('repo', 'unknown')}: {e}")

        # Remove processed deployments (in reverse order to maintain indices)
        for i in reversed(deployments_to_remove):
            removed = self.pending_deployments.pop(i)
            logger.info(f"✅ Removed deployment for {removed.get('repo')} from scheduler")

        if self.pending_deployments:
            logger.info(f"📋 {len(self.pending_deployments)} deployments still pending")

    async def _re_evaluate_deployment(self, deployment: dict[str, Any]) -> bool:
        """Re-evaluate a deployment against current time-based rules."""
        try:
            # Validate required fields
            required_fields = ["repo", "environment", "installation_id", "event_data", "rules"]
            missing_fields = [field for field in required_fields if not deployment.get(field)]
            if missing_fields:
                logger.error(f"Missing required fields for deployment re-evaluation: {missing_fields}")
                return False

            logger.info(
                f"🔄 Re-evaluating deployment for {deployment.get('repo')} environment {deployment.get('environment')}"
            )

            # Refresh the GitHub token (it might have expired)
            try:
                fresh_token = await github_client.get_installation_access_token(deployment["installation_id"])
                if not fresh_token:
                    logger.error(f"Failed to get fresh GitHub token for installation {deployment['installation_id']}")
                    return False
                deployment["github_token"] = fresh_token
            except Exception as e:
                logger.error(f"Failed to refresh GitHub token: {e}")
                return False

            # Convert rules to the format expected by the analysis agent
            formatted_rules = DeploymentScheduler._convert_rules_to_new_format(deployment["rules"])

            # Re-run rule analysis
            result = await self.engine_agent.execute(
                event_type="deployment",
                event_data=deployment["event_data"],
                rules=formatted_rules,
            )

            violations = result.data.get("violations", [])

            if not violations:
                logger.info("✅ No violations found - deployment can be approved")
                return True

            # Check if any violations are still time-based
            time_based_violations = []
            other_violations = []

            for violation in violations:
                rule_description = violation.get("rule_description", "").lower()
                message = violation.get("message", "").lower()

                # Check if this is a time-based violation
                if any(
                    keyword in rule_description + message
                    for keyword in [
                        "hour",
                        "day",
                        "weekend",
                        "time",
                        "monday",
                        "tuesday",
                        "wednesday",
                        "thursday",
                        "friday",
                        "saturday",
                        "sunday",
                    ]
                ):
                    time_based_violations.append(violation)
                else:
                    other_violations.append(violation)

            if other_violations:
                logger.info(f"❌ Deployment still has non-time-based violations: {len(other_violations)}")
                # Remove from scheduler since these won't resolve automatically
                return False

            if time_based_violations:
                logger.info(f"⏰ Deployment still blocked by {len(time_based_violations)} time-based violations")
                return False

            # No violations left
            logger.info("✅ All violations resolved - deployment can be approved")
            return True

        except Exception as e:
            logger.error(f"Error re-evaluating deployment: {e}")
            return False

    async def _approve_deployment(self, deployment: dict[str, Any]) -> None:
        """Approve a previously rejected deployment."""
        try:
            callback_url = deployment.get("callback_url")
            installation_id = deployment.get("installation_id")
            repo = deployment.get("repo", "unknown")
            environment = deployment.get("environment", "unknown")

            if not callback_url:
                logger.error(f"No callback URL found for deployment {repo}")
                return

            if not installation_id:
                logger.error(f"No installation ID found for deployment {repo}")
                return

            comment = (
                "✅ **Deployment Automatically Approved**\n\n"
                "Time-based restrictions have been lifted. The deployment can now proceed.\n\n"
                f"**Environment:** {environment}\n"
                f"**Approved at:** {datetime.now(UTC).strftime('%Y-%m-%d %H:%M:%S')} UTC\n\n"
                "The deployment will be automatically approved on GitHub."
            )

            # Approve the deployment protection rule
            await github_client.review_deployment_protection_rule(
                callback_url=callback_url,
                environment=environment,
                state="approved",
                comment=comment,
                installation_id=installation_id,
            )

            logger.info(f"✅ Deployment {deployment.get('deployment_id')} automatically approved for {repo}")

        except Exception as e:
            logger.error(f"Error approving deployment {deployment.get('deployment_id')}: {e}")

    def get_status(self) -> dict[str, Any]:
        """Get current scheduler status."""
        try:
            pending_deployments_status = []
            for d in self.pending_deployments:
                created_at = d.get("created_at")
                created_at_iso = None
                if created_at:
                    if isinstance(created_at, int | float):
                        created_at_iso = datetime.fromtimestamp(created_at).isoformat()
                    elif hasattr(created_at, "isoformat"):
                        created_at_iso = created_at.isoformat()
                    else:
                        created_at_iso = str(created_at)

                last_checked = d.get("last_checked")
                last_checked_iso = last_checked.isoformat() if last_checked else None

                pending_deployments_status.append(
                    {
                        "repo": d.get("repo"),
                        "environment": d.get("environment"),
                        "deployment_id": d.get("deployment_id"),
                        "created_at": created_at_iso,
                        "last_checked": last_checked_iso,
                        "violations_count": len(d.get("violations", [])),
                        "time_based_violations_count": len(d.get("time_based_violations", [])),
                    }
                )

            return {
                "running": self.running,
                "pending_count": len(self.pending_deployments),
                "pending_deployments": pending_deployments_status,
            }
        except Exception as e:
            logger.error(f"Error getting scheduler status: {e}")
            return {"running": self.running, "pending_count": len(self.pending_deployments), "error": str(e)}

    async def start_background_scheduler(self) -> None:
        """Start the background scheduler task."""
        if not self.running:
            await self.start()

    async def stop_background_scheduler(self) -> None:
        """Stop the background scheduler task."""
        if self.running:
            await self.stop()

    @staticmethod
    def _convert_rules_to_new_format(rules: list[dict[str, Any]]) -> list[dict[str, Any]]:
        """
        Convert old rule format to new format if needed.
        This is for backward compatibility.
        """
        if not rules:
            return []

        # Check if conversion is needed by inspecting the first rule
        first_rule = rules[0]
        if "rule_description" in first_rule and "event_types" not in first_rule:
            # This looks like the old format
            logger.info("Converting old rule format to new format")
            converted_rules = []
            for rule in rules:
                converted_rules.append(
                    {
                        "description": rule.get("rule_description", ""),
                        "severity": rule.get("severity", "medium"),
                        "event_types": rule.get("event_types", ["deployment"]),
                        "parameters": rule.get("parameters", {}),
                    }
                )
            return converted_rules

        # Already in new format
        return rules


# Global instance - lazy loaded to avoid API key validation at import time
deployment_scheduler = None


def get_deployment_scheduler() -> DeploymentScheduler:
    """Get the global deployment scheduler instance, creating it if needed."""
    global deployment_scheduler
    if deployment_scheduler is None:
        deployment_scheduler = DeploymentScheduler()
    return deployment_scheduler
