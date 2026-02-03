"""
Optimized Pydantic AI OpenTelemetry instrumentation utility functions
This version reduces code duplication and improves performance while maintaining all data.
"""

import logging
import json
from typing import Dict, Any, Optional, List, Tuple
from opentelemetry.sdk.resources import (
    SERVICE_NAME,
    TELEMETRY_SDK_NAME,
    DEPLOYMENT_ENVIRONMENT,
)
from opentelemetry.trace import Status, StatusCode, SpanKind
from opentelemetry import context as context_api
from openlit.__helpers import handle_exception
from openlit.semcov import SemanticConvention

# Try to import enhanced helpers for business intelligence
try:
    from openlit.__helpers import get_chat_model_cost

    ENHANCED_HELPERS_AVAILABLE = True
except ImportError:
    ENHANCED_HELPERS_AVAILABLE = False

# Initialize logger for logging potential issues and operations
logger = logging.getLogger(__name__)

# Constants for common node names to avoid hardcoding
INTERNAL_NODE_NAMES = {"tool_calls_node", "model_request_node", "user_prompt_node"}


class PydanticAIInstrumentationContext:
    """
    Context object to hold common instrumentation data and reduce repeated extraction.
    """

    def __init__(self, instance, args, kwargs, version, environment, application_name):
        self.instance = instance
        self.args = args
        self.kwargs = kwargs
        self.version = version
        self.environment = environment
        self.application_name = application_name

        # Pre-extract common data to avoid repeated parsing
        self._agent_name = None
        self._model_name = None
        self._server_info = None
        self._messages = None
        self._tools = None
        self._model_params = None

    @property
    def agent_name(self) -> str:
        """Get agent name with caching."""
        if self._agent_name is None:
            self._agent_name = getattr(self.instance, "name", None) or "pydantic_agent"
        return self._agent_name

    @property
    def model_name(self) -> str:
        """Get model name with caching."""
        if self._model_name is None:
            if hasattr(self.instance, "model") and hasattr(
                self.instance.model, "model_name"
            ):
                self._model_name = str(self.instance.model.model_name)
            else:
                self._model_name = "unknown"
        return self._model_name

    @property
    def server_info(self) -> Tuple[str, int]:
        """Get server address and port with caching."""
        if self._server_info is None:
            # Determine server based on model
            if "openai" in self.model_name.lower():
                self._server_info = ("api.openai.com", 443)
            else:
                self._server_info = ("127.0.0.1", 80)
        return self._server_info

    @property
    def messages(self) -> List[Dict]:
        """Get extracted messages with caching."""
        if self._messages is None:
            self._messages = self._extract_messages()
        return self._messages

    @property
    def tools(self) -> List:
        """Get extracted tools with caching."""
        if self._tools is None:
            self._tools = self._extract_tools()
        return self._tools

    @property
    def model_params(self) -> Dict[str, Any]:
        """Get model parameters with caching."""
        if self._model_params is None:
            self._model_params = self._extract_model_parameters()
        return self._model_params

    def _extract_messages(self) -> List[Dict]:
        """Extract messages from context."""
        messages = []
        try:
            # Extract user message from args
            if self.args and len(self.args) > 0:
                user_message = self.args[0]
                if isinstance(user_message, str):
                    messages.append({"role": "user", "content": user_message})

            # Extract system prompt if available
            if (
                hasattr(self.instance, "_system_prompts")
                and self.instance._system_prompts
            ):
                system_prompt = str(self.instance._system_prompts)
                if system_prompt:
                    messages.insert(0, {"role": "system", "content": system_prompt})

            # Extract additional context from kwargs
            if "message_history" in self.kwargs:
                history = self.kwargs["message_history"]
                if isinstance(history, list):
                    messages.extend(history)
        except Exception as e:
            logger.debug("Failed to extract messages: %s", e)

        return messages

    def _extract_tools(self) -> List:
        """Extract tool definitions from instance."""
        tools = []
        try:
            if hasattr(self.instance, "_tools") and self.instance._tools:
                tools = self.instance._tools
        except Exception as e:
            logger.debug("Failed to extract tools: %s", e)
        return tools

    def _extract_model_parameters(self) -> Dict[str, Any]:
        """Extract model parameters from instance."""
        parameters = {}
        try:
            if hasattr(self.instance, "model"):
                model = self.instance.model
                param_names = [
                    "temperature",
                    "top_p",
                    "max_tokens",
                    "frequency_penalty",
                    "presence_penalty",
                    "stop",
                    "seed",
                    "top_k",
                ]

                for param in param_names:
                    if hasattr(model, param):
                        value = getattr(model, param)
                        if value is not None:
                            parameters[param] = value
        except Exception as e:
            logger.debug("Failed to extract model parameters: %s", e)

        return parameters


