import logging
import time
from typing import Any

from src.agents import get_agent
from src.core.models import Violation
from src.event_processors.base import BaseEventProcessor, ProcessingResult
from src.event_processors.pull_request.enricher import PullRequestEnricher
from src.integrations.github.check_runs import CheckRunManager
from src.presentation import github_formatter
from src.rules.loaders.github_loader import RulesFileNotFoundError
from src.tasks.task_queue import Task

logger = logging.getLogger(__name__)


class PullRequestProcessor(BaseEventProcessor):
    """Processor for pull request events using agentic rule evaluation."""

    def __init__(self) -> None:
        super().__init__()
        self.engine_agent = get_agent("engine")
        self.enricher = PullRequestEnricher(self.github_client)
        self.check_run_manager = CheckRunManager(self.github_client)

    def get_event_type(self) -> str:
        return "pull_request"

    async def process(self, task: Task) -> ProcessingResult:
        """Process a pull request event using the agentic approach."""
        start_time = time.time()
        api_calls = 0

        # Extract common data for check runs
        repo_full_name = task.repo_full_name
        installation_id = task.installation_id
        pr_data = task.payload.get("pull_request", {})
        pr_number = pr_data.get("number")
        sha = pr_data.get("head", {}).get("sha")

        if not installation_id:
            logger.error("No installation ID found in task")
            return ProcessingResult(
                success=False,
                violations=[],
                api_calls_made=api_calls,
                processing_time_ms=int((time.time() - start_time) * 1000),
                error="No installation ID found",
            )

        try:
            logger.info("=" * 80)
            logger.info(f"🚀 Processing PR event for {repo_full_name}")
            logger.info(f"   Action: {task.payload.get('action')}")
            logger.info(f"   PR Number: {pr_number}")
            logger.info("=" * 80)

            github_token_optional = await self.github_client.get_installation_access_token(installation_id)
            if not github_token_optional:
                raise ValueError("Failed to get installation access token")
            github_token = github_token_optional

            # 1. Enrich event data
            event_data = await self.enricher.enrich_event_data(task, github_token)
            api_calls += 1

            # 2. Fetch rules
            try:
                rules_optional = await self.rule_provider.get_rules(repo_full_name, installation_id)
                rules = rules_optional if rules_optional is not None else []
                api_calls += 1
            except RulesFileNotFoundError as e:
                logger.warning(f"Rules file not found: {e}")
                if sha:
                    await self.check_run_manager.create_check_run(
                        repo=repo_full_name,
                        sha=sha,
                        installation_id=installation_id,
                        violations=[],
                        conclusion="neutral",
                        error="Rules not configured. Please create `.watchflow/rules.yaml` in your repository.",
                    )
                # Post welcome comment with instructions and link to watchflow.dev (installation_id as URL param)
                if pr_number and installation_id:
                    try:
                        welcome_comment = github_formatter.format_rules_not_configured_comment(
                            repo_full_name=repo_full_name,
                            installation_id=installation_id,
                        )
                        await self.github_client.create_pull_request_comment(
                            repo_full_name, pr_number, welcome_comment, installation_id
                        )
                    except Exception as comment_err:
                        logger.warning(f"Could not post rules-not-configured comment: {comment_err}")
                return ProcessingResult(
                    success=True,
                    violations=[],
                    api_calls_made=api_calls,
                    processing_time_ms=int((time.time() - start_time) * 1000),
                    error="Rules not configured",
                )

            # 3. Check for existing acknowledgments
            previous_acknowledgments = {}
            if pr_number:
                previous_acknowledgments = await self.enricher.fetch_acknowledgments(
                    repo_full_name, pr_number, installation_id
                )
                if previous_acknowledgments:
                    logger.info(f"📋 Found {len(previous_acknowledgments)} previous acknowledgments")

            # 4. Run engine-based rule evaluation (pass Rule objects so .conditions are preserved).
            # No conversion to flat schema: a previous _convert_rules_to_new_format helper had issues and was removed.
            result = await self.engine_agent.execute(event_type="pull_request", event_data=event_data, rules=rules)

            # 5. Extract and filter violations
            violations: list[Violation] = []
            if result.data and "evaluation_result" in result.data:
                eval_result = result.data["evaluation_result"]
                if hasattr(eval_result, "violations"):
                    violations = [Violation.model_validate(v) for v in eval_result.violations]

            original_violations = violations.copy()
            acknowledgable_violations = []
            require_acknowledgment_violations = []

            for violation in violations:
                # Match by rule_id so acknowledgment lookup matches parsed comments
                ack_key = violation.rule_id or violation.rule_description
                if ack_key in previous_acknowledgments:
                    acknowledgable_violations.append(violation)
                else:
                    require_acknowledgment_violations.append(violation)

            logger.info(
                f"📊 Violation breakdown: {len(acknowledgable_violations)} acknowledged, {len(require_acknowledgment_violations)} requiring fixes"
            )

            violations = require_acknowledgment_violations

            # 6. Report results to GitHub
            if sha:
                if previous_acknowledgments and original_violations:
                    await self.check_run_manager.create_acknowledgment_check_run(
                        repo=repo_full_name,
                        sha=sha,
                        installation_id=installation_id,
                        acknowledgable_violations=acknowledgable_violations,
                        violations=violations,
                        acknowledgments=previous_acknowledgments,
                    )
                else:
                    await self.check_run_manager.create_check_run(
                        repo=repo_full_name,
                        sha=sha,
                        installation_id=installation_id,
                        violations=violations,
                    )

            if violations:
                logger.info(f"🚨 Found {len(violations)} violations, posting to PR...")
                await self._post_violations_to_github(task, violations)
                api_calls += 1

            processing_time = int((time.time() - start_time) * 1000)
            logger.info("=" * 80)
            logger.info(f"🏁 PR processing completed in {processing_time}ms")
            logger.info("=" * 80)

            return ProcessingResult(
                success=(not violations),
                violations=violations,
                api_calls_made=api_calls,
                processing_time_ms=processing_time,
            )

        except Exception as e:
            logger.error(f"❌ Error processing PR event: {e}")
            if sha:
                await self.check_run_manager.create_check_run(
                    repo=repo_full_name,
                    sha=sha,
                    installation_id=installation_id,
                    violations=[],
                    conclusion="failure",
                    error=str(e),
                )
            return ProcessingResult(
                success=False,
                violations=[],
                api_calls_made=api_calls,
                processing_time_ms=int((time.time() - start_time) * 1000),
                error=str(e),
            )

    async def _post_violations_to_github(self, task: Task, violations: list[Violation]) -> None:
        """Post violations as comments on the pull request."""
        try:
            pr_number = task.payload.get("pull_request", {}).get("number")
            if not pr_number or not task.installation_id:
                return

            comment_body = github_formatter.format_violations_comment(violations)
            await self.github_client.create_pull_request_comment(
                task.repo_full_name, pr_number, comment_body, task.installation_id
            )
        except Exception as e:
            logger.error(f"Error posting violations to GitHub: {e}")

    async def prepare_webhook_data(self, task: Task) -> dict[str, Any]:
        """Extract data available in webhook payload."""
        return self.enricher.prepare_webhook_data(task)

    async def prepare_api_data(self, task: Task) -> dict[str, Any]:
        """Fetch data not available in webhook."""
        pr_number = task.payload.get("pull_request", {}).get("number")
        if not pr_number or not task.installation_id:
            return {}
        return await self.enricher.fetch_api_data(task.repo_full_name, pr_number, task.installation_id)
