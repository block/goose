"""
LangGraph async wrapper for instrumentation
"""

import time
import json
from opentelemetry.trace import SpanKind, Status, StatusCode
from opentelemetry import context as context_api
from openlit.__helpers import handle_exception
from openlit.instrumentation.langgraph.utils import (
    process_langgraph_response,
    OPERATION_MAP,
    set_server_address_and_port,
    generate_span_name,
    extract_messages_from_input,
    get_message_content,
    get_message_role,
    extract_config_info,
    SemanticConvention,
)


def async_general_wrap(
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
    Creates a wrapper for async LangGraph operations.

    Args:
        gen_ai_endpoint: The endpoint being wrapped (e.g., "graph_ainvoke")
        version: SDK version
        environment: Deployment environment
        application_name: Application name
        tracer: OpenTelemetry tracer
        pricing_info: Pricing information for cost calculation
        capture_message_content: Whether to capture message content
        metrics: Metrics registry
        disable_metrics: Whether metrics are disabled

    Returns:
        Async wrapper function
    """

    # For astream, we need a sync wrapper that returns an async generator
    if gen_ai_endpoint == "graph_astream":

        def astream_wrapper(wrapped, instance, args, kwargs):
            """
            Wraps the astream method - returns an async generator wrapper.
            """
            # Check for suppression
            if context_api.get_value(context_api._SUPPRESS_INSTRUMENTATION_KEY):
                return wrapped(*args, **kwargs)

            # Get server address and port
            server_address, server_port = set_server_address_and_port(instance)

            # Get operation type
            operation_type = OPERATION_MAP.get(
                gen_ai_endpoint, SemanticConvention.GEN_AI_OPERATION_TYPE_FRAMEWORK
            )

            # Generate span name
            span_name = generate_span_name(
                operation_type, gen_ai_endpoint, instance, args, kwargs
            )

            # Return an async generator wrapper
            return _create_async_stream_wrapper(
                wrapped,
                instance,
                args,
                kwargs,
                span_name,
                tracer,
                operation_type,
                server_address,
                server_port,
                environment,
                application_name,
                metrics,
                capture_message_content,
                disable_metrics,
                version,
                gen_ai_endpoint,
            )

        return astream_wrapper

    # For other async operations (ainvoke, aget_state), use async wrapper
    async def wrapper(wrapped, instance, args, kwargs):
        """
        Wraps the async LangGraph operation with telemetry.
        """
        # Check for suppression
        if context_api.get_value(context_api._SUPPRESS_INSTRUMENTATION_KEY):
            return await wrapped(*args, **kwargs)

        # Get server address and port
        server_address, server_port = set_server_address_and_port(instance)

        # Get operation type
        operation_type = OPERATION_MAP.get(
            gen_ai_endpoint, SemanticConvention.GEN_AI_OPERATION_TYPE_FRAMEWORK
        )

        # Generate span name
        span_name = generate_span_name(
            operation_type, gen_ai_endpoint, instance, args, kwargs
        )

        with tracer.start_as_current_span(span_name, kind=SpanKind.CLIENT) as span:
            start_time = time.time()

            try:
                # Execute the wrapped async function
                response = await wrapped(*args, **kwargs)

                # Process response
                response = process_langgraph_response(
                    response,
                    operation_type,
                    server_address,
                    server_port,
                    environment,
                    application_name,
                    metrics,
                    start_time,
                    span,
                    capture_message_content,
                    disable_metrics,
                    version,
                    instance,
                    args,
                    endpoint=gen_ai_endpoint,
                    **kwargs,
                )

                return response

            except Exception as e:
                handle_exception(span, e)
                raise

    return wrapper


async def _create_async_stream_wrapper(
    wrapped,
    instance,
    args,
    kwargs,
    span_name,
    tracer,
    operation_type,
    server_address,
    server_port,
    environment,
    application_name,
    metrics,
    capture_message_content,
    disable_metrics,
    version,
    endpoint,
):
    """
    Creates an async generator wrapper for astream.
    """
    with tracer.start_as_current_span(span_name, kind=SpanKind.CLIENT) as span:
        start_time = time.time()

        # Set basic attributes
        span.set_attribute("telemetry.sdk.name", "openlit")
        span.set_attribute(SemanticConvention.GEN_AI_SDK_VERSION, version)
        span.set_attribute(
            SemanticConvention.GEN_AI_SYSTEM, SemanticConvention.GEN_AI_SYSTEM_LANGGRAPH
        )
        span.set_attribute(SemanticConvention.GEN_AI_OPERATION, operation_type)
        span.set_attribute(SemanticConvention.GEN_AI_REQUEST_MODEL, "unknown")
        span.set_attribute(SemanticConvention.SERVER_ADDRESS, server_address)
        span.set_attribute(SemanticConvention.SERVER_PORT, server_port)
        span.set_attribute(SemanticConvention.GEN_AI_ENVIRONMENT, environment)
        span.set_attribute(SemanticConvention.GEN_AI_APPLICATION_NAME, application_name)
        span.set_attribute(SemanticConvention.LANGGRAPH_EXECUTION_MODE, "stream")
        span.set_attribute(SemanticConvention.GEN_AI_REQUEST_IS_STREAM, True)

        # Extract stream mode from kwargs
        stream_mode = kwargs.get("stream_mode", "values")
        span.set_attribute(SemanticConvention.LANGGRAPH_STREAM_MODE, str(stream_mode))

        # Track execution state
        execution_state = {
            "executed_nodes": [],
            "message_count": 0,
            "chunk_count": 0,
            "final_response": None,
            "first_token_time": None,
        }

        # Capture input messages
        input_data = args[0] if args else kwargs.get("input", {})
        messages = extract_messages_from_input(input_data)
        if messages:
            execution_state["message_count"] = len(messages)
            if capture_message_content:
                for i, msg in enumerate(messages[:3]):
                    content = get_message_content(msg)
                    role = get_message_role(msg)
                    if content:
                        span.set_attribute(f"gen_ai.prompt.{i}.content", content[:500])
                        span.set_attribute(f"gen_ai.prompt.{i}.role", role)

        # Extract config information
        config = kwargs.get("config")
        if config:
            config_info = extract_config_info(config)
            if config_info.get("thread_id"):
                span.set_attribute(
                    SemanticConvention.LANGGRAPH_THREAD_ID, config_info["thread_id"]
                )
            if config_info.get("checkpoint_id"):
                span.set_attribute(
                    SemanticConvention.LANGGRAPH_CHECKPOINT_ID,
                    config_info["checkpoint_id"],
                )

        try:
            # Get the async stream - call without await since it returns async generator
            stream_gen = wrapped(*args, **kwargs)

            async for chunk in stream_gen:
                # Track first token time
                if execution_state["first_token_time"] is None:
                    execution_state["first_token_time"] = time.time()
                    ttft = execution_state["first_token_time"] - start_time
                    span.set_attribute(SemanticConvention.GEN_AI_SERVER_TTFT, ttft)

                execution_state["chunk_count"] += 1

                # Process chunk
                _process_stream_chunk(chunk, execution_state, stream_mode)

                yield chunk

            # Set final attributes
            _finalize_async_stream_span(
                span, execution_state, capture_message_content, start_time
            )

        except Exception as e:
            handle_exception(span, e)
            raise


def _process_stream_chunk(chunk, execution_state, stream_mode):
    """Process a stream chunk and update execution state."""
    try:
        if isinstance(chunk, dict):
            for key in chunk:
                # Track node executions (exclude special keys)
                if key not in ("__start__", "__end__", "__interrupt__", "messages"):
                    if key not in execution_state["executed_nodes"]:
                        execution_state["executed_nodes"].append(key)

                # Track messages in chunk
                chunk_value = chunk[key]
                if isinstance(chunk_value, dict) and "messages" in chunk_value:
                    msg_list = chunk_value["messages"]
                    if isinstance(msg_list, list):
                        execution_state["message_count"] += len(msg_list)
                        for msg in msg_list:
                            content = get_message_content(msg)
                            if content:
                                execution_state["final_response"] = content
                elif key == "messages" and isinstance(chunk_value, list):
                    # Direct messages in chunk
                    execution_state["message_count"] += len(chunk_value)
                    for msg in chunk_value:
                        content = get_message_content(msg)
                        if content:
                            execution_state["final_response"] = content

        # Handle tuple format (node_name, value) for some stream modes
        elif isinstance(chunk, tuple) and len(chunk) >= 2:
            node_name, value = chunk[0], chunk[1]
            if node_name not in ("__start__", "__end__", "__interrupt__"):
                if node_name not in execution_state["executed_nodes"]:
                    execution_state["executed_nodes"].append(node_name)

            if isinstance(value, dict) and "messages" in value:
                msg_list = value["messages"]
                if isinstance(msg_list, list):
                    execution_state["message_count"] += len(msg_list)
                    for msg in msg_list:
                        content = get_message_content(msg)
                        if content:
                            execution_state["final_response"] = content

    except Exception:
        # Don't fail on chunk processing errors
        pass


def _finalize_async_stream_span(
    span, execution_state, capture_message_content, start_time
):
    """Finalize async stream span with collected telemetry."""
    end_time = time.time()

    # Set execution attributes
    span.set_attribute(
        SemanticConvention.LANGGRAPH_EXECUTED_NODES,
        json.dumps(execution_state["executed_nodes"]),
    )
    span.set_attribute(
        SemanticConvention.LANGGRAPH_NODE_EXECUTION_COUNT,
        len(execution_state["executed_nodes"]),
    )
    span.set_attribute(
        SemanticConvention.LANGGRAPH_MESSAGE_COUNT, execution_state["message_count"]
    )
    span.set_attribute(
        SemanticConvention.LANGGRAPH_CHUNK_COUNT, execution_state["chunk_count"]
    )

    if execution_state["final_response"] and capture_message_content:
        span.set_attribute(
            SemanticConvention.LANGGRAPH_FINAL_RESPONSE,
            execution_state["final_response"][:500],
        )
        span.set_attribute(
            SemanticConvention.GEN_AI_CONTENT_COMPLETION,
            execution_state["final_response"][:1000],
        )

    span.set_attribute(SemanticConvention.LANGGRAPH_GRAPH_STATUS, "success")
    span.set_attribute(
        SemanticConvention.GEN_AI_CLIENT_OPERATION_DURATION, end_time - start_time
    )
    span.set_status(Status(StatusCode.OK))


def async_checkpoint_wrap(
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
    Creates a wrapper for async checkpointer operations.

    Args:
        gen_ai_endpoint: The endpoint being wrapped (e.g., "checkpoint_setup")
        version: SDK version
        environment: Deployment environment
        application_name: Application name
        tracer: OpenTelemetry tracer
        pricing_info: Pricing information
        capture_message_content: Whether to capture message content
        metrics: Metrics registry
        disable_metrics: Whether metrics are disabled

    Returns:
        Async wrapper function
    """

    async def wrapper(wrapped, instance, args, kwargs):
        """
        Wraps async checkpointer operations with telemetry.
        """
        if context_api.get_value(context_api._SUPPRESS_INSTRUMENTATION_KEY):
            return await wrapped(*args, **kwargs)

        operation_type = OPERATION_MAP.get(
            gen_ai_endpoint, SemanticConvention.GEN_AI_OPERATION_TYPE_FRAMEWORK
        )
        span_name = generate_span_name(
            operation_type, gen_ai_endpoint, instance, args, kwargs
        )

        with tracer.start_as_current_span(span_name, kind=SpanKind.CLIENT) as span:
            start_time = time.time()

            # Set basic attributes
            span.set_attribute(
                SemanticConvention.GEN_AI_SYSTEM,
                SemanticConvention.GEN_AI_SYSTEM_LANGGRAPH,
            )
            span.set_attribute(SemanticConvention.GEN_AI_OPERATION, operation_type)
            span.set_attribute(SemanticConvention.GEN_AI_ENVIRONMENT, environment)
            span.set_attribute(
                SemanticConvention.GEN_AI_APPLICATION_NAME, application_name
            )
            span.set_attribute(SemanticConvention.GEN_AI_SDK_VERSION, version)

            try:
                # Execute the wrapped function
                result = await wrapped(*args, **kwargs)

                end_time = time.time()
                span.set_attribute(
                    SemanticConvention.GEN_AI_CLIENT_OPERATION_DURATION,
                    end_time - start_time,
                )

                # Set checkpoint-specific attributes based on operation
                if gen_ai_endpoint == "checkpoint_write":
                    # Extract config info if available
                    config = args[0] if args else kwargs.get("config")
                    if config:
                        config_info = extract_config_info(config)
                        if config_info.get("thread_id"):
                            span.set_attribute(
                                SemanticConvention.LANGGRAPH_THREAD_ID,
                                config_info["thread_id"],
                            )

                elif gen_ai_endpoint == "checkpoint_read":
                    config = args[0] if args else kwargs.get("config")
                    if config:
                        config_info = extract_config_info(config)
                        if config_info.get("thread_id"):
                            span.set_attribute(
                                SemanticConvention.LANGGRAPH_THREAD_ID,
                                config_info["thread_id"],
                            )

                    # If result has checkpoint info
                    if result and hasattr(result, "checkpoint"):
                        checkpoint = result.checkpoint
                        if checkpoint and hasattr(checkpoint, "id"):
                            span.set_attribute(
                                SemanticConvention.LANGGRAPH_CHECKPOINT_ID,
                                str(checkpoint.id),
                            )

                span.set_attribute(SemanticConvention.LANGGRAPH_GRAPH_STATUS, "success")
                span.set_status(Status(StatusCode.OK))

                # Record metrics
                if not disable_metrics and metrics:
                    _record_checkpoint_metrics(
                        metrics,
                        operation_type,
                        end_time - start_time,
                        environment,
                        application_name,
                    )

                return result

            except Exception as e:
                handle_exception(span, e)
                raise

    return wrapper


def _record_checkpoint_metrics(
    metrics, operation_type, duration, environment, application_name
):
    """Record checkpoint operation metrics."""
    try:
        attributes = {
            "gen_ai.operation.name": operation_type,
            "gen_ai.system": SemanticConvention.GEN_AI_SYSTEM_LANGGRAPH,
            "service.name": application_name,
            "deployment.environment": environment,
        }

        if "genai_client_operation_duration" in metrics:
            metrics["genai_client_operation_duration"].record(duration, attributes)

        if "genai_requests" in metrics:
            metrics["genai_requests"].add(1, attributes)

    except Exception:
        pass