def set_span_attributes(
    span,
    operation_name: str,
    ctx: PydanticAIInstrumentationContext,
    agent_name: Optional[str] = None,
    lifecycle_phase: Optional[str] = None,
    additional_attrs: Optional[Dict[str, Any]] = None,
):
    """
    Optimized function to set common OpenTelemetry span attributes.

    Args:
        span: OpenTelemetry span object
        operation_name: The operation name for the span
        ctx: PydanticAIInstrumentationContext with cached data
        agent_name: Optional agent name (uses ctx.agent_name if not provided)
        lifecycle_phase: Optional lifecycle phase
        additional_attrs: Optional additional attributes to set
    """

    # Set core attributes
    span.set_attribute(TELEMETRY_SDK_NAME, "openlit")
    span.set_attribute(SemanticConvention.GEN_AI_OPERATION, operation_name)
    span.set_attribute(
        SemanticConvention.GEN_AI_SYSTEM, SemanticConvention.GEN_AI_SYSTEM_PYDANTIC_AI
    )

    # Set agent name if meaningful
    final_agent_name = agent_name or ctx.agent_name
    if final_agent_name and final_agent_name not in INTERNAL_NODE_NAMES:
        span.set_attribute(SemanticConvention.GEN_AI_AGENT_NAME, final_agent_name)

    # Set server info
    server_address, server_port = ctx.server_info
    span.set_attribute(SemanticConvention.SERVER_ADDRESS, server_address)
    span.set_attribute(SemanticConvention.SERVER_PORT, server_port)

    # Set model info
    span.set_attribute(SemanticConvention.GEN_AI_REQUEST_MODEL, ctx.model_name)

    # Set environment attributes
    span.set_attribute(DEPLOYMENT_ENVIRONMENT, ctx.environment)
    span.set_attribute(SERVICE_NAME, ctx.application_name)
    span.set_attribute(SemanticConvention.GEN_AI_SDK_VERSION, ctx.version)

    # Set lifecycle phase if provided
    if lifecycle_phase:
        span.set_attribute(
            SemanticConvention.GEN_AI_AGENT_LIFECYCLE_PHASE, lifecycle_phase
        )

    # Set additional attributes
    if additional_attrs:
        for key, value in additional_attrs.items():
            span.set_attribute(key, value)


def add_message_tracking(span, messages: List[Dict], message_type: str = "input"):
    """
    Optimized message tracking function.
    """
    if not messages:
        return

    try:
        # Convert to standard format
        formatted_messages = []
        for message in messages:
            formatted_message = {
                "role": message.get("role", "user"),
                "content": message.get("content", ""),
            }
            if "tool_calls" in message:
                formatted_message["tool_calls"] = message["tool_calls"]
            formatted_messages.append(formatted_message)

        # Set message attributes
        if message_type == "input":
            span.set_attribute(
                SemanticConvention.GEN_AI_CONTENT_PROMPT, json.dumps(formatted_messages)
            )
        else:
            span.set_attribute(
                SemanticConvention.GEN_AI_OUTPUT_MESSAGES,
                json.dumps(formatted_messages),
            )

        # Add metadata
        if formatted_messages:
            span.set_attribute(
                SemanticConvention.GEN_AI_MESSAGE_ROLE,
                formatted_messages[0].get("role", "user"),
            )
            total_length = sum(
                len(str(msg.get("content", ""))) for msg in formatted_messages
            )
            span.set_attribute("gen_ai.message.total_length", total_length)

    except Exception as e:
        logger.debug("Failed to add message tracking: %s", e)


