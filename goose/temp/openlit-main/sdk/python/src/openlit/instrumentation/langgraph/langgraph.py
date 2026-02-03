"""
LangGraph sync wrapper for instrumentation
"""

import time
from opentelemetry.trace import SpanKind
from opentelemetry import context as context_api
from openlit.__helpers import handle_exception
from openlit.instrumentation.langgraph.utils import (
    process_langgraph_response,
    OPERATION_MAP,
    set_server_address_and_port,
    generate_span_name,
    set_graph_attributes,
    extract_messages_from_input,
    get_message_content,
    get_message_role,
    SemanticConvention,
)


def general_wrap(
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
    Creates a wrapper for sync LangGraph operations.

    Args:
        gen_ai_endpoint: The endpoint being wrapped (e.g., "graph_invoke")
        version: SDK version
        environment: Deployment environment
        application_name: Application name
        tracer: OpenTelemetry tracer
        pricing_info: Pricing information for cost calculation
        capture_message_content: Whether to capture message content
        metrics: Metrics registry
        disable_metrics: Whether metrics are disabled

    Returns:
        Wrapper function
    """

    def wrapper(wrapped, instance, args, kwargs):
        """
        Wraps the LangGraph operation with telemetry.
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

        with tracer.start_as_current_span(span_name, kind=SpanKind.CLIENT) as span:
            start_time = time.time()

            try:
                # Handle stream operations specially
                if gen_ai_endpoint == "graph_stream":
                    return _handle_stream(
                        wrapped,
                        instance,
                        args,
                        kwargs,
                        span,
                        start_time,
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

                # Execute the wrapped function
                response = wrapped(*args, **kwargs)

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


def _handle_stream(
    wrapped,
    instance,
    args,
    kwargs,
    span,
    start_time,
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
    Handle streaming responses with proper telemetry.

    Returns a generator that wraps the stream and captures telemetry.
    """
    # Set basic attributes directly without using common_framework_span_attributes
    # (since we don't have _end_time for streaming until it completes)
    span.set_attribute("telemetry.sdk.name", "openlit")
    span.set_attribute(SemanticConvention.GEN_AI_SDK_VERSION, version)
    span.set_attribute(
        SemanticConvention.GEN_AI_SYSTEM, SemanticConvention.GEN_AI_SYSTEM_LANGGRAPH
    )
    span.set_attribute(SemanticConvention.GEN_AI_OPERATION, endpoint)
    span.set_attribute(SemanticConvention.GEN_AI_REQUEST_MODEL, "unknown")
    span.set_attribute(SemanticConvention.SERVER_ADDRESS, server_address)
    span.set_attribute(SemanticConvention.SERVER_PORT, server_port)
    span.set_attribute(SemanticConvention.GEN_AI_ENVIRONMENT, environment)
    span.set_attribute(SemanticConvention.GEN_AI_APPLICATION_NAME, application_name)

    span.set_attribute(SemanticConvention.GEN_AI_OPERATION, operation_type)
    span.set_attribute(SemanticConvention.LANGGRAPH_EXECUTION_MODE, "stream")
    span.set_attribute(SemanticConvention.GEN_AI_REQUEST_IS_STREAM, True)

    # Track execution state
    execution_state = {
        "executed_nodes": [],
        "message_count": 0,
        "chunk_count": 0,
        "final_response": None,
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

    try:
        # Get the stream
        stream_gen = wrapped(*args, **kwargs)

        def stream_wrapper():
            try:
                for chunk in stream_gen:
                    execution_state["chunk_count"] += 1

                    # Process chunk
                    if isinstance(chunk, dict):
                        for key in chunk:
                            # Track node executions
                            if key not in ("__start__", "__end__", "__interrupt__"):
                                if key not in execution_state["executed_nodes"]:
                                    execution_state["executed_nodes"].append(key)

                            # Track messages in chunk
                            chunk_value = chunk[key]
                            if (
                                isinstance(chunk_value, dict)
                                and "messages" in chunk_value
                            ):
                                msg_list = chunk_value["messages"]
                                if isinstance(msg_list, list):
                                    execution_state["message_count"] += len(msg_list)
                                    for msg in msg_list:
                                        content = get_message_content(msg)
                                        if content:
                                            execution_state["final_response"] = content

                    yield chunk

                # Set final attributes
                _finalize_stream_span(
                    span, execution_state, capture_message_content, start_time
                )

            except Exception as e:
                handle_exception(span, e)
                raise

        return stream_wrapper()

    except Exception as e:
        handle_exception(span, e)
        raise


def _finalize_stream_span(span, execution_state, capture_message_content, start_time):
    """Finalize stream span with collected telemetry."""
    import json
    from opentelemetry.trace import Status, StatusCode

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


def wrap_compile(
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
    Special wrapper for StateGraph.compile that captures graph structure.
    """

    def wrapper(wrapped, instance, args, kwargs):
        if context_api.get_value(context_api._SUPPRESS_INSTRUMENTATION_KEY):
            return wrapped(*args, **kwargs)

        server_address, server_port = set_server_address_and_port(instance)
        operation_type = OPERATION_MAP.get(
            gen_ai_endpoint, SemanticConvention.GEN_AI_OPERATION_TYPE_FRAMEWORK
        )
        span_name = generate_span_name(
            operation_type, gen_ai_endpoint, instance, args, kwargs
        )

        with tracer.start_as_current_span(span_name, kind=SpanKind.INTERNAL) as span:
            start_time = time.time()

            try:
                # Execute compile
                result = wrapped(*args, **kwargs)

                # Extract graph structure from instance (StateGraph)
                nodes = []
                edges = []

                if hasattr(instance, "nodes"):
                    nodes = (
                        list(instance.nodes.keys())
                        if hasattr(instance.nodes, "keys")
                        else []
                    )

                if hasattr(instance, "edges"):
                    edge_set = instance.edges
                    if isinstance(edge_set, set):
                        for edge in edge_set:
                            if isinstance(edge, tuple) and len(edge) >= 2:
                                edges.append(f"{edge[0]}->{edge[1]}")

                # Set graph attributes
                set_graph_attributes(span, nodes, edges)

                # Set common attributes
                from openlit.__helpers import common_framework_span_attributes

                scope = type("GenericScope", (), {})()
                scope._span = span
                scope._start_time = start_time
                scope._end_time = time.time()

                common_framework_span_attributes(
                    scope,
                    SemanticConvention.GEN_AI_SYSTEM_LANGGRAPH,
                    server_address,
                    server_port,
                    environment,
                    application_name,
                    version,
                    gen_ai_endpoint,
                    instance,
                )

                span.set_attribute(SemanticConvention.GEN_AI_OPERATION, operation_type)
                span.set_attribute(SemanticConvention.LANGGRAPH_GRAPH_STATUS, "success")

                from opentelemetry.trace import Status, StatusCode

                span.set_status(Status(StatusCode.OK))

                return result

            except Exception as e:
                handle_exception(span, e)
                raise

    return wrapper


def wrap_add_node(
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
    Special wrapper for StateGraph.add_node that wraps node functions for per-node tracing.
    """
    import inspect
    from functools import wraps

    def wrapper(wrapped, instance, args, kwargs):
        if context_api.get_value(context_api._SUPPRESS_INSTRUMENTATION_KEY):
            return wrapped(*args, **kwargs)

        # Extract node name and action
        if args:
            node_key = args[0]
            action = args[1] if len(args) > 1 else kwargs.get("action")
        else:
            node_key = kwargs.get("node") or kwargs.get("key")
            action = kwargs.get("action")

        if not action:
            return wrapped(*args, **kwargs)

        # Create wrapped node function for instrumentation
        def create_wrapped_node(original_func, node_name):
            if inspect.iscoroutinefunction(original_func):

                @wraps(original_func)
                async def wrapped_node_async(state, *node_args, **node_kwargs):
                    if context_api.get_value(context_api._SUPPRESS_INSTRUMENTATION_KEY):
                        return await original_func(state, *node_args, **node_kwargs)

                    span_name = f"invoke_agent {node_name}"
                    with tracer.start_as_current_span(
                        span_name, kind=SpanKind.CLIENT
                    ) as span:
                        start_time = time.time()
                        span.set_attribute(
                            SemanticConvention.GEN_AI_SYSTEM,
                            SemanticConvention.GEN_AI_SYSTEM_LANGGRAPH,
                        )
                        span.set_attribute(
                            SemanticConvention.GEN_AI_OPERATION,
                            SemanticConvention.GEN_AI_OPERATION_TYPE_AGENT,
                        )
                        span.set_attribute(
                            SemanticConvention.LANGGRAPH_NODE_NAME, node_name
                        )
                        span.set_attribute(
                            SemanticConvention.GEN_AI_ENVIRONMENT, environment
                        )
                        span.set_attribute(
                            SemanticConvention.GEN_AI_APPLICATION_NAME, application_name
                        )

                        try:
                            result = await original_func(
                                state, *node_args, **node_kwargs
                            )

                            # Extract LLM info from result
                            from openlit.instrumentation.langgraph.utils import (
                                extract_llm_info_from_result,
                            )

                            extract_llm_info_from_result(span, state, result)

                            from opentelemetry.trace import Status, StatusCode

                            span.set_attribute(
                                SemanticConvention.GEN_AI_CLIENT_OPERATION_DURATION,
                                time.time() - start_time,
                            )
                            span.set_status(Status(StatusCode.OK))
                            return result

                        except Exception as e:
                            handle_exception(span, e)
                            raise

                return wrapped_node_async
            else:

                @wraps(original_func)
                def wrapped_node_sync(state, *node_args, **node_kwargs):
                    if context_api.get_value(context_api._SUPPRESS_INSTRUMENTATION_KEY):
                        return original_func(state, *node_args, **node_kwargs)

                    span_name = f"invoke_agent {node_name}"
                    with tracer.start_as_current_span(
                        span_name, kind=SpanKind.CLIENT
                    ) as span:
                        start_time = time.time()
                        span.set_attribute(
                            SemanticConvention.GEN_AI_SYSTEM,
                            SemanticConvention.GEN_AI_SYSTEM_LANGGRAPH,
                        )
                        span.set_attribute(
                            SemanticConvention.GEN_AI_OPERATION,
                            SemanticConvention.GEN_AI_OPERATION_TYPE_AGENT,
                        )
                        span.set_attribute(
                            SemanticConvention.LANGGRAPH_NODE_NAME, node_name
                        )
                        span.set_attribute(
                            SemanticConvention.GEN_AI_ENVIRONMENT, environment
                        )
                        span.set_attribute(
                            SemanticConvention.GEN_AI_APPLICATION_NAME, application_name
                        )

                        try:
                            result = original_func(state, *node_args, **node_kwargs)

                            # Extract LLM info from result
                            from openlit.instrumentation.langgraph.utils import (
                                extract_llm_info_from_result,
                            )

                            extract_llm_info_from_result(span, state, result)

                            from opentelemetry.trace import Status, StatusCode

                            span.set_attribute(
                                SemanticConvention.GEN_AI_CLIENT_OPERATION_DURATION,
                                time.time() - start_time,
                            )
                            span.set_status(Status(StatusCode.OK))
                            return result

                        except Exception as e:
                            handle_exception(span, e)
                            raise

                return wrapped_node_sync

        # Wrap the action function
        node_name = str(node_key) if not isinstance(node_key, str) else node_key
        wrapped_action = create_wrapped_node(action, node_name)

        # Call original add_node with wrapped action
        if args and len(args) > 1:
            new_args = (args[0], wrapped_action) + args[2:]
            return wrapped(*new_args, **kwargs)
        else:
            kwargs["action"] = wrapped_action
            if args:
                return wrapped(*args, **kwargs)
            else:
                kwargs["node"] = node_key
                return wrapped(**kwargs)

    return wrapper
