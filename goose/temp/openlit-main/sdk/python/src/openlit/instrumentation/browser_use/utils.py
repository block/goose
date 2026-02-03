"""
Utilities for Browser-Use instrumentation with proper OpenLIT semantic conventions.
"""

import json
import logging
from typing import Any, Dict, Optional
from openlit.__helpers import get_chat_model_cost
from openlit.semcov import SemanticConvention

logger = logging.getLogger(__name__)


class BrowserUseInstrumentationContext:
    """
    Context object to cache agent information and provide optimized attribute access.
    Uses __slots__ for memory efficiency.
    """

    __slots__ = (
        "instance",
        "args",
        "kwargs",
        "version",
        "environment",
        "application_name",
        "_cache",
    )

    def __init__(
        self,
        instance: Any,
        args: tuple,
        kwargs: dict,
        version: str,
        environment: str,
        application_name: str,
    ):
        self.instance = instance
        self.args = args
        self.kwargs = kwargs
        self.version = version
        self.environment = environment
        self.application_name = application_name

        # Cache for expensive operations with lazy loading
        self._cache = {}

    @property
    def agent_name(self) -> str:
        """Get agent name with caching."""
        if "agent_name" not in self._cache:
            self._cache["agent_name"] = self._extract_agent_name()
        return self._cache["agent_name"]

    def _extract_agent_name(self) -> str:
        """Extract agent name from instance."""
        try:
            # Check for agent name attribute
            if hasattr(self.instance, "name"):
                return str(self.instance.name)
            if hasattr(self.instance, "agent_name"):
                return str(self.instance.agent_name)
            return "browser_use"
        except (AttributeError, ValueError, TypeError):
            return "browser_use"

    @property
    def agent_description(self) -> str:
        """Get agent task description with caching."""
        if "agent_description" not in self._cache:
            self._cache["agent_description"] = self._extract_agent_description()
        return self._cache["agent_description"]

    def _extract_agent_description(self) -> str:
        """Extract agent task description from arguments or instance."""
        try:
            # Check args for task description (usually first argument in Agent constructor)
            if self.args and len(self.args) > 0:
                first_arg = self.args[0]
                if isinstance(first_arg, str):
                    return first_arg[:200]  # Limit length

            # Check kwargs for task-related parameters
            for key in ["task", "instruction", "description"]:
                if key in self.kwargs:
                    value = self.kwargs[key]
                    if isinstance(value, str):
                        return value[:200]

            # Check instance attributes
            if hasattr(self.instance, "task"):
                return str(self.instance.task)[:200]

            return "browser_automation_task"
        except (AttributeError, ValueError, TypeError):
            return "browser_automation_task"

    @property
    def model_name(self) -> str:
        """Get model name with caching."""
        if "model_name" not in self._cache:
            self._cache["model_name"] = self._extract_model_name()
        return self._cache["model_name"]

    def _extract_model_name(self) -> str:
        """Extract model name from instance."""
        try:
            # Check instance for llm attribute (Agent uses llm)
            if hasattr(self.instance, "llm"):
                llm = self.instance.llm
                if hasattr(llm, "model_name"):
                    return str(llm.model_name)
                if hasattr(llm, "model"):
                    return str(llm.model)
                if hasattr(llm, "name"):
                    return str(llm.name)

            # Check for model attribute directly
            if hasattr(self.instance, "model"):
                model = self.instance.model
                if isinstance(model, str):
                    return model
                if hasattr(model, "model_name"):
                    return str(model.model_name)

            return "unknown"
        except (AttributeError, ValueError, TypeError):
            return "unknown"

    @property
    def current_url(self) -> Optional[str]:
        """Get current browser URL."""
        if "current_url" not in self._cache:
            self._cache["current_url"] = self._extract_current_url()
        return self._cache["current_url"]

    def _extract_current_url(self) -> Optional[str]:
        """Extract current URL from browser session."""
        try:
            # Check for browser session
            if hasattr(self.instance, "browser_session"):
                browser_session = self.instance.browser_session
                if browser_session and hasattr(browser_session, "current_page"):
                    page = browser_session.current_page
                    if page and hasattr(page, "url"):
                        return str(page.url)

            # Check controller for page info
            if hasattr(self.instance, "page"):
                page = self.instance.page
                if hasattr(page, "url"):
                    return str(page.url)

            return None
        except (AttributeError, ValueError, TypeError):
            return None

    @property
    def step_count(self) -> Optional[int]:
        """Get current step count."""
        if "step_count" not in self._cache:
            self._cache["step_count"] = self._extract_step_count()
        return self._cache["step_count"]

    def _extract_step_count(self) -> Optional[int]:
        """Extract step count from instance."""
        try:
            if hasattr(self.instance, "step_count"):
                return self.instance.step_count
            if hasattr(self.instance, "current_step"):
                return self.instance.current_step
            return None
        except (AttributeError, ValueError, TypeError):
            return None

    @property
    def max_steps(self) -> Optional[int]:
        """Get max steps with caching."""
        if "max_steps" not in self._cache:
            self._cache["max_steps"] = self._extract_max_steps()
        return self._cache["max_steps"]

    def _extract_max_steps(self) -> Optional[int]:
        """Extract max steps from kwargs or instance."""
        try:
            # Check kwargs first (from run() method)
            if "max_steps" in self.kwargs:
                return int(self.kwargs["max_steps"])

            # Check instance attributes
            if hasattr(self.instance, "max_steps"):
                return self.instance.max_steps

            return None
        except (AttributeError, ValueError, TypeError):
            return None