def add_tool_tracking(span, tools: List):
    """
    Optimized tool tracking function.
    """
    if not tools:
        return

    try:
        formatted_tools = []
        for tool in tools:
            if hasattr(tool, "name"):
                formatted_tool = {
                    "name": tool.name,
                    "description": getattr(tool, "description", ""),
                }
                if hasattr(tool, "json_schema"):
                    formatted_tool["schema"] = tool.json_schema
            else:
                formatted_tool = {
                    "name": tool.get("name", ""),
                    "description": tool.get("description", ""),
                }
                if "schema" in tool:
                    formatted_tool["schema"] = tool["schema"]
            formatted_tools.append(formatted_tool)

        span.set_attribute(
            SemanticConvention.GEN_AI_AGENT_TOOLS, json.dumps(formatted_tools)
        )

    except Exception as e:
        logger.debug("Failed to add tool tracking: %s", e)


def execute_with_error_handling(
    span, wrapped, args, kwargs, capture_completion: bool = False
):
    """
    Execute wrapped function with standardized error handling.
    """
    try:
        response = wrapped(*args, **kwargs)

        # Add completion content if requested
        if capture_completion and hasattr(response, "data"):
            span.set_attribute(
                SemanticConvention.GEN_AI_CONTENT_COMPLETION, str(response.data)
            )

        span.set_status(Status(StatusCode.OK))
        return response

    except Exception as e:
        handle_exception(span, e)
        logger.error("Error in instrumentation: %s", e)
        raise


# Context extraction utilities for internal nodes
def extract_context_info(args, kwargs) -> Dict[str, Any]:
    """
    Extract context information from internal node arguments.
    This reduces code duplication across node instrumentation functions.
    """
    info = {
        "model_info": "",
        "agent_name": "",
        "user_input": "",
        "tool_info": "",
        "tool_count": 0,
        "message_count": 0,
    }

    try:
        if args and len(args) > 0:
            context = args[0]

            # Extract model info
            if hasattr(context, "deps") and hasattr(context.deps, "model"):
                model = context.deps.model
                if hasattr(model, "model_name"):
                    info["model_info"] = str(model.model_name)

            # Extract agent name
            if hasattr(context, "deps") and hasattr(context.deps, "agent"):
                agent = context.deps.agent
                if hasattr(agent, "name") and agent.name:
                    info["agent_name"] = str(agent.name)
            elif hasattr(context, "agent") and hasattr(context.agent, "name"):
                info["agent_name"] = str(context.agent.name)

            # Extract user input
            if hasattr(context, "user_input"):
                info["user_input"] = str(context.user_input)[:50]

            # Extract tool information
            if hasattr(context, "tool_calls") and context.tool_calls:
                info["tool_count"] = len(context.tool_calls)
                if context.tool_calls:
                    info["tool_info"] = getattr(
                        context.tool_calls[0], "function", {}
                    ).get("name", "")

            # Extract message count
            if hasattr(context, "messages") and context.messages:
                info["message_count"] = len(context.messages)

    except Exception as e:
        logger.debug("Failed to extract context info: %s", e)

    return info


