"""
Rule Engine Agent with hybrid validation strategy.

Focuses on rule descriptions and parameters, using fast validators with LLM reasoning as fallback.
"""

import logging
import time
from typing import Any

from langgraph.graph import END, START, StateGraph

from src.agents.base import AgentResult, BaseAgent
from src.agents.engine_agent.models import (
    EngineRequest,
    EngineState,
    RuleDescription,
    RuleEvaluationResult,
    RuleViolation,
    ValidationStrategy,
    ValidatorDescription,
)
from src.agents.engine_agent.nodes import (
    analyze_rule_descriptions,
    execute_llm_fallback,
    execute_validator_evaluation,
    select_validation_strategy,
    validate_violations,
)
from src.rules.registry import AVAILABLE_CONDITIONS

logger = logging.getLogger(__name__)


class RuleEngineAgent(BaseAgent):
    """
    Hybrid rule engine that prioritizes fast validators with LLM reasoning as fallback.

    Strategy:
    1. Analyze rule descriptions and parameters
    2. Use LLM to select appropriate validation strategy based on available validators
    3. Execute fast validators for common rules (PR approvals, title patterns, etc.)
    4. Use LLM reasoning for complex/custom rules or as fallback
    5. This provides 80% speed/cost savings while maintaining 100% flexibility
    """

    def __init__(self, max_retries: int = 3, timeout: float = 300.0):
        super().__init__(max_retries=max_retries, agent_name="engine_agent")
        self.timeout = timeout

        logger.info("🔧 Rule Engine agent initializing...")
        logger.info(f"🔧 Available validators: {len(AVAILABLE_CONDITIONS)}")
        logger.info("🔧 Validation strategy: Hybrid (validators + LLM fallback)")

    def _build_graph(self) -> Any:
        """Build the LangGraph workflow for hybrid rule evaluation."""
        workflow = StateGraph(EngineState)

        # Add nodes
        workflow.add_node("analyze_rule_descriptions", analyze_rule_descriptions)
        workflow.add_node("select_validation_strategy", select_validation_strategy)
        workflow.add_node("execute_validator_evaluation", execute_validator_evaluation)
        workflow.add_node("execute_llm_fallback", execute_llm_fallback)
        workflow.add_node("validate_violations", validate_violations)

        # Add edges
        workflow.add_edge(START, "analyze_rule_descriptions")
        workflow.add_edge("analyze_rule_descriptions", "select_validation_strategy")
        workflow.add_edge("select_validation_strategy", "execute_validator_evaluation")
        workflow.add_edge("execute_validator_evaluation", "execute_llm_fallback")
        workflow.add_edge("execute_llm_fallback", "validate_violations")
        workflow.add_edge("validate_violations", END)

        return workflow.compile()

    async def execute(self, **kwargs: Any) -> AgentResult:
        """
        Hybrid rule evaluation focusing on rule descriptions and parameters.
        Prioritizes fast validators with LLM reasoning as fallback.

        Args:
            **kwargs: Must match EngineRequest: event_type, event_data, rules.
        """
        # Strict typing validation via Pydantic using strict=False to allow type coercion if needed
        # but primarily to ensure structure.
        try:
            # If request object is passed directly (future proofing)
            if "request" in kwargs and isinstance(kwargs["request"], EngineRequest):
                request = kwargs["request"]
            else:
                # Validate kwargs against EngineRequest
                request = EngineRequest(**kwargs)
        except Exception as e:
            return AgentResult(success=False, message=f"Invalid arguments for EngineAgent: {e}", data={})

        start_time = time.time()

        try:
            logger.info(f"🔧 Rule Engine starting evaluation for {request.event_type} with {len(request.rules)} rules")

            # Convert rules to rule descriptions (without id/name dependency)
            rule_descriptions = self._convert_rules_to_descriptions(request.rules)

            # Get validator descriptions from the validators themselves
            available_validators = self._get_validator_descriptions()

            # Prepare initial state
            initial_state = EngineState(
                event_type=request.event_type,
                event_data=request.event_data,
                rules=request.rules,
                rule_descriptions=rule_descriptions,
                available_validators=available_validators,
                violations=[],
                evaluation_context={},
                analysis_steps=[],
                validator_usage={},
                llm_usage=0,
            )

            logger.info("🔧 Rule Engine initial state prepared")

            # Run the hybrid graph with timeout
            result = await self._execute_with_timeout(self.graph.ainvoke(initial_state), timeout=self.timeout)

            execution_time = time.time() - start_time
            logger.info(f"🔧 Rule Engine evaluation completed in {execution_time:.2f}s")

            # Extract violations from result
            violations = []
            if isinstance(result, dict):
                violations = result.get("violations", [])
            elif hasattr(result, "violations"):
                violations = result.violations

            logger.info(f"🔧 Rule Engine extracted {len(violations)} violations")

            # Convert violations to RuleViolation objects
            rule_violations = []
            for violation in violations:
                rule_violation = RuleViolation(
                    rule_description=violation.get("rule_description", "Unknown rule"),
                    rule_id=violation.get("rule_id"),
                    severity=violation.get("severity", "medium"),
                    message=violation.get("message", "Rule violation detected"),
                    details=violation.get("details", {}),
                    how_to_fix=violation.get("how_to_fix", ""),
                    docs_url=violation.get("docs_url", ""),
                    validation_strategy=violation.get("validation_strategy", ValidationStrategy.VALIDATOR),
                    execution_time_ms=violation.get("execution_time_ms", 0.0),
                )
                rule_violations.append(rule_violation)

            # Create evaluation result
            evaluation_result = RuleEvaluationResult(
                event_type=request.event_type,
                repo_full_name=request.event_data.get("repository", {}).get("full_name", "unknown"),
                violations=rule_violations,
                total_rules_evaluated=len(request.rules),
                rules_triggered=len(rule_violations),
                total_rules=len(request.rules),
                evaluation_time_ms=execution_time * 1000,
                validator_usage=result.validator_usage if hasattr(result, "validator_usage") else {},
                llm_usage=result.llm_usage if hasattr(result, "llm_usage") else 0,
            )

            logger.info("🔧 Rule Engine evaluation completed successfully")
            logger.info(f"🔧 Validator usage: {evaluation_result.validator_usage}")
            logger.info(f"🔧 LLM usage: {evaluation_result.llm_usage} calls")

            return AgentResult(
                success=len(violations) == 0,
                message=f"Hybrid evaluation completed: {len(violations)} violations found",
                data={"evaluation_result": evaluation_result},
                metadata={
                    "execution_time_ms": execution_time * 1000,
                    "validator_usage": evaluation_result.validator_usage,
                    "llm_usage": evaluation_result.llm_usage,
                    "validation_strategy": "hybrid",
                },
            )
        except Exception as e:
            execution_time = time.time() - start_time
            logger.error(f"🔧 Error in Rule Engine evaluation: {e}")
            return AgentResult(
                success=False,
                message=f"Rule Engine evaluation failed: {str(e)}",
                data={},
                metadata={"execution_time_ms": execution_time * 1000, "error_type": type(e).__name__},
            )

    def _convert_rules_to_descriptions(self, rules: list[Any]) -> list[RuleDescription]:
        """Convert rules to RuleDescription objects without id/name dependency."""
        rule_descriptions = []

        for rule in rules:
            # Handle Rule objects (preferred) or dicts (legacy/fallback)
            if hasattr(rule, "description"):
                # It's a Rule object (or similar)
                description = rule.description
                parameters = rule.parameters
                conditions = getattr(rule, "conditions", [])
                event_types = [et.value if hasattr(et, "value") else str(et) for et in rule.event_types]
                severity = str(rule.severity.value) if hasattr(rule.severity, "value") else str(rule.severity)
                rule_id = getattr(rule, "rule_id", None)
            else:
                # It's a dict
                description = (
                    rule.get("description")
                    or rule.get("name")
                    or rule.get("rule_description")
                    or "Rule with parameters"
                )
                parameters = rule.get("parameters", {})
                conditions = []  # Dicts don't have attached conditions
                event_types = rule.get("event_types", [])
                severity = rule.get("severity", "medium")
                rule_id = rule.get("rule_id")

            rule_description = RuleDescription(
                description=description,
                rule_id=rule_id,
                parameters=parameters,
                event_types=event_types,
                severity=severity,
                validation_strategy=ValidationStrategy.HYBRID,  # Will be determined by LLM or conditions
                validator_name=None,  # Will be selected by LLM
                fallback_to_llm=True,
                conditions=conditions,
            )

            rule_descriptions.append(rule_description)

        return rule_descriptions

    def _get_validator_descriptions(self) -> list[ValidatorDescription]:
        """Get validator descriptions from the validators themselves."""
        validator_descriptions = []

        for condition_cls in AVAILABLE_CONDITIONS:
            validator_desc = ValidatorDescription(
                name=condition_cls.name,
                description=condition_cls.description,
                parameter_patterns=condition_cls.parameter_patterns,
                event_types=condition_cls.event_types,
                examples=condition_cls.examples,
            )
            validator_descriptions.append(validator_desc)

        return validator_descriptions

    async def evaluate(
        self, event_type: str, rules: list[dict[str, Any]], event_data: dict[str, Any], github_token: str = ""
    ) -> dict[str, Any]:
        """
        Legacy method for backwards compatibility.
        """
        result = await self.execute(event_type=event_type, event_data=event_data, rules=rules)

        if result.success:
            return {"status": "success", "message": result.message, "violations": []}
        else:
            eval_result = result.data.get("evaluation_result")
            violations = [v.__dict__ for v in eval_result.violations] if eval_result else []

            return {
                "status": "violations_found" if violations else "success",
                "message": result.message,
                "violations": violations,
            }

    async def evaluate_pull_request(self, rules: list[Any], event_data: dict[str, Any]) -> dict[str, Any]:
        """Legacy method for backwards compatibility."""
        logger.warning("evaluate_pull_request is deprecated. Use evaluate() with event_type='pull_request'")
        return await self.evaluate("pull_request", rules, event_data, "")