def get_operation_name(gen_ai_endpoint: str) -> str:
    """Extract operation name from endpoint."""
    if "." in gen_ai_endpoint:
        return gen_ai_endpoint.split(".")[-1]
    return gen_ai_endpoint.replace("_", " ")


def create_span_name(operation_name: str, ctx: BrowserUseInstrumentationContext) -> str:
    """Create span name following OpenLIT pattern."""

    # Map operation types to span name patterns
    operation_patterns = {
        "run": f"agent {ctx.agent_name}",
        "multi_act": "agent multi_act",
        "act": "browser act",
    }

    if operation_name in operation_patterns:
        return operation_patterns[operation_name]

    if operation_name in ["step"]:
        # Individual step execution
        step_num = ctx.step_count
        return (
            f"agent step_{step_num}"
            if step_num is not None
            else f"agent {ctx.agent_name}_step"
        )

    if operation_name in ["pause", "resume", "stop"]:
        # Task management
        return f"agent {operation_name}"

    # Generic operations
    return f"agent {operation_name}"


def set_span_attributes(
    span: Any,
    operation_name: str,
    ctx: BrowserUseInstrumentationContext,
    additional_attrs: Optional[Dict[str, Any]] = None,
) -> None:
    """Set comprehensive span attributes using proper OpenLIT semantic conventions."""

    # Core framework attributes
    span.set_attribute(
        SemanticConvention.GEN_AI_SYSTEM, SemanticConvention.GEN_AI_SYSTEM_BROWSER_USE
    )
    span.set_attribute(SemanticConvention.GEN_AI_OPERATION, operation_name)

    # Environment and application attributes
    span.set_attribute(SemanticConvention.GEN_AI_ENVIRONMENT, ctx.environment)
    span.set_attribute(SemanticConvention.GEN_AI_APPLICATION_NAME, ctx.application_name)
    span.set_attribute(SemanticConvention.GEN_AI_SDK_VERSION, ctx.version)

    # Agent attributes using proper semcov
    span.set_attribute(SemanticConvention.GEN_AI_AGENT_NAME, ctx.agent_name)
    span.set_attribute(
        SemanticConvention.GEN_AI_AGENT_TYPE,
        SemanticConvention.GEN_AI_AGENT_TYPE_BROWSER,
    )

    # Agent description (task)
    agent_desc = ctx.agent_description
    if agent_desc != "browser_automation_task":
        span.set_attribute(SemanticConvention.GEN_AI_AGENT_DESCRIPTION, agent_desc)

    # Model information if available
    model_name = ctx.model_name
    if model_name != "unknown":
        span.set_attribute(SemanticConvention.GEN_AI_REQUEST_MODEL, model_name)

    # Step information
    step_count = ctx.step_count
    if step_count is not None:
        span.set_attribute(SemanticConvention.GEN_AI_AGENT_STEP_COUNT, step_count)

    max_steps = ctx.max_steps
    if max_steps is not None:
        span.set_attribute(SemanticConvention.GEN_AI_AGENT_MAX_STEPS, max_steps)

    # Current URL if available
    current_url = ctx.current_url
    if current_url:
        span.set_attribute(SemanticConvention.GEN_AI_AGENT_BROWSE_URL, current_url)

    # Operation type mapping
    operation_type_map = {
        "run": SemanticConvention.GEN_AI_OPERATION_TYPE_AGENT,
        "step": SemanticConvention.GEN_AI_OPERATION_TYPE_EXECUTE_AGENT_TASK,
        "multi_act": SemanticConvention.GEN_AI_OPERATION_TYPE_TOOL_COORDINATION,
        "act": SemanticConvention.GEN_AI_OPERATION_TYPE_EXECUTE_AGENT_TASK,
        "pause": SemanticConvention.GEN_AI_OPERATION_TYPE_FRAMEWORK,
        "resume": SemanticConvention.GEN_AI_OPERATION_TYPE_FRAMEWORK,
        "stop": SemanticConvention.GEN_AI_OPERATION_TYPE_FRAMEWORK,
    }

    if operation_name in operation_type_map:
        span.set_attribute(
            SemanticConvention.GEN_AI_OPERATION_TYPE, operation_type_map[operation_name]
        )

    # Set additional attributes if provided
    if additional_attrs:
        for key, value in additional_attrs.items():
            if value is not None:
                span.set_attribute(key, value)


