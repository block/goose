"""
Module for monitoring Agno agent framework operations.
"""

import logging
import time
from opentelemetry.trace import Status, StatusCode, SpanKind
from opentelemetry import context as context_api
from openlit.__helpers import (
    handle_exception,
)
from openlit.instrumentation.agno.utils import (
    process_agent_request,
    process_tool_request,
    process_memory_request,
    process_reasoning_request,
    process_vectordb_request,
    process_knowledge_request,
    process_workflow_request,
    process_team_request,
)
from openlit.semcov import SemanticConvention

# Initialize logger for Agno monitoring
logger = logging.getLogger(__name__)


def agent_run_wrap(
    gen_ai_endpoint,
    version,
    environment,
    application_name,
    tracer,
    pricing_info,
    capture_message_content,
    metrics,
    disable_metrics,
):
    """
    Wrap Agno Agent.run method with comprehensive instrumentation using process functions only.
    """

    def wrapper(wrapped, instance, args, kwargs):
        # CRITICAL: Suppression check to prevent recursive instrumentation
        if context_api.get_value(context_api._SUPPRESS_INSTRUMENTATION_KEY):
            return wrapped(*args, **kwargs)

        # Extract agent name for span naming with fallback to agent_id
        agent_name = (
            getattr(instance, "name", None)
            or getattr(instance, "agent_id", None)
            or "default_agent"
        )
        span_name = f"agent {agent_name}"

        with tracer.start_as_current_span(span_name, kind=SpanKind.CLIENT) as span:
            start_time = time.time()
            response = wrapped(*args, **kwargs)

            try:
                # Process request using utils function with ALL attributes from semcov
                process_agent_request(
                    span,
                    instance,
                    args,
                    kwargs,
                    response,
                    start_time,
                    pricing_info,
                    environment,
                    application_name,
                    metrics,
                    capture_message_content,
                    disable_metrics,
                    version,
                    SemanticConvention.GEN_AI_OPERATION_TYPE_AGENT,
                )

                # Mark span as successful
                span.set_status(Status(StatusCode.OK))

            except Exception as e:
                # Handle instrumentation exceptions - log but don't raise
                handle_exception(span, e)
                logger.error("Error in agent.run trace creation: %s", e)

            return response

    return wrapper


def agent_run_tool_wrap(
    gen_ai_endpoint,
    version,
    environment,
    application_name,
    tracer,
    pricing_info,
    capture_message_content,
    metrics,
    disable_metrics,
):
    """
    Wrap Agno Agent._run_tool method - the critical bridge between agent and tool execution.
    This method runs within the agent context and coordinates tool execution.
    """

    def wrapper(wrapped, instance, args, kwargs):
        # CRITICAL: Suppression check to prevent recursive instrumentation
        if context_api.get_value(context_api._SUPPRESS_INSTRUMENTATION_KEY):
            return wrapped(*args, **kwargs)

        # Extract tool information for span naming
        tool = args[1] if len(args) > 1 else None  # tool: ToolExecution
        tool_name = "unknown_tool"
        if tool and hasattr(tool, "name"):
            tool_name = tool.name
        elif tool and hasattr(tool, "function") and hasattr(tool.function, "name"):
            tool_name = tool.function.name

        span_name = f"agent run tool {tool_name}"

        with tracer.start_as_current_span(span_name, kind=SpanKind.INTERNAL) as span:
            start_time = time.time()
            result = wrapped(*args, **kwargs)

            try:
                # Process request using utils function with ALL attributes from semcov
                process_tool_request(
                    span,
                    instance,
                    args,
                    kwargs,
                    result,
                    start_time,
                    pricing_info,
                    environment,
                    application_name,
                    metrics,
                    capture_message_content,
                    disable_metrics,
                    version,
                    tool_name,
                )

                span.set_status(Status(StatusCode.OK))

            except Exception as e:
                handle_exception(span, e)
                logger.error("Error in agent._run_tool trace creation: %s", e)

            return result

    return wrapper