def add_business_intelligence_attributes(
    span, model_name: str, response, pricing_info, capture_message_content: bool
):
    """
    Optimized business intelligence attributes function.
    """
    try:
        # Extract usage information
        usage_info = {"input_tokens": 0, "output_tokens": 0, "total_tokens": 0}

        if hasattr(response, "usage"):
            usage_obj = response.usage
            usage_info["input_tokens"] = (
                getattr(usage_obj, "input_tokens", 0)
                or getattr(usage_obj, "request_tokens", 0)
                or getattr(usage_obj, "prompt_tokens", 0)
                or 0
            )
            usage_info["output_tokens"] = (
                getattr(usage_obj, "output_tokens", 0)
                or getattr(usage_obj, "response_tokens", 0)
                or getattr(usage_obj, "completion_tokens", 0)
                or 0
            )
            usage_info["total_tokens"] = (
                usage_info["input_tokens"] + usage_info["output_tokens"]
            )

        # Set usage attributes
        if usage_info["input_tokens"] > 0:
            span.set_attribute(
                SemanticConvention.GEN_AI_USAGE_INPUT_TOKENS, usage_info["input_tokens"]
            )
        if usage_info["output_tokens"] > 0:
            span.set_attribute(
                SemanticConvention.GEN_AI_USAGE_OUTPUT_TOKENS,
                usage_info["output_tokens"],
            )
        if usage_info["total_tokens"] > 0:
            span.set_attribute(
                SemanticConvention.GEN_AI_USAGE_TOTAL_TOKENS, usage_info["total_tokens"]
            )

        # Calculate cost
        if (
            ENHANCED_HELPERS_AVAILABLE
            and pricing_info
            and usage_info["input_tokens"] > 0
        ):
            try:
                cost = get_chat_model_cost(
                    model_name,
                    pricing_info,
                    usage_info["input_tokens"],
                    usage_info["output_tokens"],
                )
                if cost > 0:
                    span.set_attribute(SemanticConvention.GEN_AI_USAGE_COST, cost)
            except Exception as e:
                logger.debug("Failed to calculate cost: %s", e)

        # Add performance metrics
        if hasattr(response, "duration") and response.duration:
            span.set_attribute(
                SemanticConvention.GEN_AI_CLIENT_OPERATION_DURATION, response.duration
            )
            if usage_info["total_tokens"] > 0:
                tokens_per_second = usage_info["total_tokens"] / response.duration
                span.set_attribute(
                    SemanticConvention.GEN_AI_PERFORMANCE_TOKENS_PER_SECOND,
                    tokens_per_second,
                )

        # Enhanced content capture
        if capture_message_content and hasattr(response, "output") and response.output:
            span.set_attribute(
                SemanticConvention.GEN_AI_CONTENT_COMPLETION, str(response.output)
            )

    except Exception as e:
        logger.debug("Failed to add business intelligence attributes: %s", e)


def common_agent_run(
    wrapped,
    instance,
    args,
    kwargs,
    tracer,
    version,
    environment,
    application_name,
    capture_message_content,
    pricing_info=None,
):
    """
    Optimized agent run function using context caching and standardized patterns.
    """
    # Suppression check
    if context_api.get_value(context_api._SUPPRESS_INSTRUMENTATION_KEY):
        return wrapped(*args, **kwargs)

    # Create cached context
    ctx = PydanticAIInstrumentationContext(
        instance, args, kwargs, version, environment, application_name
    )

    # Determine span name
    operation_type = SemanticConvention.GEN_AI_OPERATION_TYPE_AGENT
    span_name = f"{operation_type} {ctx.agent_name}"

    with tracer.start_as_current_span(span_name, kind=SpanKind.CLIENT) as span:
        # Set common attributes
        set_span_attributes(
            span=span,
            operation_name=SemanticConvention.GEN_AI_OPERATION_TYPE_EXECUTE_AGENT_TASK,
            ctx=ctx,
            lifecycle_phase=SemanticConvention.GEN_AI_AGENT_LIFECYCLE_PHASE_EXECUTE,
            additional_attrs={
                SemanticConvention.GEN_AI_AGENT_DESCRIPTION: str(
                    getattr(instance, "_system_prompts", "")
                ),
                SemanticConvention.GEN_AI_RESPONSE_MODEL: ctx.model_name,
            },
        )

        # Add message tracking if enabled
        if capture_message_content and ctx.messages:
            add_message_tracking(span, ctx.messages, "input")

        # Add tool tracking if tools exist
        if ctx.tools:
            add_tool_tracking(span, ctx.tools)

        # Add model parameters if available
        if ctx.model_params:
            span.set_attribute(
                SemanticConvention.GEN_AI_REQUEST_PARAMETERS,
                json.dumps(ctx.model_params),
            )

        # Execute with error handling
        response = execute_with_error_handling(
            span, wrapped, args, kwargs, capture_completion=False
        )

        # Add business intelligence
        add_business_intelligence_attributes(
            span, ctx.model_name, response, pricing_info, capture_message_content
        )

        return response


