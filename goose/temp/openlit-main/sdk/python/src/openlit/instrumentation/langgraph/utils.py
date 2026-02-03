"""
LangGraph utilities for comprehensive telemetry processing and business intelligence
"""

import time
import json
from opentelemetry.trace import Status, StatusCode
from openlit.__helpers import common_framework_span_attributes
from openlit.semcov import SemanticConvention


# === OPERATION MAPPING - Framework Guide Compliant ===
OPERATION_MAP = {
    # Graph Construction Operations
    "graph_init": SemanticConvention.GEN_AI_OPERATION_TYPE_FRAMEWORK,
    "graph_add_node": SemanticConvention.GEN_AI_OPERATION_TYPE_FRAMEWORK,
    "graph_add_edge": SemanticConvention.GEN_AI_OPERATION_TYPE_FRAMEWORK,
    "graph_compile": SemanticConvention.GEN_AI_OPERATION_TYPE_FRAMEWORK,
    # Graph Execution Operations
    "graph_invoke": SemanticConvention.GEN_AI_OPERATION_TYPE_GRAPH_EXECUTION,
    "graph_ainvoke": SemanticConvention.GEN_AI_OPERATION_TYPE_GRAPH_EXECUTION,
    "graph_stream": SemanticConvention.GEN_AI_OPERATION_TYPE_GRAPH_EXECUTION,
    "graph_astream": SemanticConvention.GEN_AI_OPERATION_TYPE_GRAPH_EXECUTION,
    # State Management Operations
    "graph_get_state": SemanticConvention.GEN_AI_OPERATION_TYPE_RETRIEVE,
    "graph_aget_state": SemanticConvention.GEN_AI_OPERATION_TYPE_RETRIEVE,
    # Checkpointing Operations
    "checkpoint_setup": SemanticConvention.GEN_AI_OPERATION_TYPE_FRAMEWORK,
    "checkpoint_write": SemanticConvention.GEN_AI_OPERATION_TYPE_FRAMEWORK,
    "checkpoint_read": SemanticConvention.GEN_AI_OPERATION_TYPE_RETRIEVE,
    # Node Execution
    "node_execute": SemanticConvention.GEN_AI_OPERATION_TYPE_AGENT,
}


def set_server_address_and_port(instance):
    """
    Extract server information from LangGraph instance.

    Args:
        instance: LangGraph instance (StateGraph, CompiledGraph, etc.)

    Returns:
        tuple: (server_address, server_port)
    """
    server_address = "localhost"
    server_port = 8080

    # LangGraph doesn't have a direct server, but we can try to
    # extract checkpointer connection info if available
    try:
        if hasattr(instance, "checkpointer"):
            checkpointer = instance.checkpointer
            if hasattr(checkpointer, "conn"):
                conn = checkpointer.conn
                if hasattr(conn, "info"):
                    info = conn.info
                    server_address = info.host or "localhost"
                    server_port = info.port or 5432
    except Exception:
        pass

    return server_address, server_port


def ensure_no_none_values(attributes):
    """Filter out None values from attributes dict."""
    return {k: v for k, v in attributes.items() if v is not None}


def extract_messages_from_input(input_data):
    """
    Extract messages from graph input.

    Args:
        input_data: Input data to the graph (dict or other)

    Returns:
        list: List of message objects
    """
    if isinstance(input_data, dict) and "messages" in input_data:
        return input_data["messages"]
    return []


def extract_messages_from_output(output_data):
    """
    Extract messages from graph output.

    Args:
        output_data: Output data from the graph (dict or other)

    Returns:
        list: List of message objects
    """
    if isinstance(output_data, dict) and "messages" in output_data:
        return output_data["messages"]
    return []


def get_message_content(message):
    """
    Get content string from a message object.

    Args:
        message: Message object with content attribute

    Returns:
        str: Content string
    """
    if hasattr(message, "content"):
        content = message.content
        if isinstance(content, str):
            return content
        elif isinstance(content, list):
            # Handle list of content items (multimodal)
            parts = []
            for item in content:
                if isinstance(item, str):
                    parts.append(item)
                elif isinstance(item, dict) and "text" in item:
                    parts.append(str(item["text"]))
                elif hasattr(item, "text"):
                    parts.append(str(item.text))
            return "".join(parts)
        return str(content)
    return ""