def agent_continue_run_wrap(
    gen_ai_endpoint,
    version,
    environment,
    application_name,
    tracer,
    pricing_info,
    capture_message_content,
    metrics,
    disable_metrics,
):
    """
    Wrap Agno Agent.continue_run method.
    """

    def wrapper(wrapped, instance, args, kwargs):
        # CRITICAL: Suppression check to prevent recursive instrumentation
        if context_api.get_value(context_api._SUPPRESS_INSTRUMENTATION_KEY):
            return wrapped(*args, **kwargs)

        agent_name = getattr(instance, "name", "unknown")
        span_name = f"continue {agent_name}"

        with tracer.start_as_current_span(span_name, kind=SpanKind.CLIENT) as span:
            start_time = time.time()
            result = wrapped(*args, **kwargs)

            try:
                # Process request using utils function with ALL attributes from semcov
                process_agent_request(
                    span,
                    instance,
                    args,
                    kwargs,
                    result,
                    start_time,
                    pricing_info,
                    environment,
                    application_name,
                    metrics,
                    capture_message_content,
                    disable_metrics,
                    version,
                    SemanticConvention.GEN_AI_OPERATION_TYPE_AGENT,
                )

                span.set_status(Status(StatusCode.OK))

            except Exception as e:
                handle_exception(span, e)
                logger.error("Error in agent.continue_run trace creation: %s", e)

            return result

    return wrapper


def agent_add_tool_wrap(
    gen_ai_endpoint,
    version,
    environment,
    application_name,
    tracer,
    pricing_info,
    capture_message_content,
    metrics,
    disable_metrics,
):
    """
    Wrap Agno Agent.add_tool method for detailed tracing.
    """

    def wrapper(wrapped, instance, args, kwargs):
        # CRITICAL: Suppression check to prevent recursive instrumentation
        if context_api.get_value(context_api._SUPPRESS_INSTRUMENTATION_KEY):
            return wrapped(*args, **kwargs)

        agent_name = getattr(instance, "name", "unknown")
        span_name = f"agent {agent_name}"

        with tracer.start_as_current_span(span_name, kind=SpanKind.INTERNAL) as span:
            start_time = time.time()
            result = wrapped(*args, **kwargs)

            try:
                # Process request using utils function with ALL attributes from semcov
                process_tool_request(
                    span,
                    instance,
                    args,
                    kwargs,
                    result,
                    start_time,
                    pricing_info,
                    environment,
                    application_name,
                    metrics,
                    capture_message_content,
                    disable_metrics,
                    version,
                    SemanticConvention.GEN_AI_OPERATION_TYPE_TOOLS,
                )

                span.set_status(Status(StatusCode.OK))

            except Exception as e:
                handle_exception(span, e)
                logger.error("Error in agent.add_tool trace creation: %s", e)

            return result

    return wrapper


def session_memory_wrap(
    gen_ai_endpoint,
    version,
    environment,
    application_name,
    tracer,
    pricing_info,
    capture_message_content,
    metrics,
    disable_metrics,
):
    """
    Wrap Agno Agent memory operations (get_session_summary, get_user_memories).
    """

    def wrapper(wrapped, instance, args, kwargs):
        # CRITICAL: Suppression check to prevent recursive instrumentation
        if context_api.get_value(context_api._SUPPRESS_INSTRUMENTATION_KEY):
            return wrapped(*args, **kwargs)

        # Extract method name from endpoint
        method_name = gen_ai_endpoint.split(".")[-1]
        span_name = f"memory {method_name.replace('get_', '').replace('_', ' ')}"

        with tracer.start_as_current_span(span_name, kind=SpanKind.INTERNAL) as span:
            # Execute memory operation with timing
            start_time = time.time()
            result = wrapped(*args, **kwargs)

            try:
                # Process request using utils function with ALL attributes from semcov
                # Extract method name for operation type
                operation_type = method_name.replace("get_", "").replace("_", " ")
                process_memory_request(
                    span,
                    instance,
                    args,
                    kwargs,
                    result,
                    start_time,
                    pricing_info,
                    environment,
                    application_name,
                    metrics,
                    capture_message_content,
                    disable_metrics,
                    version,
                    operation_type,
                )

                span.set_status(Status(StatusCode.OK))

            except Exception as e:
                handle_exception(span, e)
                logger.error("Error in memory operation trace creation: %s", e)

            return result

    return wrapper