async def common_agent_run_async(
    wrapped,
    instance,
    args,
    kwargs,
    tracer,
    version,
    environment,
    application_name,
    capture_message_content,
    pricing_info=None,
):
    """
    Optimized async agent run function using context caching and standardized patterns.
    """
    # Suppression check
    if context_api.get_value(context_api._SUPPRESS_INSTRUMENTATION_KEY):
        return await wrapped(*args, **kwargs)

    # Create cached context
    ctx = PydanticAIInstrumentationContext(
        instance, args, kwargs, version, environment, application_name
    )

    # Determine span name
    operation_type = SemanticConvention.GEN_AI_OPERATION_TYPE_EXECUTE_AGENT_TASK
    span_name = f"{operation_type} {ctx.agent_name}"

    with tracer.start_as_current_span(span_name, kind=SpanKind.CLIENT) as span:
        # Set common attributes
        set_span_attributes(
            span=span,
            operation_name=SemanticConvention.GEN_AI_OPERATION_TYPE_EXECUTE_AGENT_TASK,
            ctx=ctx,
            lifecycle_phase=SemanticConvention.GEN_AI_AGENT_LIFECYCLE_PHASE_EXECUTE,
            additional_attrs={
                SemanticConvention.GEN_AI_AGENT_DESCRIPTION: str(
                    getattr(instance, "_system_prompts", "")
                ),
                SemanticConvention.GEN_AI_RESPONSE_MODEL: ctx.model_name,
            },
        )

        # Add message tracking if enabled
        if capture_message_content and ctx.messages:
            add_message_tracking(span, ctx.messages, "input")

        # Add tool tracking if tools exist
        if ctx.tools:
            add_tool_tracking(span, ctx.tools)

        # Add model parameters if available
        if ctx.model_params:
            span.set_attribute(
                SemanticConvention.GEN_AI_REQUEST_PARAMETERS,
                json.dumps(ctx.model_params),
            )

        # Execute async function
        response = await wrapped(*args, **kwargs)

        # Add business intelligence
        add_business_intelligence_attributes(
            span, ctx.model_name, response, pricing_info, capture_message_content
        )

        span.set_status(Status(StatusCode.OK))
        return response