def get_message_role(message):
    """
    Get role/type from a message object.

    Args:
        message: Message object

    Returns:
        str: Role string
    """
    if hasattr(message, "role"):
        return message.role
    elif hasattr(message, "type"):
        return message.type
    elif hasattr(message, "__class__"):
        # Extract role from class name (e.g., HumanMessage -> human)
        class_name = message.__class__.__name__
        return class_name.replace("Message", "").lower()
    return "unknown"


def set_graph_attributes(span, nodes=None, edges=None):
    """
    Set graph structure attributes on span.

    Args:
        span: OpenTelemetry span
        nodes: List of node names
        edges: Set/list of edge tuples or edge strings
    """
    if nodes:
        span.set_attribute(
            SemanticConvention.LANGGRAPH_GRAPH_NODES, json.dumps(list(nodes))
        )
        span.set_attribute(SemanticConvention.LANGGRAPH_GRAPH_NODE_COUNT, len(nodes))
        # Set individual node names
        for i, node in enumerate(list(nodes)[:10]):  # Limit to first 10
            span.set_attribute(f"langgraph.node.{i}.name", str(node))

    if edges:
        edge_list = []
        for edge in edges:
            if isinstance(edge, tuple) and len(edge) >= 2:
                edge_list.append(f"{edge[0]}->{edge[1]}")
            elif isinstance(edge, str):
                edge_list.append(edge)

        span.set_attribute(
            SemanticConvention.LANGGRAPH_GRAPH_EDGES, json.dumps(edge_list)
        )
        span.set_attribute(
            SemanticConvention.LANGGRAPH_GRAPH_EDGE_COUNT, len(edge_list)
        )
        # Set individual edge info
        for i, edge in enumerate(edge_list[:10]):  # Limit to first 10
            parts = edge.split("->")
            if len(parts) == 2:
                span.set_attribute(f"langgraph.edge.{i}.source", parts[0])
                span.set_attribute(f"langgraph.edge.{i}.target", parts[1])