def model_run_function_call_wrap(
    gen_ai_endpoint,
    version,
    environment,
    application_name,
    tracer,
    pricing_info,
    capture_message_content,
    metrics,
    disable_metrics,
):
    """
    Wrap Agno Model.run_function_call method to bridge agent and tool span context.
    This is the critical method that connects agent execution to tool execution.
    """

    def wrapper(wrapped, instance, args, kwargs):
        # CRITICAL: Suppression check to prevent recursive instrumentation
        if context_api.get_value(context_api._SUPPRESS_INSTRUMENTATION_KEY):
            return wrapped(*args, **kwargs)

        # Extract function call information for span naming
        function_call = args[0] if args else kwargs.get("function_call", None)
        function_name = None

        if function_call:
            # Extract function name from the FunctionCall object
            if hasattr(function_call, "function") and hasattr(
                function_call.function, "name"
            ):
                function_name = function_call.function.name
            elif hasattr(function_call, "name"):
                function_name = function_call.name

        # Skip creating span if we can't identify the function
        if not function_name or function_name == "unknown_function":
            return wrapped(*args, **kwargs)

        span_name = f"tool {function_name}"

        with tracer.start_as_current_span(span_name, kind=SpanKind.INTERNAL) as span:
            start_time = time.time()
            result = wrapped(*args, **kwargs)

            try:
                # Process request using utils function with ALL attributes from semcov
                process_tool_request(
                    span,
                    function_call,
                    args,
                    kwargs,
                    result,
                    start_time,
                    pricing_info,
                    environment,
                    application_name,
                    metrics,
                    capture_message_content,
                    disable_metrics,
                    version,
                    function_name,
                )

                span.set_status(Status(StatusCode.OK))

            except Exception as e:
                handle_exception(span, e)
                logger.error("Error in model.run_function_call trace creation: %s", e)

            return result

    return wrapper


# Tools Component Wrappers
def function_call_wrap(
    gen_ai_endpoint,
    version,
    environment,
    application_name,
    tracer,
    pricing_info,
    capture_message_content,
    metrics,
    disable_metrics,
):
    """
    Wrap Agno Function call method for tool execution tracing.
    """

    def wrapper(wrapped, instance, args, kwargs):
        # CRITICAL: Suppression check to prevent recursive instrumentation
        if context_api.get_value(context_api._SUPPRESS_INSTRUMENTATION_KEY):
            return wrapped(*args, **kwargs)

        # Extract function information
        function_name = getattr(
            instance, "name", getattr(instance, "__name__", "unknown_function")
        )
        span_name = f"tool {function_name}"

        with tracer.start_as_current_span(span_name, kind=SpanKind.INTERNAL) as span:
            # Execute function with timing
            start_time = time.time()
            result = wrapped(*args, **kwargs)

            try:
                # Process request using utils function with ALL attributes from semcov
                process_tool_request(
                    span,
                    instance,
                    args,
                    kwargs,
                    result,
                    start_time,
                    pricing_info,
                    environment,
                    application_name,
                    metrics,
                    capture_message_content,
                    disable_metrics,
                    version,
                    SemanticConvention.GEN_AI_OPERATION_TYPE_TOOLS,
                )

                span.set_status(Status(StatusCode.OK))

            except Exception as e:
                handle_exception(span, e)
                logger.error("Error in tool execution trace creation: %s", e)

            return result

    return wrapper


def toolkit_run_wrap(
    gen_ai_endpoint,
    version,
    environment,
    application_name,
    tracer,
    pricing_info,
    capture_message_content,
    metrics,
    disable_metrics,
):
    """
    Wrap Agno Toolkit run method for toolkit execution tracing.
    """

    def wrapper(wrapped, instance, args, kwargs):
        # CRITICAL: Suppression check to prevent recursive instrumentation
        if context_api.get_value(context_api._SUPPRESS_INSTRUMENTATION_KEY):
            return wrapped(*args, **kwargs)

        # Extract toolkit information
        toolkit_name = getattr(
            instance,
            "name",
            getattr(instance, "__class__", {}).get("__name__", "unknown_toolkit"),
        )
        span_name = f"toolkit {toolkit_name}"

        with tracer.start_as_current_span(span_name, kind=SpanKind.INTERNAL) as span:
            # Execute toolkit with timing
            start_time = time.time()
            result = wrapped(*args, **kwargs)

            try:
                # Process request using utils function with ALL attributes from semcov
                process_tool_request(
                    span,
                    instance,
                    args,
                    kwargs,
                    result,
                    start_time,
                    pricing_info,
                    environment,
                    application_name,
                    metrics,
                    capture_message_content,
                    disable_metrics,
                    version,
                    SemanticConvention.GEN_AI_OPERATION_TYPE_TOOLS,
                )

                span.set_status(Status(StatusCode.OK))

            except Exception as e:
                handle_exception(span, e)
                logger.error("Error in toolkit execution trace creation: %s", e)

            return result

    return wrapper