def common_agent_create(
    wrapped,
    instance,
    args,
    kwargs,
    tracer,
    version,
    environment,
    application_name,
    capture_message_content,
):
    """
    Optimized agent creation function using context caching and standardized patterns.
    """
    # Suppression check
    if context_api.get_value(context_api._SUPPRESS_INSTRUMENTATION_KEY):
        return wrapped(*args, **kwargs)

    # Create minimal context for agent creation
    agent_name = kwargs.get("name", "pydantic_agent")
    request_model = (
        args[0] if args else kwargs.get("model", "google-gla:gemini-1.5-flash")
    )

    # Extract model_name if request_model is a model object
    if hasattr(request_model, "model_name"):
        model_name_str = str(request_model.model_name)
    else:
        model_name_str = str(request_model)

    # Create a minimal context object for creation
    class CreateContext:
        """Minimal context for agent creation instrumentation."""

        def __init__(self):
            self.agent_name = agent_name
            self.model_name = model_name_str
            self.server_info = ("127.0.0.1", 80)
            self.environment = environment
            self.application_name = application_name
            self.version = version
            self.messages = []
            self.tools = kwargs.get("tools", [])
            self.model_params = {}

        def get_context_info(self):
            """Get context information for instrumentation."""
            return {
                "agent_name": self.agent_name,
                "model_name": self.model_name,
                "tools_count": len(self.tools),
            }

        def has_tools(self):
            """Check if agent has tools configured."""
            return len(self.tools) > 0

    ctx = CreateContext()

    with tracer.start_as_current_span(
        f"create_agent {agent_name}", kind=SpanKind.CLIENT
    ) as span:
        # Set common attributes
        set_span_attributes(
            span=span,
            operation_name=SemanticConvention.GEN_AI_OPERATION_TYPE_CREATE_AGENT,
            ctx=ctx,
            lifecycle_phase=SemanticConvention.GEN_AI_AGENT_LIFECYCLE_PHASE_CREATE,
            additional_attrs={
                SemanticConvention.GEN_AI_AGENT_DESCRIPTION: str(
                    kwargs.get("system_prompt", "")
                ),
                SemanticConvention.GEN_AI_RESPONSE_MODEL: model_name_str,
            },
        )

        # Add tools if any are provided during creation
        if ctx.tools:
            add_tool_tracking(span, ctx.tools)

        # Execute with error handling
        return execute_with_error_handling(
            span, wrapped, args, kwargs, capture_completion=False
        )


def common_graph_execution(
    wrapped,
    instance,
    args,
    kwargs,
    tracer,
    version,
    environment,
    application_name,
    capture_message_content,
):
    """
    Handle telemetry for Pydantic AI graph execution operations.
    This wraps the Agent.iter() method to track graph execution.
    """

    # CRITICAL: Suppression check to prevent double instrumentation
    if context_api.get_value(context_api._SUPPRESS_INSTRUMENTATION_KEY):
        return wrapped(*args, **kwargs)

    # Create cached context for agent-based operations
    ctx = PydanticAIInstrumentationContext(
        instance, args, kwargs, version, environment, application_name
    )

    operation_type = SemanticConvention.GEN_AI_OPERATION_TYPE_GRAPH_EXECUTION
    span_name = f"{operation_type} {ctx.agent_name}"

    with tracer.start_as_current_span(span_name, kind=SpanKind.CLIENT) as span:
        # Set common attributes
        set_span_attributes(
            span=span,
            operation_name=operation_type,
            ctx=ctx,
            lifecycle_phase=SemanticConvention.GEN_AI_AGENT_LIFECYCLE_PHASE_GRAPH_EXECUTION,
            additional_attrs={
                SemanticConvention.GEN_AI_AGENT_DESCRIPTION: str(
                    getattr(instance, "_system_prompts", "")
                ),
            },
        )

        # Add model parameters if available
        if ctx.model_params:
            span.set_attribute(
                SemanticConvention.GEN_AI_REQUEST_PARAMETERS,
                json.dumps(ctx.model_params),
            )

        # Execute with error handling
        return execute_with_error_handling(
            span, wrapped, args, kwargs, capture_completion=False
        )