def extract_llm_info_from_result(span, state, result):
    """
    Extract LLM token usage, model info from node result.

    Args:
        span: OpenTelemetry span
        state: Input state dict
        result: Result from node execution
    """
    try:
        # Extract messages from state for context
        if isinstance(state, dict) and "messages" in state:
            messages = state["messages"]
            # Set prompt content from last few messages
            for i, msg in enumerate(messages[-3:]):
                content = get_message_content(msg)
                role = get_message_role(msg)
                if content:
                    span.set_attribute(f"gen_ai.prompt.{i}.content", content[:500])
                    span.set_attribute(f"gen_ai.prompt.{i}.role", role)

        # Extract from result
        if isinstance(result, dict) and "messages" in result:
            output_messages = result["messages"]
            if output_messages:
                last_msg = (
                    output_messages[-1]
                    if isinstance(output_messages, list)
                    else output_messages
                )

                # Extract model info from response_metadata
                if hasattr(last_msg, "response_metadata"):
                    metadata = last_msg.response_metadata
                    if isinstance(metadata, dict):
                        # Model name
                        model_name = metadata.get("model_name") or metadata.get("model")
                        if model_name:
                            span.set_attribute(
                                SemanticConvention.GEN_AI_REQUEST_MODEL, model_name
                            )
                            span.set_attribute(
                                SemanticConvention.GEN_AI_RESPONSE_MODEL, model_name
                            )

                        # Token usage
                        if "token_usage" in metadata:
                            usage = metadata["token_usage"]
                            if isinstance(usage, dict):
                                if "prompt_tokens" in usage:
                                    span.set_attribute(
                                        SemanticConvention.GEN_AI_USAGE_INPUT_TOKENS,
                                        usage["prompt_tokens"],
                                    )
                                if "completion_tokens" in usage:
                                    span.set_attribute(
                                        SemanticConvention.GEN_AI_USAGE_OUTPUT_TOKENS,
                                        usage["completion_tokens"],
                                    )
                                if "total_tokens" in usage:
                                    span.set_attribute(
                                        SemanticConvention.GEN_AI_USAGE_TOTAL_TOKENS,
                                        usage["total_tokens"],
                                    )

                        # Finish reason
                        if "finish_reason" in metadata:
                            span.set_attribute(
                                SemanticConvention.GEN_AI_RESPONSE_FINISH_REASON,
                                metadata["finish_reason"],
                            )

                        # Response ID
                        if "id" in metadata and metadata["id"]:
                            span.set_attribute(
                                SemanticConvention.GEN_AI_RESPONSE_ID, metadata["id"]
                            )

                # Extract content
                if hasattr(last_msg, "content"):
                    content = get_message_content(last_msg)
                    if content:
                        span.set_attribute(
                            SemanticConvention.GEN_AI_CONTENT_COMPLETION, content[:1000]
                        )

                # Extract usage_metadata (alternative location)
                if hasattr(last_msg, "usage_metadata"):
                    usage = last_msg.usage_metadata
                    if hasattr(usage, "input_tokens"):
                        span.set_attribute(
                            SemanticConvention.GEN_AI_USAGE_INPUT_TOKENS,
                            usage.input_tokens,
                        )
                    if hasattr(usage, "output_tokens"):
                        span.set_attribute(
                            SemanticConvention.GEN_AI_USAGE_OUTPUT_TOKENS,
                            usage.output_tokens,
                        )
                    if hasattr(usage, "total_tokens"):
                        span.set_attribute(
                            SemanticConvention.GEN_AI_USAGE_TOTAL_TOKENS,
                            usage.total_tokens,
                        )

                # Tool calls
                if hasattr(last_msg, "tool_calls") and last_msg.tool_calls:
                    tool_calls_info = []
                    for j, tool_call in enumerate(last_msg.tool_calls[:5]):
                        tool_info = {}
                        if hasattr(tool_call, "name"):
                            tool_info["name"] = tool_call.name
                            span.set_attribute(
                                f"gen_ai.tool_call.{j}.name", tool_call.name
                            )
                        if hasattr(tool_call, "args"):
                            args_str = (
                                json.dumps(tool_call.args)
                                if isinstance(tool_call.args, dict)
                                else str(tool_call.args)
                            )
                            span.set_attribute(
                                f"gen_ai.tool_call.{j}.arguments", args_str[:500]
                            )
                        if hasattr(tool_call, "id"):
                            span.set_attribute(f"gen_ai.tool_call.{j}.id", tool_call.id)
                        if tool_info:
                            tool_calls_info.append(tool_info)

                    if tool_calls_info:
                        span.set_attribute(
                            SemanticConvention.GEN_AI_TOOL_CALLS,
                            json.dumps(tool_calls_info),
                        )

    except Exception:
        # Don't fail the span if we can't extract info
        pass


def extract_config_info(config):
    """
    Extract configuration information from RunnableConfig.

    Args:
        config: RunnableConfig dict

    Returns:
        dict: Extracted config info (thread_id, checkpoint_id, etc.)
    """
    info = {}
    if not config:
        return info

    try:
        # Extract configurable values
        configurable = config.get("configurable", {})

        if "thread_id" in configurable:
            info["thread_id"] = str(configurable["thread_id"])

        if "checkpoint_id" in configurable:
            info["checkpoint_id"] = str(configurable["checkpoint_id"])

        if "checkpoint_ns" in configurable:
            info["checkpoint_ns"] = str(configurable["checkpoint_ns"])

    except Exception:
        pass

    return info