def function_entrypoint_wrap(
    gen_ai_endpoint,
    version,
    environment,
    application_name,
    tracer,
    pricing_info,
    capture_message_content,
    metrics,
    disable_metrics,
):
    """
    Wrap Agno Function.entrypoint method to catch direct tool executions that bypass Model.run_function_call.
    This ensures ALL tool executions maintain span context.
    """

    def wrapper(wrapped, instance, args, kwargs):
        # CRITICAL: Suppression check to prevent recursive instrumentation
        if context_api.get_value(context_api._SUPPRESS_INSTRUMENTATION_KEY):
            return wrapped(*args, **kwargs)

        # Extract function information for span naming
        function_name = getattr(instance, "name", None) or getattr(
            instance, "__name__", "unknown_function"
        )
        span_name = f"tool {function_name}"

        with tracer.start_as_current_span(span_name, kind=SpanKind.INTERNAL) as span:
            start_time = time.time()
            result = wrapped(*args, **kwargs)

            try:
                # Process request using utils function with ALL attributes from semcov
                process_tool_request(
                    span,
                    instance,
                    args,
                    kwargs,
                    result,
                    start_time,
                    pricing_info,
                    environment,
                    application_name,
                    metrics,
                    capture_message_content,
                    disable_metrics,
                    version,
                    function_name,
                )

                span.set_status(Status(StatusCode.OK))

            except Exception as e:
                handle_exception(span, e)
                logger.error("Error in function.entrypoint trace creation: %s", e)

            return result

    return wrapper


# Tool Execution Component Wrappers
def function_execute_wrap(
    gen_ai_endpoint,
    version,
    environment,
    application_name,
    tracer,
    pricing_info,
    capture_message_content,
    metrics,
    disable_metrics,
):
    """
    Wrap Agno FunctionCall.execute method for tool execution tracing using process functions only.
    """

    def wrapper(wrapped, instance, args, kwargs):
        # CRITICAL: Suppression check to prevent recursive instrumentation
        if context_api.get_value(context_api._SUPPRESS_INSTRUMENTATION_KEY):
            return wrapped(*args, **kwargs)

        # Extract function information
        function_name = getattr(instance.function, "name", "unknown_function")
        span_name = f"tool {function_name}"

        # CRITICAL: Capture current OpenTelemetry context to ensure proper nesting
        current_context = context_api.get_current()

        with tracer.start_as_current_span(
            span_name, kind=SpanKind.INTERNAL, context=current_context
        ) as span:
            # Execute tool with timing
            start_time = time.time()
            # CRITICAL: Set the span context as current for nested calls
            token = context_api.attach(
                context_api.set_value("current_span", span, current_context)
            )
            try:
                result = wrapped(*args, **kwargs)
            finally:
                context_api.detach(token)

            try:
                # Process request using utils function with ALL attributes from semcov
                process_tool_request(
                    span,
                    instance,
                    args,
                    kwargs,
                    result,
                    start_time,
                    pricing_info,
                    environment,
                    application_name,
                    metrics,
                    capture_message_content,
                    disable_metrics,
                    version,
                    function_name,
                )

                span.set_status(Status(StatusCode.OK))

            except Exception as e:
                handle_exception(span, e)
                logger.error("Error in tool execution trace creation: %s", e)

            return result

    return wrapper


