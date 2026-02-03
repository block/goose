import logging
import time
from typing import TYPE_CHECKING, Any

from src.agents import get_agent
from src.core.models import Acknowledgment, EventType, Violation
from src.event_processors.base import BaseEventProcessor, ProcessingResult
from src.integrations.github.check_runs import CheckRunManager
from src.rules.acknowledgment import extract_acknowledgment_reason
from src.tasks.task_queue import Task

if TYPE_CHECKING:
    from src.agents.acknowledgment_agent.agent import AcknowledgmentAgent

logger = logging.getLogger(__name__)

# Add at the top
acknowledged_prs: set[str] = set()


class ViolationAcknowledgmentProcessor(BaseEventProcessor):
    """Processor for violation acknowledgment events using intelligent agentic evaluation."""

    def __init__(self) -> None:
        # Call super class __init__ first
        super().__init__()

        # Create instance of hybrid RuleEngineAgent for rule evaluation
        self.engine_agent = get_agent("engine")
        # Create instance of intelligent AcknowledgmentAgent for acknowledgment evaluation

        self.acknowledgment_agent: AcknowledgmentAgent = get_agent("acknowledgment")  # type: ignore[assignment]
        self.check_run_manager = CheckRunManager(self.github_client)

    def get_event_type(self) -> str:
        return "violation_acknowledgment"

    async def process(self, task: Task) -> ProcessingResult:
        """Process violation acknowledgment with intelligent validation."""
        start_time = time.time()
        api_calls = 0

        try:
            event_data = task.payload
            repo = event_data.get("repository", {}).get("full_name")
            installation_id = event_data.get("installation", {}).get("id")

            # Extract PR information from the comment
            issue = event_data.get("issue", {})
            pr_number = issue.get("number")
            comment_body = event_data.get("comment", {}).get("body", "")
            commenter = event_data.get("comment", {}).get("user", {}).get("login")

            # Helper to get SHA efficiently without full PR fetch if possible,
            # but we need PR data anyway later.
            pr_data: dict[str, Any] = {}
            sha = ""

            logger.info("=" * 80)
            logger.info(f"🔍 Processing VIOLATION ACKNOWLEDGMENT for {repo}#{pr_number}")
            logger.info(f"    Commenter: {commenter}")
            logger.info(f"    Comment: {comment_body[:100]}...")
            logger.info("=" * 80)

            # Extract acknowledgment reason from comment
            acknowledgment_reason = self._extract_acknowledgment_reason(comment_body)

            if not acknowledgment_reason:
                logger.info("❌ No valid acknowledgment reason found in comment")

                # Post a helpful comment explaining what went wrong
                help_comment = (
                    "❌ **Acknowledgment Failed**\n\n"
                    "No valid acknowledgment reason was found in your comment.\n\n"
                    "**Valid formats:**\n"
                    '- `@watchflow ack "Your reason here"`\n'
                    '- `@watchflow acknowledge "Your reason here"`\n'
                    "- `@watchflow ack Your reason here` (without quotes)\n\n"
                    "**Example:**\n"
                    '`@watchflow ack "Urgent production fix for critical security vulnerability"`\n\n'
                    "Please provide a specific and justified reason for overriding the rule violations."
                )

                await self._post_comment(repo, pr_number, installation_id, help_comment)
                api_calls += 1

                return ProcessingResult(
                    success=True,
                    violations=[],
                    api_calls_made=api_calls,
                    processing_time_ms=int((time.time() - start_time) * 1000),
                )

            # Get installation token
            github_token = await self.github_client.get_installation_access_token(installation_id)
            api_calls += 1

            if not github_token:
                logger.error(f"❌ Failed to get installation token for {installation_id}")
                return ProcessingResult(
                    success=False,
                    violations=[],
                    api_calls_made=api_calls,
                    processing_time_ms=int((time.time() - start_time) * 1000),
                    error="Failed to get installation token",
                )

            # Get current PR data and violations
            pr_data_optional = await self.github_client.get_pull_request(repo, pr_number, installation_id)
            if not pr_data_optional:
                logger.error(f"❌ Failed to get PR data for {repo}#{pr_number}")
                return ProcessingResult(
                    success=False,
                    violations=[],
                    api_calls_made=api_calls,
                    processing_time_ms=int((time.time() - start_time) * 1000),
                    error="Failed to get PR data",
                )
            pr_data = pr_data_optional
            if pr_data:
                sha = pr_data.get("head", {}).get("sha")

            api_calls += 1

            # Get PR files for better analysis
            pr_files = await self.github_client.get_pull_request_files(repo, pr_number, installation_id)
            api_calls += 1

            # Get PR reviews for approval analysis
            pr_reviews = await self.github_client.get_pull_request_reviews(repo, pr_number, installation_id)
            api_calls += 1

            # Get current violations for this PR
            rules = await self.rule_provider.get_rules(repo, installation_id)

            # Filter pull request rules
            pr_rules = []
            for rule in rules:
                if EventType.PULL_REQUEST in rule.event_types:
                    pr_rules.append(rule)

            logger.info(f"📋 Found {len(pr_rules)} pull request rules out of {len(rules)} total rules")

            # ✅ Use the same format as PullRequestProcessor
            formatted_rules = self._convert_rules_to_new_format(pr_rules)

            # ✅ Prepare event data in the format expected by the agentic analysis
            enriched_event_data = {
                "pull_request_details": pr_data,
                "files": pr_files,
                "reviews": pr_reviews,
                "repository": {"full_name": repo},
                "installation": {"id": installation_id},
                "github_client": self.github_client,  # Pass GitHub client for validators
            }

            # Run rule analysis to get ALL violations (not just current ones)
            analysis_result = await self.engine_agent.execute(
                event_type="pull_request",  # ✅ Use pull_request since we're evaluating PR rules
                event_data=enriched_event_data,
                rules=formatted_rules,
            )

            # Extract violations from AgentResult
            all_violations: list[Violation] = []
            if analysis_result.data and "evaluation_result" in analysis_result.data:
                eval_result = analysis_result.data["evaluation_result"]
                if hasattr(eval_result, "violations"):
                    # Use objects directly
                    all_violations = list(eval_result.violations)

            logger.info(f"Found {len(all_violations)} total violations")
            for violation in all_violations:
                logger.info(f"    • {violation.message}")

            # Check if the analysis failed due to timeout or other issues
            if not analysis_result.data or "evaluation_result" not in analysis_result.data:
                logger.warning(f"⚠️ Rule analysis failed: {analysis_result.message}")
                await self._post_comment(
                    repo,
                    pr_number,
                    installation_id,
                    f"⚠️ **Acknowledgment Processing Error**\n\n"
                    f"The rule analysis failed: {analysis_result.message}\n\n"
                    f"Please try again or contact an administrator if the issue persists.",
                )
                api_calls += 1
                return ProcessingResult(
                    success=False,
                    violations=[],
                    api_calls_made=api_calls,
                    processing_time_ms=int((time.time() - start_time) * 1000),
                    error=f"Rule analysis failed: {analysis_result.message}",
                )

            if not all_violations:
                logger.info("✅ No violations found - acknowledgment not needed")
                await self._post_comment(
                    repo, pr_number, installation_id, "✅ No rule violations detected. Acknowledgment not needed."
                )
                api_calls += 1

                # Update check run to reflect no violations
                if sha:
                    await self.check_run_manager.create_check_run(
                        repo=repo, sha=sha, installation_id=installation_id, violations=[], conclusion="success"
                    )
                api_calls += 1

                return ProcessingResult(
                    success=True,
                    violations=[],
                    api_calls_made=api_calls,
                    processing_time_ms=int((time.time() - start_time) * 1000),
                )

            # Evaluate acknowledgment against ALL violations
            evaluation_result = await self._evaluate_acknowledgment(
                acknowledgment_reason=acknowledgment_reason,
                pr_data=pr_data,
                violations=all_violations,  # Use ALL violations, not just current ones
                commenter=commenter,
                rules=formatted_rules,  # Pass the formatted rules
            )

            if evaluation_result["valid"]:
                # Acknowledgment is valid - selectively approve violations and provide guidance
                acknowledgable_violations = evaluation_result["acknowledgable_violations"]
                require_fixes = evaluation_result["require_fixes"]

                await self._approve_violations_selectively(
                    repo=repo,
                    pr_number=pr_number,
                    acknowledgable_violations=acknowledgable_violations,
                    require_fixes=require_fixes,
                    reason=acknowledgment_reason,
                    commenter=commenter,
                    installation_id=installation_id,
                )
                api_calls += 1

                # Update check run to reflect post-acknowledgment state
                if sha:
                    acknowledgments = {}
                    for v in acknowledgable_violations:
                        key = v.rule_id or v.rule_description
                        acknowledgments[key] = Acknowledgment(
                            rule_id=key,
                            reason=acknowledgment_reason,
                            commenter=commenter,
                        )

                    await self.check_run_manager.create_acknowledgment_check_run(
                        repo=repo,
                        sha=sha,
                        installation_id=installation_id,
                        acknowledgable_violations=acknowledgable_violations,
                        violations=require_fixes,
                        acknowledgments=acknowledgments,
                    )
                api_calls += 1

                logger.info(f"✅ Acknowledgment accepted: {evaluation_result['reason']}")
            else:
                # Acknowledgment is invalid - reject it
                await self._reject_acknowledgment(
                    repo=repo,
                    pr_number=pr_number,
                    reason=evaluation_result["reason"],
                    commenter=commenter,
                    require_fixes=evaluation_result["require_fixes"],
                    installation_id=installation_id,
                )
                api_calls += 1

                logger.info(f"❌ Acknowledgment rejected: {evaluation_result['reason']}")

            processing_time = int((time.time() - start_time) * 1000)
            logger.info("=" * 80)
            logger.info(f"🏁 VIOLATION ACKNOWLEDGMENT processing completed in {processing_time}ms")
            logger.info(f"    Status: {'accepted' if evaluation_result['valid'] else 'rejected'}")
            logger.info("=" * 80)

            # Return typed objects, not model dumps
            final_violations = []
            if not evaluation_result["valid"]:
                final_violations = evaluation_result["require_fixes"]

            return ProcessingResult(
                success=True,
                violations=final_violations,
                api_calls_made=api_calls,
                processing_time_ms=processing_time,
            )

        except Exception as e:
            logger.error(f"❌ Error processing violation acknowledgment: {str(e)}")
            return ProcessingResult(
                success=False,
                violations=[],
                api_calls_made=api_calls,
                processing_time_ms=int((time.time() - start_time) * 1000),
                error=str(e),
            )

    def _convert_rules_to_new_format(self, rules: list[Any]) -> list[dict[str, Any]]:
        """Convert Rule objects to the new flat schema format - same as PullRequestProcessor."""
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

            formatted_rules.append(rule_dict)

        return formatted_rules

    def _extract_acknowledgment_reason(self, comment_body: str) -> str:
        """Extract acknowledgment reason from comment.

        Delegates to centralized acknowledgment module.
        """
        return extract_acknowledgment_reason(comment_body)

    async def _evaluate_acknowledgment(
        self,
        acknowledgment_reason: str,
        pr_data: dict[str, Any],
        violations: list[Violation],
        commenter: str,
        rules: list[dict[str, Any]],
    ) -> dict[str, Any]:
        """
        Use intelligent LLM-based evaluation to determine which violations can be acknowledged vs. require fixes.
        """
        try:
            logger.info("🧠 Using intelligent acknowledgment agent for evaluation")

            # Use the rules parameter that was passed in (already formatted)
            # Don't fetch rules again - use the ones passed from the calling method

            # Use the intelligent acknowledgment agent
            agent_result = await self.acknowledgment_agent.evaluate_acknowledgment(
                acknowledgment_reason=acknowledgment_reason,
                violations=[v.model_dump() for v in violations],
                pr_data=pr_data,
                commenter=commenter,
                rules=rules,
            )

            if not agent_result.success:
                logger.error(f"🧠 Acknowledgment agent failed: {agent_result.message}")
                return {
                    "valid": False,
                    "acknowledgable_violations": [],
                    "require_fixes": violations,  # All violations require fixes if agent fails
                    "reason": f"Acknowledgment evaluation failed: {agent_result.message}",
                    "confidence": 0.0,
                    "details": {"error": agent_result.data.get("error", "Unknown error")},
                }

            # Extract results from agent response
            evaluation_data = agent_result.data
            is_valid = evaluation_data.get("is_valid", False)
            reasoning = evaluation_data.get("reasoning", "No reasoning provided")
            acknowledgable_violations = evaluation_data.get("acknowledgable_violations", [])
            require_fixes = evaluation_data.get("require_fixes", [])
            confidence = evaluation_data.get("confidence", 0.5)
            recommendations = evaluation_data.get("recommendations", [])

            logger.info("🧠 Intelligent evaluation completed:")
            logger.info(f"    Valid: {is_valid}")
            logger.info(f"    Reasoning: {reasoning}")
            logger.info(f"    Acknowledged violations: {len(acknowledgable_violations)}")
            logger.info(f"    Require fixes: {len(require_fixes)}")
            logger.info(f"    Confidence: {confidence}")

            return {
                "valid": is_valid,
                "acknowledgable_violations": acknowledgable_violations,
                "require_fixes": require_fixes,
                "reason": reasoning,
                "confidence": confidence,
                "details": {
                    "recommendations": recommendations,
                    "evaluation_method": "intelligent_llm",
                    "acknowledgable_count": len(acknowledgable_violations),
                    "require_fixes_count": len(require_fixes),
                },
            }

        except Exception as e:
            logger.error(f"🧠 Error in intelligent acknowledgment evaluation: {e}")
            return {
                "valid": False,
                "acknowledgable_violations": [],
                "require_fixes": violations,  # All violations require fixes on error
                "reason": f"Intelligent evaluation failed: {str(e)}",
                "confidence": 0.0,
                "details": {"error": str(e)},
            }

    async def _approve_violations_selectively(
        self,
        repo: str,
        pr_number: int,
        acknowledgable_violations: list[Violation],
        require_fixes: list[Violation],
        reason: str,
        commenter: str,
        installation_id: int,
    ) -> None:
        """Selectively approve violations and provide guidance for those that require fixes."""
        comment_parts = []

        # Add acknowledgment section
        if acknowledgable_violations:
            comment_parts.append("✅ **Violations Acknowledged**")
            comment_parts.append(f"**Reason:** {reason}")
            comment_parts.append(f"**Acknowledged by:** {commenter}")
            comment_parts.append("")
            comment_parts.append("The following violations have been overridden:")

            for violation in acknowledgable_violations:
                message = violation.message
                comment_parts.append(f"• {message}")

            comment_parts.append("")

        # Add violations requiring fixes section
        if require_fixes:
            if acknowledgable_violations:
                comment_parts.append("---")
                comment_parts.append("")

            comment_parts.append("⚠️ **Violations Requiring Fixes**")
            comment_parts.append("The following violations cannot be acknowledged and must be addressed:")
            comment_parts.append("")

            for violation in require_fixes:
                rule_description = violation.rule_description or "Unknown Rule"
                message = violation.message
                how_to_fix = violation.how_to_fix or ""

                comment_parts.append(f"**{rule_description}**")
                comment_parts.append(f"• {message}")
                if how_to_fix:
                    comment_parts.append(f"• **How to fix:** {how_to_fix}")
                comment_parts.append("")

        comment_parts.append("*This acknowledgment was validated using intelligent analysis.*")

        # Post the comment
        await self._post_comment(
            repo=repo, pr_number=pr_number, installation_id=installation_id, comment="\n".join(comment_parts)
        )

    async def _reject_acknowledgment(
        self,
        repo: str,
        pr_number: int,
        reason: str,
        commenter: str,
        require_fixes: list[Violation],
        installation_id: int,
    ) -> None:
        """Reject acknowledgment and explain why, showing violations that still need resolution."""
        comment_parts = []

        # Add rejection section
        comment_parts.append("❌ **Acknowledgment Rejected**")
        comment_parts.append(f"**Reason:** {reason}")
        comment_parts.append(f"**Attempted by:** {commenter}")
        comment_parts.append("")
        comment_parts.append(
            "The acknowledgment request was not valid. Please provide a more specific and justified reason for overriding these rule violations."
        )
        comment_parts.append("")

        # Add violations requiring fixes section (same format as _approve_violations_selectively)
        if require_fixes:
            comment_parts.append("---")
            comment_parts.append("")
            comment_parts.append("⚠️ **Violations Requiring Fixes**")
            comment_parts.append("Since the acknowledgment was rejected, all rule violations must be addressed:")
            comment_parts.append("")

            for violation in require_fixes:
                rule_description = violation.rule_description or "Unknown Rule"
                message = violation.message
                how_to_fix = violation.how_to_fix or ""

                comment_parts.append(f"**{rule_description}**")
                comment_parts.append(f"• {message}")
                if how_to_fix:
                    comment_parts.append(f"• **How to fix:** {how_to_fix}")
                comment_parts.append("")

        comment_parts.append("*This acknowledgment was validated using intelligent analysis.*")

        # Post the comment
        await self._post_comment(
            repo=repo, pr_number=pr_number, installation_id=installation_id, comment="\n".join(comment_parts)
        )

    async def _post_comment(self, repo: str, pr_number: int, installation_id: int, comment: str) -> None:
        """Post a comment on the PR."""
        await self.github_client.create_issue_comment(
            repo=repo, issue_number=pr_number, comment=comment, installation_id=installation_id
        )

    # Required abstract methods
    async def prepare_webhook_data(self, task: Task) -> dict[str, Any]:
        """Prepare data from webhook payload."""
        return task.payload

    async def prepare_api_data(self, task: Task) -> dict[str, Any]:
        """Prepare data from GitHub API calls."""
        return {}

    def _get_rule_provider(self) -> Any:
        """Get the rule provider for this processor."""
        from src.rules.loaders.github_loader import github_rule_loader

        return github_rule_loader
