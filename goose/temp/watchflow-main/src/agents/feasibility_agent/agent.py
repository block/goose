"""
Rule Feasibility Agent implementation with error handling  and retry logic.
"""

import asyncio
import logging
import time
from typing import Any

from langgraph.graph import END, START, StateGraph
from langgraph.graph.state import CompiledStateGraph

from src.agents.base import AgentResult, BaseAgent
from src.agents.feasibility_agent.models import FeasibilityState
from src.agents.feasibility_agent.nodes import analyze_rule_feasibility, generate_yaml_config

logger = logging.getLogger(__name__)


class RuleFeasibilityAgent(BaseAgent):
    """
    LangGraph agent for checking if a user's natural language rule is feasible.

    Features:
    - Retry logic for structured output
    - Timeout handling
    - Enhanced error reporting
    - Performance metrics
    """

    def __init__(self, max_retries: int = 3, timeout: float = 30.0):
        super().__init__(max_retries=max_retries, agent_name="feasibility_agent")
        self.timeout = timeout
        logger.info(f"🔧 FeasibilityAgent initialized with max_retries={max_retries}, timeout={timeout}s")

    def _build_graph(self) -> CompiledStateGraph:
        """Build the LangGraph workflow for rule feasibility checking."""
        workflow = StateGraph(FeasibilityState)

        # Add nodes
        workflow.add_node("analyze_feasibility", analyze_rule_feasibility)
        workflow.add_node("generate_yaml", generate_yaml_config)

        # Add edges with conditional logic
        workflow.add_edge(START, "analyze_feasibility")

        # Conditional edge: only generate YAML if feasible
        workflow.add_conditional_edges(
            "analyze_feasibility",
            lambda state: "generate_yaml" if state.is_feasible else END,
            {"generate_yaml": "generate_yaml", END: END},
        )

        workflow.add_edge("generate_yaml", END)

        logger.info("🔧 FeasibilityAgent graph built with conditional structured output workflow")
        return workflow.compile()

    async def execute(self, **kwargs: Any) -> AgentResult:
        """
        Check if a rule description is feasible and return YAML or feedback.
        """
        rule_description = kwargs.get("rule_description")
        if not rule_description:
            return AgentResult(success=False, message="Missing 'rule_description' in arguments", data={})

        start_time = time.time()

        try:
            logger.info(f"🚀 Starting feasibility analysis for rule: {rule_description[:100]}...")

            # Prepare initial state
            initial_state = FeasibilityState(rule_description=rule_description)

            # Run the graph with timeout
            result = await self._execute_with_timeout(self.graph.ainvoke(initial_state), timeout=self.timeout)

            # Convert dict result back to FeasibilityState if needed
            if isinstance(result, dict):
                result = FeasibilityState(**result)

            execution_time = time.time() - start_time
            logger.info(f"✅ Feasibility analysis completed in {execution_time:.2f}s")
            logger.info(
                f"✅ Results: feasible={result.is_feasible}, type={result.rule_type}, confidence={result.confidence_score}"
            )

            # Convert to AgentResult with metadata
            return AgentResult(
                success=result.is_feasible,
                message=result.feedback,
                data={
                    "is_feasible": result.is_feasible,
                    "yaml_content": result.yaml_content,
                    "confidence_score": result.confidence_score,
                    "chosen_validators": result.chosen_validators,
                    "rule_type": result.rule_type,
                    "analysis_steps": result.analysis_steps,
                },
                metadata={
                    "execution_time_ms": execution_time * 1000,
                    "retry_count": 0,  # Will be updated by retry logic
                    "timeout_used": self.timeout,
                },
            )

        except TimeoutError:
            execution_time = time.time() - start_time
            logger.error(f"❌ Feasibility analysis timed out after {execution_time:.2f}s")
            return AgentResult(
                success=False,
                message=f"Feasibility analysis timed out after {self.timeout}s",
                data={},
                metadata={
                    "execution_time_ms": execution_time * 1000,
                    "timeout_used": self.timeout,
                    "error_type": "timeout",
                },
            )

        except Exception as e:
            execution_time = time.time() - start_time
            logger.error(f"❌ Feasibility analysis failed: {e}")
            return AgentResult(
                success=False,
                message=f"Feasibility analysis failed: {str(e)}",
                data={},
                metadata={"execution_time_ms": execution_time * 1000, "error_type": type(e).__name__},
            )

    async def execute_with_retry(self, rule_description: str) -> AgentResult:
        """
        Execute feasibility analysis with automatic retry on failure.
        """
        for attempt in range(self.max_retries):
            try:
                result = await self.execute(rule_description=rule_description)
                if result.success:
                    result.metadata = result.metadata or {}
                    result.metadata["retry_count"] = attempt
                    return result
                else:
                    logger.warning(f"⚠️ Feasibility analysis failed on attempt {attempt + 1}")
                    if attempt == self.max_retries - 1:
                        return result

                    # Wait before retry
                    await asyncio.sleep(self.retry_delay * (2**attempt))

            except Exception as e:
                logger.error(f"❌ Exception on attempt {attempt + 1}: {e}")
                if attempt == self.max_retries - 1:
                    return AgentResult(
                        success=False,
                        message=f"All retry attempts failed: {str(e)}",
                        data={},
                        metadata={"retry_count": attempt + 1, "final_error": str(e)},
                    )

                await asyncio.sleep(self.retry_delay * (2**attempt))

        return AgentResult(
            success=False, message="All retry attempts failed", data={}, metadata={"retry_count": self.max_retries}
        )