def parallel_execution_wrap(
    gen_ai_endpoint,
    version,
    environment,
    application_name,
    tracer,
    pricing_info,
    capture_message_content,
    metrics,
    disable_metrics,
):
    """
    Wrap parallel execution methods to preserve OpenTelemetry context through ThreadPoolExecutor.
    This is critical for maintaining span hierarchy in parallel workflows and team operations.
    """
    from concurrent.futures import ThreadPoolExecutor

    def wrapper(wrapped, instance, args, kwargs):
        # CRITICAL: Suppression check to prevent recursive instrumentation
        if context_api.get_value(context_api._SUPPRESS_INSTRUMENTATION_KEY):
            return wrapped(*args, **kwargs)

        # Capture current OpenTelemetry context
        current_context = context_api.get_current()

        # Patch ThreadPoolExecutor.submit to preserve context
        original_submit = ThreadPoolExecutor.submit

        def context_preserving_submit(
            executor_self, func, *submit_args, **submit_kwargs
        ):
            """Submit function to executor while preserving OpenTelemetry context"""

            def context_wrapper(*wrapper_args, **wrapper_kwargs):
                # Restore the OpenTelemetry context in the thread
                token = context_api.attach(current_context)
                try:
                    return func(*wrapper_args, **wrapper_kwargs)
                finally:
                    context_api.detach(token)

            return original_submit(
                executor_self, context_wrapper, *submit_args, **submit_kwargs
            )

        # Temporarily patch the submit method
        ThreadPoolExecutor.submit = context_preserving_submit

        try:
            result = wrapped(*args, **kwargs)
        finally:
            # Restore original submit method
            ThreadPoolExecutor.submit = original_submit

        return result

    return wrapper


def reasoning_tool_wrap(
    gen_ai_endpoint,
    version,
    environment,
    application_name,
    tracer,
    pricing_info,
    capture_message_content,
    metrics,
    disable_metrics,
):
    """
    Wrap reasoning tools (think, analyze) to maintain proper span hierarchy.
    These tools were previously creating disconnected spans.
    """

    def wrapper(wrapped, instance, args, kwargs):
        # CRITICAL: Suppression check to prevent recursive instrumentation
        if context_api.get_value(context_api._SUPPRESS_INSTRUMENTATION_KEY):
            return wrapped(*args, **kwargs)

        # Extract tool name from endpoint
        tool_name = gen_ai_endpoint.split(".")[-1]
        span_name = f"reasoning {tool_name}"

        with tracer.start_as_current_span(span_name, kind=SpanKind.INTERNAL) as span:
            start_time = time.time()
            result = wrapped(*args, **kwargs)

            try:
                # Process request using utils function with ALL attributes from semcov
                process_reasoning_request(
                    span,
                    instance,
                    args,
                    kwargs,
                    result,
                    start_time,
                    pricing_info,
                    environment,
                    application_name,
                    metrics,
                    capture_message_content,
                    disable_metrics,
                    version,
                    tool_name,
                )

                span.set_status(Status(StatusCode.OK))

            except Exception as e:
                handle_exception(span, e)
                logger.error("Error in reasoning tool trace creation: %s", e)

            return result

    return wrapper


# Reasoning Component Wrappers
def reasoning_wrap(
    gen_ai_endpoint,
    version,
    environment,
    application_name,
    tracer,
    pricing_info,
    capture_message_content,
    metrics,
    disable_metrics,
):
    """
    Wrap Agno Agent.reason method for reasoning operation tracing.
    """

    def wrapper(wrapped, instance, args, kwargs):
        # CRITICAL: Suppression check to prevent recursive instrumentation
        if context_api.get_value(context_api._SUPPRESS_INSTRUMENTATION_KEY):
            return wrapped(*args, **kwargs)

        agent_name = getattr(instance, "name", "unknown_agent")
        span_name = f"reasoning {agent_name}"

        with tracer.start_as_current_span(span_name, kind=SpanKind.INTERNAL) as span:
            # Execute reasoning with timing
            start_time = time.time()
            result = wrapped(*args, **kwargs)

            try:
                # Process request using utils function with ALL attributes from semcov
                # Use reasoning operation for reasoning requests
                process_reasoning_request(
                    span,
                    instance,
                    args,
                    kwargs,
                    result,
                    start_time,
                    pricing_info,
                    environment,
                    application_name,
                    metrics,
                    capture_message_content,
                    disable_metrics,
                    version,
                    "reasoning",
                )

                span.set_status(Status(StatusCode.OK))

            except Exception as e:
                handle_exception(span, e)
                logger.error("Error in reasoning trace creation: %s", e)

            return result

    return wrapper