def common_internal_node(
    wrapped,
    instance,
    args,
    kwargs,
    tracer,
    version,
    environment,
    application_name,
    capture_message_content,
    operation_type,
    lifecycle_phase,
    node_type="internal",
):
    """
    Optimized generic function for internal node instrumentation.
    This consolidates common logic for all internal node types.
    """
    # Suppression check
    if context_api.get_value(context_api._SUPPRESS_INSTRUMENTATION_KEY):
        return wrapped(*args, **kwargs)

    # Extract context info efficiently
    context_info = extract_context_info(args, kwargs)

    # Determine span name
    if context_info["model_info"]:
        span_name = f"{operation_type} {context_info['model_info']}"
    elif context_info["agent_name"]:
        span_name = f"{operation_type} {context_info['agent_name']}"
    elif context_info["tool_info"]:
        span_name = f"{operation_type} {context_info['tool_info']}"
    else:
        span_name = f"{operation_type} {node_type}"

    with tracer.start_as_current_span(span_name, kind=SpanKind.CLIENT) as span:
        # Set basic attributes
        span.set_attribute(SemanticConvention.GEN_AI_OPERATION, operation_type)
        span.set_attribute(
            SemanticConvention.GEN_AI_SYSTEM,
            SemanticConvention.GEN_AI_SYSTEM_PYDANTIC_AI,
        )
        span.set_attribute(SemanticConvention.GEN_AI_SDK_VERSION, version)
        span.set_attribute(
            SemanticConvention.GEN_AI_AGENT_LIFECYCLE_PHASE, lifecycle_phase
        )

        # Set server info
        if operation_type == SemanticConvention.GEN_AI_OPERATION_TYPE_MODEL_REQUEST:
            span.set_attribute(SemanticConvention.SERVER_ADDRESS, "api.openai.com")
            span.set_attribute(SemanticConvention.SERVER_PORT, 443)
        else:
            span.set_attribute(SemanticConvention.SERVER_ADDRESS, "127.0.0.1")
            span.set_attribute(SemanticConvention.SERVER_PORT, 80)

        # Set extracted context attributes
        if context_info["model_info"]:
            span.set_attribute(
                SemanticConvention.GEN_AI_REQUEST_MODEL, context_info["model_info"]
            )
        if context_info["agent_name"]:
            span.set_attribute(
                SemanticConvention.GEN_AI_AGENT_NAME, context_info["agent_name"]
            )
        if context_info["user_input"]:
            span.set_attribute(
                SemanticConvention.GEN_AI_CONTENT_PROMPT, context_info["user_input"]
            )
        if context_info["tool_info"]:
            span.set_attribute(
                SemanticConvention.GEN_AI_TOOL_NAME, context_info["tool_info"]
            )

        # Execute with error handling
        return execute_with_error_handling(
            span, wrapped, args, kwargs, capture_completion=False
        )


def common_user_prompt_processing(
    wrapped,
    instance,
    args,
    kwargs,
    tracer,
    version,
    environment,
    application_name,
    capture_message_content,
):
    """
    Optimized user prompt processing function using generic internal node handler.
    """
    return common_internal_node(
        wrapped,
        instance,
        args,
        kwargs,
        tracer,
        version,
        environment,
        application_name,
        capture_message_content,
        operation_type=SemanticConvention.GEN_AI_OPERATION_TYPE_USER_PROMPT_PROCESSING,
        lifecycle_phase=SemanticConvention.GEN_AI_AGENT_LIFECYCLE_PHASE_USER_PROMPT_PROCESSING,
        node_type="user_input",
    )


def common_model_request_processing(
    wrapped,
    instance,
    args,
    kwargs,
    tracer,
    version,
    environment,
    application_name,
    capture_message_content,
):
    """
    Optimized model request processing function using generic internal node handler.
    """
    return common_internal_node(
        wrapped,
        instance,
        args,
        kwargs,
        tracer,
        version,
        environment,
        application_name,
        capture_message_content,
        operation_type=SemanticConvention.GEN_AI_OPERATION_TYPE_MODEL_REQUEST,
        lifecycle_phase=SemanticConvention.GEN_AI_AGENT_LIFECYCLE_PHASE_MODEL_REQUEST,
        node_type="llm",
    )


def common_tool_calls_processing(
    wrapped,
    instance,
    args,
    kwargs,
    tracer,
    version,
    environment,
    application_name,
    capture_message_content,
):
    """
    Optimized tool calls processing function using generic internal node handler.
    """
    return common_internal_node(
        wrapped,
        instance,
        args,
        kwargs,
        tracer,
        version,
        environment,
        application_name,
        capture_message_content,
        operation_type=SemanticConvention.GEN_AI_OPERATION_TYPE_TOOLS,
        lifecycle_phase=SemanticConvention.GEN_AI_AGENT_LIFECYCLE_PHASE_TOOL_EXECUTION,
        node_type="tools",
    )