def generate_span_name(operation_type, endpoint, instance=None, args=None, kwargs=None):
    """
    Generate proper span name following OpenTelemetry conventions.

    Args:
        operation_type: Standard operation type from OPERATION_MAP
        endpoint: Endpoint name (e.g., "graph_invoke")
        instance: The instance being instrumented
        args: Positional arguments
        kwargs: Keyword arguments

    Returns:
        str: Span name
    """
    # Graph execution operations
    if endpoint in ("graph_invoke", "graph_ainvoke"):
        graph_name = _get_graph_name(instance)
        return f"{operation_type} {graph_name}"

    elif endpoint in ("graph_stream", "graph_astream"):
        graph_name = _get_graph_name(instance)
        return f"{operation_type} {graph_name} stream"

    # State operations
    elif endpoint in ("graph_get_state", "graph_aget_state"):
        return "retrieve graph_state"

    # Graph construction
    elif endpoint == "graph_init":
        return f"{operation_type} graph_init"

    elif endpoint == "graph_compile":
        return f"{operation_type} graph_compile"

    elif endpoint == "graph_add_node":
        node_name = args[0] if args else "node"
        return f"{operation_type} add_node:{node_name}"

    elif endpoint == "graph_add_edge":
        return f"{operation_type} add_edge"

    # Node execution
    elif endpoint == "node_execute":
        node_name = kwargs.get("node_name", "node") if kwargs else "node"
        return f"invoke_agent {node_name}"

    # Checkpointing
    elif endpoint == "checkpoint_setup":
        return f"{operation_type} checkpoint_setup"

    elif endpoint == "checkpoint_write":
        return f"{operation_type} checkpoint_write"

    elif endpoint == "checkpoint_read":
        return "retrieve checkpoint"

    # Default
    return f"{operation_type} {endpoint}"


def _get_graph_name(instance):
    """Get the name of a graph instance."""
    if instance is None:
        return "graph"

    # Try to get name from instance
    name = getattr(instance, "name", None)
    if name:
        return str(name)

    # Try to get from graph_id
    graph_id = getattr(instance, "graph_id", None)
    if graph_id:
        return str(graph_id)

    # Try class name
    class_name = instance.__class__.__name__
    if class_name not in ("Pregel", "CompiledStateGraph", "StateGraph"):
        return class_name

    return "graph"


def process_langgraph_response(
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
    endpoint=None,
    **kwargs,
):
    """
    Process LangGraph response with comprehensive business intelligence.

    Args:
        response: The response from the LangGraph operation
        operation_type: Type of operation performed
        server_address: Server address
        server_port: Server port
        environment: Deployment environment
        application_name: Application name
        metrics: Metrics registry
        start_time: Operation start time
        span: OpenTelemetry span
        capture_message_content: Whether to capture message content
        disable_metrics: Whether metrics are disabled
        version: SDK version
        instance: LangGraph instance
        args: Positional arguments
        endpoint: Endpoint name
        **kwargs: Additional keyword arguments

    Returns:
        The original response
    """
    end_time = time.time()

    # Create scope for common_framework_span_attributes
    scope = type("GenericScope", (), {})()
    scope._span = span
    scope._start_time = start_time
    scope._end_time = end_time

    # Get standard operation name
    standard_operation = OPERATION_MAP.get(
        endpoint, SemanticConvention.GEN_AI_OPERATION_TYPE_FRAMEWORK
    )

    # Set common framework attributes
    common_framework_span_attributes(
        scope,
        SemanticConvention.GEN_AI_SYSTEM_LANGGRAPH,
        server_address,
        server_port,
        environment,
        application_name,
        version,
        endpoint,
        instance,
    )

    # Set operation name
    span.set_attribute(SemanticConvention.GEN_AI_OPERATION, standard_operation)

    # Set execution mode
    if endpoint in ("graph_invoke", "graph_ainvoke"):
        span.set_attribute(SemanticConvention.LANGGRAPH_EXECUTION_MODE, "invoke")
    elif endpoint in ("graph_stream", "graph_astream"):
        span.set_attribute(SemanticConvention.LANGGRAPH_EXECUTION_MODE, "stream")

    # Extract config information
    config = kwargs.get("config") or (args[1] if len(args) > 1 else None)
    if config:
        config_info = extract_config_info(config)
        if config_info.get("thread_id"):
            span.set_attribute(
                SemanticConvention.LANGGRAPH_THREAD_ID, config_info["thread_id"]
            )
        if config_info.get("checkpoint_id"):
            span.set_attribute(
                SemanticConvention.LANGGRAPH_CHECKPOINT_ID, config_info["checkpoint_id"]
            )

    # Process response based on type
    if endpoint in ("graph_invoke", "graph_ainvoke"):
        _process_invoke_response(span, response, capture_message_content)
    elif endpoint in ("graph_get_state", "graph_aget_state"):
        _process_state_response(span, response)
    elif endpoint == "graph_compile":
        _process_compile_response(span, instance, response)

    # Set success status
    span.set_attribute(SemanticConvention.LANGGRAPH_GRAPH_STATUS, "success")
    span.set_status(Status(StatusCode.OK))

    # Record metrics
    if not disable_metrics and metrics:
        _record_langgraph_metrics(
            metrics,
            standard_operation,
            end_time - start_time,
            environment,
            application_name,
        )

    return response