def memory_operation_wrap(
    gen_ai_endpoint,
    version,
    environment,
    application_name,
    tracer,
    pricing_info,
    capture_message_content,
    metrics,
    disable_metrics,
):
    """
    Wrap Memory operations that bypass Model.run_function_call to maintain span context.
    """

    def wrapper(wrapped, instance, args, kwargs):
        # CRITICAL: Suppression check to prevent recursive instrumentation
        if context_api.get_value(context_api._SUPPRESS_INSTRUMENTATION_KEY):
            return wrapped(*args, **kwargs)

        # Extract operation name from endpoint
        operation_name = gen_ai_endpoint.split(".")[-1]
        span_name = f"memory {operation_name.replace('_', ' ')}"

        with tracer.start_as_current_span(span_name, kind=SpanKind.INTERNAL) as span:
            start_time = time.time()
            result = wrapped(*args, **kwargs)

            try:
                # Process request using utils function with ALL attributes from semcov
                process_memory_request(
                    span,
                    instance,
                    args,
                    kwargs,
                    result,
                    start_time,
                    pricing_info,
                    environment,
                    application_name,
                    metrics,
                    capture_message_content,
                    disable_metrics,
                    version,
                    operation_name.replace("_", " "),
                )

                span.set_status(Status(StatusCode.OK))

            except Exception as e:
                handle_exception(span, e)
                logger.error("Error in memory operation trace creation: %s", e)

            return result

    return wrapper


# Memory Component Wrappers
def memory_add_wrap(
    gen_ai_endpoint,
    version,
    environment,
    application_name,
    tracer,
    pricing_info,
    capture_message_content,
    metrics,
    disable_metrics,
):
    """
    Wrap Agno Memory add method for memory storage tracing using process functions only.
    """

    def wrapper(wrapped, instance, args, kwargs):
        # CRITICAL: Suppression check to prevent recursive instrumentation
        if context_api.get_value(context_api._SUPPRESS_INSTRUMENTATION_KEY):
            return wrapped(*args, **kwargs)

        span_name = "memory add"

        with tracer.start_as_current_span(span_name, kind=SpanKind.INTERNAL) as span:
            # Execute memory operation with timing
            start_time = time.time()
            result = wrapped(*args, **kwargs)

            try:
                # Process request using utils function with ALL attributes from semcov
                process_memory_request(
                    span,
                    instance,
                    args,
                    kwargs,
                    result,
                    start_time,
                    pricing_info,
                    environment,
                    application_name,
                    metrics,
                    capture_message_content,
                    disable_metrics,
                    version,
                    "add",
                )

                span.set_status(Status(StatusCode.OK))

            except Exception as e:
                handle_exception(span, e)
                logger.error("Error in memory add trace creation: %s", e)

            return result

    return wrapper


def memory_search_wrap(
    gen_ai_endpoint,
    version,
    environment,
    application_name,
    tracer,
    pricing_info,
    capture_message_content,
    metrics,
    disable_metrics,
):
    """
    Wrap Agno Memory search method for memory retrieval tracing using process functions only.
    """

    def wrapper(wrapped, instance, args, kwargs):
        # CRITICAL: Suppression check to prevent recursive instrumentation
        if context_api.get_value(context_api._SUPPRESS_INSTRUMENTATION_KEY):
            return wrapped(*args, **kwargs)

        span_name = "memory search"

        with tracer.start_as_current_span(span_name, kind=SpanKind.INTERNAL) as span:
            # Execute memory search with timing
            start_time = time.time()
            result = wrapped(*args, **kwargs)

            try:
                # Process request using utils function with ALL attributes from semcov
                process_memory_request(
                    span,
                    instance,
                    args,
                    kwargs,
                    result,
                    start_time,
                    pricing_info,
                    environment,
                    application_name,
                    metrics,
                    capture_message_content,
                    disable_metrics,
                    version,
                    "search",
                )

                span.set_status(Status(StatusCode.OK))

            except Exception as e:
                handle_exception(span, e)
                logger.error("Error in memory search trace creation: %s", e)

            return result

    return wrapper


