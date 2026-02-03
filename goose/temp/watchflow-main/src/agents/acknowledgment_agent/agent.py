"""
Intelligent Acknowledgment Agent for evaluating violation acknowledgment requests.
"""

import logging
from typing import Any

from langchain_core.messages import HumanMessage, SystemMessage
from langgraph.graph import StateGraph

from src.agents.acknowledgment_agent.models import AcknowledgmentContext, AcknowledgmentEvaluation
from src.agents.acknowledgment_agent.prompts import create_evaluation_prompt, get_system_prompt
from src.agents.base import AgentResult, BaseAgent
from src.integrations.providers import get_chat_model

logger = logging.getLogger(__name__)


class AcknowledgmentAgent(BaseAgent):
    """
    Intelligent agent that evaluates acknowledgment requests based on rule descriptions and context.

    Instead of relying on hardcoded rule names, this agent:
    1. Analyzes rule descriptions to understand what each rule does
    2. Evaluates the acknowledgment reason against the rule's purpose
    3. Makes intelligent decisions about which violations can be acknowledged
    4. Provides detailed reasoning for its decisions
    """

    def __init__(self, max_retries: int = 3, timeout: float = 30.0):
        # Call super class __init__ first
        super().__init__(max_retries=max_retries, agent_name="acknowledgment_agent")
        self.timeout = timeout
        logger.info(f"🧠 Acknowledgment agent initialized with timeout: {timeout}s")

    def _build_graph(self) -> Any:
        """
        Build a simple LangGraph workflow for acknowledgment evaluation.
        Since this agent is primarily LLM-based, we create a minimal graph.
        """

        # Create a simple state graph
        workflow = StateGraph(AcknowledgmentContext)

        # Add a single node that does the evaluation
        workflow.add_node("evaluate_acknowledgment", self._evaluate_node)

        # Simple linear flow
        workflow.set_entry_point("evaluate_acknowledgment")
        workflow.set_finish_point("evaluate_acknowledgment")

        return workflow.compile()

    async def _evaluate_node(self, state: Any) -> AgentResult:
        """Node function for LangGraph workflow."""
        try:
            result = await self.evaluate_acknowledgment(
                acknowledgment_reason=state.acknowledgment_reason,
                violations=state.violations,
                pr_data=state.pr_data,
                commenter=state.commenter,
                rules=state.rules,
            )
            return result
        except Exception as e:
            logger.error(f"🧠 Error in evaluation node: {e}")
            return AgentResult(success=False, message=f"Evaluation failed: {str(e)}", data={"error": str(e)})

    @staticmethod
    def _find_violation_by_rule_description(rule_description: str, violations: list[dict[str, Any]]) -> dict[str, Any]:
        """Find a violation by rule description (fallback when LLM returns rule description instead of rule_id)."""
        for violation in violations:
            if violation.get("rule_description") == rule_description:
                return violation
        return {}

    async def evaluate_acknowledgment(
        self,
        acknowledgment_reason: str,
        violations: list[dict[str, Any]],
        pr_data: dict[str, Any],
        commenter: str,
        rules: list[dict[str, Any]],
    ) -> AgentResult:
        """
        Intelligently evaluate an acknowledgment request based on rule descriptions and context.
        """
        try:
            logger.info(f"🧠 Evaluating acknowledgment request from {commenter}")
            logger.info(f"🧠 Reason: {acknowledgment_reason}")
            logger.info(f"🧠 Violations to evaluate: {len(violations)}")

            # Validate inputs
            if not acknowledgment_reason or not violations:
                return AgentResult(
                    success=False,
                    message="Invalid acknowledgment request: missing reason or violations",
                    data={"error": "Missing required parameters"},
                )

            # Create comprehensive evaluation prompt using the prompts module
            evaluation_prompt = create_evaluation_prompt(acknowledgment_reason, violations, pr_data, commenter, rules)

            # Get LLM evaluation with structured output
            logger.info("🧠 Requesting LLM evaluation with structured output...")

            # Use the same pattern as other agents: direct get_chat_model call
            llm = get_chat_model(agent="acknowledgment_agent")
            structured_llm = llm.with_structured_output(AcknowledgmentEvaluation)

            messages = [SystemMessage(content=get_system_prompt()), HumanMessage(content=evaluation_prompt)]
            structured_result = await self._execute_with_timeout(structured_llm.ainvoke(messages), timeout=self.timeout)

            if not structured_result:
                logger.error("🧠 Empty LLM response received")
                return AgentResult(
                    success=False, message="Empty response from LLM", data={"error": "LLM returned empty response"}
                )

            logger.info("🧠 Successfully received structured LLM evaluation result")

            # Map LLM decisions back to original violations using rule_description
            acknowledgable_violations = []
            require_fixes = []

            # Create a mapping of rule_description to original violation
            violation_map = {v.get("rule_description"): v for v in violations}

            # Process acknowledgable violations
            for llm_violation in structured_result.acknowledgable_violations:
                rule_description = llm_violation.rule_description
                original_violation = None

                # Try to find by rule_description first
                if rule_description in violation_map:
                    original_violation = violation_map[rule_description]
                else:
                    # Fallback: try to find by rule_description
                    original_violation = self._find_violation_by_rule_description(rule_description, violations)
                    if original_violation:
                        logger.info(f"🧠 Found violation by rule description: '{rule_description}'")
                    else:
                        logger.warning(
                            f"🧠 LLM returned rule_description '{rule_description}' not found in original violations"
                        )

                if original_violation:
                    violation_copy = original_violation.copy()
                    # Add acknowledgment-specific fields
                    violation_copy.update(
                        {
                            "acknowledgment_reason": llm_violation.reason,
                            "risk_level": llm_violation.risk_level,
                            "conditions": llm_violation.conditions,
                        }
                    )
                    acknowledgable_violations.append(violation_copy)

            # Process violations requiring fixes
            for llm_violation in structured_result.require_fixes:
                rule_description = llm_violation.rule_description
                original_violation = None

                # Try to find by rule_description first
                if rule_description in violation_map:
                    original_violation = violation_map[rule_description]
                else:
                    # Fallback: try to find by rule_description
                    original_violation = self._find_violation_by_rule_description(rule_description, violations)
                    if original_violation:
                        logger.info(f"🧠 Found violation by rule description: '{rule_description}'")
                    else:
                        logger.warning(
                            f"🧠 LLM returned rule_description '{rule_description}' not found in original violations"
                        )

                if original_violation:
                    violation_copy = original_violation.copy()
                    # Add fix-specific fields
                    violation_copy.update({"fix_reason": llm_violation.reason, "priority": llm_violation.priority})
                    require_fixes.append(violation_copy)

            logger.info("🧠 Intelligent evaluation completed:")
            logger.info(f"    Valid: {structured_result.is_valid}")
            logger.info(f"    Reasoning: {structured_result.reasoning}")
            logger.info(f"    Acknowledged violations: {len(acknowledgable_violations)}")
            logger.info(f"    Require fixes: {len(require_fixes)}")
            logger.info(f"    Confidence: {structured_result.confidence}")

            return AgentResult(
                success=True,
                message=f"Acknowledgment evaluation completed: {structured_result.reasoning}",
                data={
                    "is_valid": structured_result.is_valid,
                    "reasoning": structured_result.reasoning,
                    "acknowledgable_violations": acknowledgable_violations,  # Original violations with acknowledgment context
                    "require_fixes": require_fixes,  # Original violations with fix context
                    "confidence": structured_result.confidence,
                    "recommendations": structured_result.recommendations,
                },
            )

        except Exception as e:
            logger.error(f"🧠 Error in acknowledgment evaluation: {e}")
            import traceback

            logger.error(f"🧠 Traceback: {traceback.format_exc()}")
            return AgentResult(
                success=False, message=f"Acknowledgment evaluation failed: {str(e)}", data={"error": str(e)}
            )

    async def execute(self, **kwargs: Any) -> AgentResult:
        """
        Execute the acknowledgment agent.
        """
        acknowledgment_reason = kwargs.get("acknowledgment_reason")
        violations = kwargs.get("violations")
        pr_data = kwargs.get("pr_data")
        commenter = kwargs.get("commenter")
        rules = kwargs.get("rules")

        if acknowledgment_reason and violations and pr_data and commenter and rules:
            return await self.evaluate_acknowledgment(
                acknowledgment_reason=acknowledgment_reason,
                violations=violations,
                pr_data=pr_data,
                commenter=commenter,
                rules=rules,
            )

        logger.warning("🧠 execute() method called on AcknowledgmentAgent with missing arguments")
        return AgentResult(
            success=False, message="AcknowledgmentAgent requires specific arguments for execute()", data={}
        )