def _process_invoke_response(span, response, capture_message_content):
    """Process invoke/ainvoke response."""
    try:
        if isinstance(response, dict):
            # Extract messages
            messages = extract_messages_from_output(response)
            if messages:
                span.set_attribute(
                    SemanticConvention.LANGGRAPH_MESSAGE_COUNT, len(messages)
                )

                # Get final response content
                if messages:
                    last_msg = messages[-1] if isinstance(messages, list) else messages
                    content = get_message_content(last_msg)
                    if content and capture_message_content:
                        span.set_attribute(
                            SemanticConvention.LANGGRAPH_FINAL_RESPONSE, content[:500]
                        )
                        span.set_attribute(
                            SemanticConvention.GEN_AI_CONTENT_COMPLETION, content[:1000]
                        )

            # Try to extract LLM info
            extract_llm_info_from_result(span, {}, response)

    except Exception:
        pass


def _process_state_response(span, response):
    """Process get_state/aget_state response."""
    try:
        if hasattr(response, "values"):
            values = response.values
            if isinstance(values, dict) and "messages" in values:
                span.set_attribute(
                    SemanticConvention.LANGGRAPH_MESSAGE_COUNT, len(values["messages"])
                )

        if hasattr(response, "next"):
            next_nodes = response.next
            if next_nodes:
                span.set_attribute(
                    "langgraph.state.next_nodes", json.dumps(list(next_nodes))
                )

    except Exception:
        pass


def _process_compile_response(span, instance, response):
    """Process compile response."""
    try:
        # Extract nodes and edges from the graph
        nodes = []
        edges = []

        if hasattr(instance, "nodes"):
            nodes = (
                list(instance.nodes.keys()) if hasattr(instance.nodes, "keys") else []
            )

        if hasattr(instance, "edges"):
            edge_set = instance.edges
            if hasattr(edge_set, "items"):
                for source, targets in edge_set.items():
                    if isinstance(targets, dict):
                        for target in targets.values():
                            edges.append(f"{source}->{target}")
                    elif isinstance(targets, (list, set)):
                        for target in targets:
                            edges.append(f"{source}->{target}")
            elif hasattr(edge_set, "__iter__"):
                for edge in edge_set:
                    if isinstance(edge, tuple) and len(edge) >= 2:
                        edges.append(f"{edge[0]}->{edge[1]}")

        set_graph_attributes(span, nodes, edges)

    except Exception:
        pass


def _record_langgraph_metrics(
    metrics, operation_type, duration, environment, application_name
):
    """Record LangGraph metrics."""
    try:
        attributes = {
            "gen_ai.operation.name": operation_type,
            "gen_ai.system": SemanticConvention.GEN_AI_SYSTEM_LANGGRAPH,
            "service.name": application_name,
            "deployment.environment": environment,
        }

        # Record operation duration
        if "genai_client_operation_duration" in metrics:
            metrics["genai_client_operation_duration"].record(duration, attributes)

        # Record operation count
        if "genai_requests" in metrics:
            metrics["genai_requests"].add(1, attributes)

    except Exception:
        pass