def process_response(
    span: Any,
    response: Any,
    _ctx: BrowserUseInstrumentationContext,
    capture_message_content: bool = True,
) -> None:
    """Process and capture response data with proper semantic conventions."""

    try:
        # Handle AgentHistoryList (from agent.run())
        if hasattr(response, "history") and hasattr(response, "usage"):
            history_list = response

            # Capture usage summary if available
            if history_list.usage:
                usage = history_list.usage
                if hasattr(usage, "total_input_tokens") and usage.total_input_tokens:
                    span.set_attribute(
                        SemanticConvention.GEN_AI_USAGE_INPUT_TOKENS,
                        usage.total_input_tokens,
                    )
                if hasattr(usage, "total_output_tokens") and usage.total_output_tokens:
                    span.set_attribute(
                        SemanticConvention.GEN_AI_USAGE_OUTPUT_TOKENS,
                        usage.total_output_tokens,
                    )
                if hasattr(usage, "total_cost") and usage.total_cost:
                    span.set_attribute(
                        SemanticConvention.GEN_AI_USAGE_COST, usage.total_cost
                    )

            # Capture total duration
            if hasattr(history_list, "total_duration_seconds"):
                duration = history_list.total_duration_seconds()
                if duration:
                    span.set_attribute(
                        SemanticConvention.GEN_AI_AGENT_EXECUTION_TIME, duration
                    )

            # Capture step count
            step_count = len(history_list.history)
            span.set_attribute(SemanticConvention.GEN_AI_AGENT_STEP_COUNT, step_count)

        # Handle ActionResult (from controller.act())
        elif hasattr(response, "is_success") or hasattr(response, "error"):
            action_result = response

            # Capture success/failure
            if hasattr(action_result, "is_success"):
                span.set_attribute(
                    SemanticConvention.GEN_AI_ACTION_SUCCESS, action_result.is_success
                )

            # Capture error information
            if hasattr(action_result, "error") and action_result.error:
                span.set_attribute(SemanticConvention.ERROR_TYPE, "action_failed")

        # Generic response capture
        if capture_message_content:
            if isinstance(response, str):
                span.set_attribute(
                    SemanticConvention.GEN_AI_CONTENT_COMPLETION, response[:1000]
                )
            elif hasattr(response, "__dict__"):
                try:
                    content = json.dumps(response.__dict__, default=str)[:1000]
                    span.set_attribute(
                        SemanticConvention.GEN_AI_CONTENT_COMPLETION, content
                    )
                except (TypeError, ValueError):
                    pass

    except (AttributeError, ValueError, TypeError, KeyError) as e:
        logger.debug("Error processing response: %s", e)


def capture_agent_thoughts_and_state(span: Any, agent_output: Any) -> None:
    """Capture agent thoughts and state information."""

    try:
        if hasattr(agent_output, "thinking") and agent_output.thinking:
            span.set_attribute(
                SemanticConvention.GEN_AI_AGENT_THINKING,
                str(agent_output.thinking)[:500],
            )

        if hasattr(agent_output, "memory") and agent_output.memory:
            span.set_attribute(
                SemanticConvention.GEN_AI_AGENT_MEMORY, str(agent_output.memory)[:500]
            )

        if hasattr(agent_output, "next_goal") and agent_output.next_goal:
            span.set_attribute(
                SemanticConvention.GEN_AI_AGENT_NEXT_GOAL,
                str(agent_output.next_goal)[:200],
            )

        if (
            hasattr(agent_output, "evaluation_previous_goal")
            and agent_output.evaluation_previous_goal
        ):
            span.set_attribute(
                SemanticConvention.GEN_AI_AGENT_EVALUATION,
                str(agent_output.evaluation_previous_goal)[:200],
            )

    except (AttributeError, ValueError, TypeError) as e:
        logger.debug("Error capturing agent thoughts: %s", e)


def capture_token_and_cost_metrics(
    span: Any, response: Any, model_name: str, pricing_info: Dict[str, Any]
) -> None:
    """Capture token usage and cost metrics for business intelligence."""

    try:
        # Extract token usage if available
        input_tokens = 0
        output_tokens = 0

        if hasattr(response, "usage"):
            usage = response.usage
            if hasattr(usage, "input_tokens"):
                input_tokens = usage.input_tokens
            if hasattr(usage, "output_tokens"):
                output_tokens = usage.output_tokens
        elif isinstance(response, dict) and "usage" in response:
            usage = response["usage"]
            input_tokens = usage.get("input_tokens", 0)
            output_tokens = usage.get("output_tokens", 0)

        if input_tokens > 0:
            span.set_attribute(
                SemanticConvention.GEN_AI_USAGE_INPUT_TOKENS, input_tokens
            )

        if output_tokens > 0:
            span.set_attribute(
                SemanticConvention.GEN_AI_USAGE_OUTPUT_TOKENS, output_tokens
            )

        # Calculate cost if pricing info is available
        if pricing_info and model_name != "unknown":
            try:
                cost = get_chat_model_cost(
                    model_name, pricing_info, input_tokens, output_tokens
                )
                if cost > 0:
                    span.set_attribute(SemanticConvention.GEN_AI_USAGE_COST, cost)
            except (ValueError, TypeError, KeyError) as cost_error:
                logger.debug("Error calculating cost: %s", cost_error)

    except (AttributeError, ValueError, TypeError, KeyError) as e:
        logger.debug("Error capturing token and cost metrics: %s", e)
