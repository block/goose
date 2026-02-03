import logging
import re
import time
from typing import Any

from src.agents import get_agent
from src.agents.base import AgentResult
from src.event_processors.base import BaseEventProcessor, ProcessingResult
from src.tasks.task_queue import Task

logger = logging.getLogger(__name__)


class RuleCreationProcessor(BaseEventProcessor):
    """Processor for rule creation commands via comments."""

    def __init__(self) -> None:
        # Call super class __init__ first
        super().__init__()

        # Create instance using new structure
        self.feasibility_agent = get_agent("feasibility")

    def get_event_type(self) -> str:
        return "rule_creation"

    async def process(self, task: Task) -> ProcessingResult:
        """Process a rule creation command."""
        start_time = time.time()

        try:
            logger.info("=" * 80)
            logger.info(f"🚀 Processing RULE CREATION command for {task.repo_full_name}")
            logger.info("=" * 80)

            # Extract the rule description from the comment
            rule_description = self._extract_rule_description(task)

            if not rule_description:
                return ProcessingResult(
                    success=False,
                    violations=[],
                    api_calls_made=0,
                    processing_time_ms=int((time.time() - start_time) * 1000),
                    error="No rule description found in comment",
                )

            logger.info(f"📝 Rule description: {rule_description}")

            # Use the feasibility agent to check if the rule is supported
            feasibility_result = await self.feasibility_agent.execute(rule_description=rule_description)

            processing_time = int((time.time() - start_time) * 1000)

            # Post the result back to the comment
            await self._post_result_to_comment(task, feasibility_result)

            # Summary
            logger.info("=" * 80)
            logger.info(f"🏁 Rule creation processing completed in {processing_time}ms")
            logger.info(f"   Feasible: {feasibility_result.success}")
            logger.info("   API calls made: 1")

            if feasibility_result.success:
                logger.info("✅ Rule is feasible - YAML provided")
            else:
                logger.info("❌ Rule is not feasible - feedback provided")

            logger.info("=" * 80)

            return ProcessingResult(success=True, violations=[], api_calls_made=1, processing_time_ms=processing_time)

        except Exception as e:
            logger.error(f"❌ Error processing rule creation: {e}")
            return ProcessingResult(
                success=False,
                violations=[],
                api_calls_made=0,
                processing_time_ms=int((time.time() - start_time) * 1000),
                error=str(e),
            )

    def _extract_rule_description(self, task: Task) -> str:
        """Extract the rule description from the comment."""
        comment_body = task.payload.get("comment", {}).get("body", "")

        # Remove the @watchflow command and extract the rest
        patterns = [
            r"@watchflow\s+(?:create|add|new)\s+rule\s*[:.]?\s*(.+)",
            r"@watchflow\s+(?:create|add|new)\s+rule\s*\n(.+)",
        ]

        for pattern in patterns:
            match = re.search(pattern, comment_body, re.IGNORECASE | re.DOTALL)
            if match:
                description = match.group(1).strip()
                return description

        return ""

    async def _post_result_to_comment(self, task: Task, feasibility_result: AgentResult) -> None:
        """Post the feasibility result as a reply to the original comment."""
        try:
            # Get issue/PR number from the webhook payload
            issue = task.payload.get("issue", {})
            issue_number = issue.get("number")

            if not issue_number or not task.installation_id:
                logger.warning("No issue number or installation_id found in webhook payload, skipping reply")
                return

            reply_body = self._format_feasibility_reply(feasibility_result)

            # Create a new comment on the same issue/PR - Use self.github_client
            result = await self.github_client.create_issue_comment(
                task.repo_full_name, issue_number, reply_body, task.installation_id
            )

            if result:
                logger.info(f"✅ Successfully posted feasibility reply to issue/PR #{issue_number}")
            else:
                logger.error(f"❌ Failed to post feasibility reply to issue/PR #{issue_number}")

        except Exception as e:
            logger.error(f"Error posting feasibility reply: {e}")

    def _format_feasibility_reply(self, feasibility_result: AgentResult) -> str:
        """Format the feasibility result as a comment reply."""
        if feasibility_result.success and feasibility_result.data:
            reply = "## ✅ Rule Creation Successful!\n\n"
            reply += "Your rule is supported by Watchflow. Here's the YAML configuration:\n\n"
            reply += "```yaml\n"
            reply += feasibility_result.data.get("yaml_content", "")
            reply += "\n```\n\n"
            reply += "**Next Steps:**\n"
            reply += "1. Copy the YAML above\n"
            reply += "2. Add it to your `.watchflow/rules.yaml` file\n"
            reply += "3. Commit and push the changes\n"
            reply += "4. The rule will be active immediately!\n\n"
        else:
            reply = "## ❌ Rule Not Supported\n\n"
            reply += "Sorry, but your requested rule is not currently supported by Watchflow.\n\n"
            reply += "**Feedback:**\n"
            reply += feasibility_result.message
            reply += "\n\n"
            reply += "**Supported Rule Types:**\n"
            reply += "- Time-based restrictions (no merges on weekends)\n"
            reply += "- Branch naming conventions\n"
            reply += "- PR title patterns\n"
            reply += "- Required labels\n"
            reply += "- File size limits\n"
            reply += "- Approval requirements\n"
            reply += "- Commit message patterns\n"
            reply += "- Force push restrictions\n"
            reply += "- Branch protection rules\n\n"
            reply += "Try rephrasing your rule using one of these supported types."

        reply += "---\n"
        reply += "*This response was automatically generated by [Watchflow](https://github.com/your-org/watchflow).*"

        return reply

    async def prepare_webhook_data(self, task: Task) -> dict[str, Any]:
        """Prepare data from webhook payload."""
        return {
            "event_type": "rule_creation",
            "repo_full_name": task.repo_full_name,
            "comment": task.payload.get("comment", {}),
            "issue": task.payload.get("issue", {}),
        }

    async def prepare_api_data(self, task: Task) -> dict[str, Any]:
        """Prepare data from GitHub API calls."""
        return {}