# VectorDB Component Wrappers
def vectordb_search_wrap(
    gen_ai_endpoint,
    version,
    environment,
    application_name,
    tracer,
    pricing_info,
    capture_message_content,
    metrics,
    disable_metrics,
):
    """
    Wrap Agno VectorDb search method for vector search tracing using process functions only.
    """

    def wrapper(wrapped, instance, args, kwargs):
        # CRITICAL: Suppression check to prevent recursive instrumentation
        if context_api.get_value(context_api._SUPPRESS_INSTRUMENTATION_KEY):
            return wrapped(*args, **kwargs)

        span_name = "vectordb search"

        with tracer.start_as_current_span(span_name, kind=SpanKind.CLIENT) as span:
            # Execute vectordb search with timing
            start_time = time.time()
            result = wrapped(*args, **kwargs)

            try:
                # Process request using utils function with ALL attributes from semcov
                process_vectordb_request(
                    span,
                    instance,
                    args,
                    kwargs,
                    result,
                    start_time,
                    pricing_info,
                    environment,
                    application_name,
                    metrics,
                    capture_message_content,
                    disable_metrics,
                    version,
                    "search",
                )

                span.set_status(Status(StatusCode.OK))

            except Exception as e:
                handle_exception(span, e)
                logger.error("Error in vectordb search trace creation: %s", e)

            return result

    return wrapper


def vectordb_upsert_wrap(
    gen_ai_endpoint,
    version,
    environment,
    application_name,
    tracer,
    pricing_info,
    capture_message_content,
    metrics,
    disable_metrics,
):
    """
    Wrap Agno VectorDb upsert method for vector upsert tracing using process functions only.
    """

    def wrapper(wrapped, instance, args, kwargs):
        # CRITICAL: Suppression check to prevent recursive instrumentation
        if context_api.get_value(context_api._SUPPRESS_INSTRUMENTATION_KEY):
            return wrapped(*args, **kwargs)

        span_name = "vectordb upsert"

        with tracer.start_as_current_span(span_name, kind=SpanKind.CLIENT) as span:
            # Execute vectordb upsert with timing
            start_time = time.time()
            result = wrapped(*args, **kwargs)

            try:
                # Process request using utils function with ALL attributes from semcov
                process_vectordb_request(
                    span,
                    instance,
                    args,
                    kwargs,
                    result,
                    start_time,
                    pricing_info,
                    environment,
                    application_name,
                    metrics,
                    capture_message_content,
                    disable_metrics,
                    version,
                    "upsert",
                )

                span.set_status(Status(StatusCode.OK))

            except Exception as e:
                handle_exception(span, e)
                logger.error("Error in vectordb upsert trace creation: %s", e)

            return result

    return wrapper


# Knowledge Component Wrappers
def knowledge_search_wrap(
    gen_ai_endpoint,
    version,
    environment,
    application_name,
    tracer,
    pricing_info,
    capture_message_content,
    metrics,
    disable_metrics,
):
    """
    Wrap Agno AgentKnowledge search method for knowledge search tracing using process functions only.
    """

    def wrapper(wrapped, instance, args, kwargs):
        # CRITICAL: Suppression check to prevent recursive instrumentation
        if context_api.get_value(context_api._SUPPRESS_INSTRUMENTATION_KEY):
            return wrapped(*args, **kwargs)

        span_name = "knowledge search"

        with tracer.start_as_current_span(span_name, kind=SpanKind.INTERNAL) as span:
            # Execute knowledge search with timing
            start_time = time.time()
            result = wrapped(*args, **kwargs)

            try:
                # Process request using utils function with ALL attributes from semcov
                process_knowledge_request(
                    span,
                    instance,
                    args,
                    kwargs,
                    result,
                    start_time,
                    pricing_info,
                    environment,
                    application_name,
                    metrics,
                    capture_message_content,
                    disable_metrics,
                    version,
                    "search",
                )

                span.set_status(Status(StatusCode.OK))

            except Exception as e:
                handle_exception(span, e)
                logger.error("Error in knowledge search trace creation: %s", e)

            return result

    return wrapper


def knowledge_add_wrap(
    gen_ai_endpoint,
    version,
    environment,
    application_name,
    tracer,
    pricing_info,
    capture_message_content,
    metrics,
    disable_metrics,
):
    """
    Wrap Agno AgentKnowledge add method for knowledge addition tracing using process functions only.
    """

    def wrapper(wrapped, instance, args, kwargs):
        # CRITICAL: Suppression check to prevent recursive instrumentation
        if context_api.get_value(context_api._SUPPRESS_INSTRUMENTATION_KEY):
            return wrapped(*args, **kwargs)

        span_name = "knowledge add"

        with tracer.start_as_current_span(span_name, kind=SpanKind.INTERNAL) as span:
            # Execute knowledge addition with timing
            start_time = time.time()
            result = wrapped(*args, **kwargs)

            try:
                # Process request using utils function with ALL attributes from semcov
                process_knowledge_request(
                    span,
                    instance,
                    args,
                    kwargs,
                    result,
                    start_time,
                    pricing_info,
                    environment,
                    application_name,
                    metrics,
                    capture_message_content,
                    disable_metrics,
                    version,
                    "add",
                )

                span.set_status(Status(StatusCode.OK))

            except Exception as e:
                handle_exception(span, e)
                logger.error("Error in knowledge add trace creation: %s", e)

            return result

    return wrapper


# Workflow Component Wrappers
def workflow_run_wrap(
    gen_ai_endpoint,
    version,
    environment,
    application_name,
    tracer,
    pricing_info,
    capture_message_content,
    metrics,
    disable_metrics,
):
    """
    Wrap Agno Workflow run method for workflow execution tracing using process functions only.
    """

    def wrapper(wrapped, instance, args, kwargs):
        # CRITICAL: Suppression check to prevent recursive instrumentation
        if context_api.get_value(context_api._SUPPRESS_INSTRUMENTATION_KEY):
            return wrapped(*args, **kwargs)

        workflow_name = getattr(instance, "name", "unknown_workflow")
        span_name = f"workflow {workflow_name}"

        with tracer.start_as_current_span(span_name, kind=SpanKind.CLIENT) as span:
            # Execute workflow with timing
            start_time = time.time()
            result = wrapped(*args, **kwargs)

            try:
                # Process request using utils function with ALL attributes from semcov
                process_workflow_request(
                    span,
                    instance,
                    args,
                    kwargs,
                    result,
                    start_time,
                    pricing_info,
                    environment,
                    application_name,
                    metrics,
                    capture_message_content,
                    disable_metrics,
                    version,
                )

                span.set_status(Status(StatusCode.OK))

            except Exception as e:
                handle_exception(span, e)
                logger.error("Error in workflow run trace creation: %s", e)

            return result

    return wrapper


# Team Component Wrappers
def team_run_wrap(
    gen_ai_endpoint,
    version,
    environment,
    application_name,
    tracer,
    pricing_info,
    capture_message_content,
    metrics,
    disable_metrics,
):
    """
    Wrap Agno Team run method for team execution tracing.
    """

    def wrapper(wrapped, instance, args, kwargs):
        # CRITICAL: Suppression check to prevent recursive instrumentation
        if context_api.get_value(context_api._SUPPRESS_INSTRUMENTATION_KEY):
            return wrapped(*args, **kwargs)

        team_name = getattr(instance, "name", "unknown_team")
        span_name = f"team {team_name}"

        with tracer.start_as_current_span(span_name, kind=SpanKind.CLIENT) as span:
            # Execute team with timing
            start_time = time.time()
            result = wrapped(*args, **kwargs)

            try:
                # Process request using utils function with ALL attributes from semcov
                process_team_request(
                    span,
                    instance,
                    args,
                    kwargs,
                    result,
                    start_time,
                    pricing_info,
                    environment,
                    application_name,
                    metrics,
                    capture_message_content,
                    disable_metrics,
                    version,
                )

                span.set_status(Status(StatusCode.OK))

            except Exception as e:
                handle_exception(span, e)
                logger.error("Error in team run trace creation: %s", e)

            return result

    return wrapper
