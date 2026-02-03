"""
Haystack utilities
"""

import time
import json
from typing import Dict, Any
from opentelemetry.trace import Status, StatusCode
from openlit.__helpers import common_framework_span_attributes, record_framework_metrics
from openlit.semcov import SemanticConvention

# Optimized operation mapping - minimal and fast
OPERATION_MAP = {
    "pipeline": SemanticConvention.GEN_AI_OPERATION_TYPE_FRAMEWORK,
    "bm25_retriever": SemanticConvention.GEN_AI_OPERATION_TYPE_RETRIEVE,
    "prompt_builder": SemanticConvention.GEN_AI_OPERATION_TYPE_FRAMEWORK,
    "openai_generator": SemanticConvention.GEN_AI_OPERATION_TYPE_CHAT,
    "openai_chat_generator": SemanticConvention.GEN_AI_OPERATION_TYPE_CHAT,
    "text_embedder": SemanticConvention.GEN_AI_OPERATION_TYPE_EMBEDDING,
    "document_embedder": SemanticConvention.GEN_AI_OPERATION_TYPE_EMBEDDING,
}


def set_server_address_and_port(instance):
    """Fast server address extraction"""
    return "localhost", 8080


def object_count(obj):
    """Fast object counting"""
    try:
        return len(obj) if hasattr(obj, "__len__") else 1
    except:
        return 1


def extract_component_technical_details(
    instance, args, kwargs, endpoint
) -> Dict[str, Any]:
    """Extract comprehensive component technical details with performance optimization"""
    details = {}

    try:
        # Component class information
        if hasattr(instance, "__class__"):
            details["class_name"] = instance.__class__.__name__
            details["module_name"] = instance.__class__.__module__

        # Component input type extraction (optimized)
        if hasattr(instance, "_component_config") and hasattr(
            instance._component_config, "input_types"
        ):
            input_types = {}
            for name, type_info in instance._component_config.input_types.items():
                input_types[name] = str(type_info) if type_info else "Any"
            details["input_types"] = input_types
        elif hasattr(instance, "run") and hasattr(instance.run, "__annotations__"):
            # Fallback: extract from method annotations
            annotations = instance.run.__annotations__
            input_types = {k: str(v) for k, v in annotations.items() if k != "return"}
            details["input_types"] = input_types

        # Component output type extraction (optimized)
        if hasattr(instance, "_component_config") and hasattr(
            instance._component_config, "output_types"
        ):
            output_types = {}
            for name, type_info in instance._component_config.output_types.items():
                output_types[name] = str(type_info) if type_info else "Any"
            details["output_types"] = output_types

        # Enhanced input/output specifications with connections
        if hasattr(instance, "_component_config"):
            config = instance._component_config

            # Input specifications with data flow
            if hasattr(config, "input_sockets"):
                input_spec = {}
                for socket_name, socket in config.input_sockets.items():
                    spec_info = {
                        "type": str(getattr(socket, "type", "Any")),
                        "default_value": str(getattr(socket, "default_value", None)),
                        "is_optional": getattr(socket, "is_optional", False),
                    }
                    input_spec[socket_name] = spec_info
                details["input_spec"] = input_spec

            # Output specifications with receivers
            if hasattr(config, "output_sockets"):
                output_spec = {}
                for socket_name, socket in config.output_sockets.items():
                    spec_info = {
                        "type": str(getattr(socket, "type", "Any")),
                        "is_list": getattr(socket, "is_list", False),
                    }
                    output_spec[socket_name] = spec_info
                details["output_spec"] = output_spec

        # Runtime input data analysis (for actual values)
        if args or kwargs:
            runtime_inputs = {}
            if args:
                for i, arg in enumerate(args):
                    runtime_inputs[f"arg_{i}"] = type(arg).__name__
            if kwargs:
                for key, value in kwargs.items():
                    runtime_inputs[key] = type(value).__name__
            details["runtime_input_types"] = runtime_inputs

    except Exception:
        # Silently continue if introspection fails - maintain performance
        pass

    return details


def extract_pipeline_metadata(instance, args, kwargs) -> Dict[str, Any]:
    """Extract pipeline-level metadata and configuration"""
    metadata = {}

    try:
        # Pipeline configuration
        if hasattr(instance, "graph"):
            graph = instance.graph
        elif hasattr(instance, "_graph"):
            graph = instance._graph

            # Component count and connections
            if hasattr(graph, "nodes"):
                metadata["component_count"] = len(graph.nodes())

                # Extract component connections and data flow
                connections = []
                if hasattr(graph, "edges"):
                    for edge in graph.edges(data=True):
                        source, target, data = edge
                        connection_info = {
                            "source": source,
                            "target": target,
                            "data": str(data) if data else None,
                        }
                        connections.append(connection_info)
                metadata["connections"] = connections

                # Component list with types
                components = []
                for node in graph.nodes():
                    node_data = (
                        graph.nodes[node] if hasattr(graph.nodes[node], "get") else {}
                    )
                    component_info = {
                        "name": node,
                        "type": str(type(node_data.get("instance", "")))
                        if node_data.get("instance")
                        else "unknown",
                    }
                    components.append(component_info)
                metadata["components"] = components

        # Pipeline configuration parameters
        if hasattr(instance, "max_runs_per_component"):
            metadata["max_runs_per_component"] = instance.max_runs_per_component

        # Input/output data (if provided)
        if args and len(args) > 0:
            # Pipeline input data
            input_data = args[0] if args else {}
            if isinstance(input_data, dict):
                # Sanitize large data for telemetry
                sanitized_input = {}
                for key, value in input_data.items():
                    if isinstance(value, (str, int, float, bool)):
                        sanitized_input[key] = value
                    elif isinstance(value, dict):
                        sanitized_input[key] = {
                            k: str(v)[:100] for k, v in value.items()
                        }
                    else:
                        sanitized_input[key] = str(type(value)).__name__
                metadata["input_data"] = sanitized_input

    except Exception:
        # Silently continue if metadata extraction fails
        pass

    return metadata


def extract_component_connections(instance) -> Dict[str, Any]:
    """Extract component connection and data flow information"""
    connections = {}

    try:
        # Extract senders (components that send data to this component)
        if hasattr(instance, "_component_config") and hasattr(
            instance._component_config, "input_sockets"
        ):
            senders = []
            for socket_name, socket in instance._component_config.input_sockets.items():
                if hasattr(socket, "_senders") and socket._senders:
                    for sender in socket._senders:
                        sender_info = {"component": str(sender), "socket": socket_name}
                        senders.append(sender_info)
            connections["senders"] = senders

        # Extract receivers (components that receive data from this component)
        if hasattr(instance, "_component_config") and hasattr(
            instance._component_config, "output_sockets"
        ):
            receivers = []
            for (
                socket_name,
                socket,
            ) in instance._component_config.output_sockets.items():
                if hasattr(socket, "_receivers") and socket._receivers:
                    for receiver in socket._receivers:
                        receiver_info = {
                            "component": str(receiver),
                            "socket": socket_name,
                        }
                        receivers.append(receiver_info)
            connections["receivers"] = receivers

    except Exception:
        # Silently continue if connection extraction fails
        pass

    return connections


def process_haystack_response(
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
    instance=None,
    args=None,
    endpoint=None,
    **kwargs,
):
    """Enhanced response processing with comprehensive technical details and optimized performance"""

    end_time = time.time()

    # Essential attributes
    common_framework_span_attributes(
        type(
            "Scope",
            (),
            {
                "_span": span,
                "_server_address": server_address,
                "_server_port": server_port,
                "_start_time": start_time,
                "_end_time": end_time,
            },
        )(),
        SemanticConvention.GEN_AI_SYSTEM_HAYSTACK,
        server_address,
        server_port,
        environment,
        application_name,
        version,
        endpoint,
        instance,
    )

    # Core operation attributes
    span.set_attribute(SemanticConvention.GEN_AI_OPERATION, operation_type)

    # Enhanced technical details collection
    if instance:
        # Extract comprehensive component technical details
        tech_details = extract_component_technical_details(
            instance, args, kwargs, endpoint
        )

        # Apply component technical attributes using new semantic conventions
        if tech_details.get("class_name"):
            span.set_attribute(
                SemanticConvention.GEN_AI_FRAMEWORK_COMPONENT_CLASS_NAME,
                tech_details["class_name"],
            )

        if tech_details.get("input_types"):
            span.set_attribute(
                SemanticConvention.GEN_AI_FRAMEWORK_COMPONENT_INPUT_TYPES,
                json.dumps(tech_details["input_types"]),
            )

        if tech_details.get("output_types"):
            span.set_attribute(
                SemanticConvention.GEN_AI_FRAMEWORK_COMPONENT_OUTPUT_TYPES,
                json.dumps(tech_details["output_types"]),
            )

        if tech_details.get("input_spec"):
            span.set_attribute(
                SemanticConvention.GEN_AI_FRAMEWORK_COMPONENT_INPUT_SPEC,
                json.dumps(tech_details["input_spec"]),
            )

        if tech_details.get("output_spec"):
            span.set_attribute(
                SemanticConvention.GEN_AI_FRAMEWORK_COMPONENT_OUTPUT_SPEC,
                json.dumps(tech_details["output_spec"]),
            )

        # Component connections and data flow
        connections = extract_component_connections(instance)
        if connections.get("senders"):
            span.set_attribute(
                SemanticConvention.GEN_AI_FRAMEWORK_COMPONENT_SENDERS,
                json.dumps(connections["senders"]),
            )

        if connections.get("receivers"):
            span.set_attribute(
                SemanticConvention.GEN_AI_FRAMEWORK_COMPONENT_RECEIVERS,
                json.dumps(connections["receivers"]),
            )

    # Enhanced telemetry - pipeline level
    if endpoint == "pipeline" and isinstance(response, dict):
        span.set_attribute(
            SemanticConvention.GEN_AI_FRAMEWORK_CONTEXT_COUNT, len(response)
        )

        # Enhanced pipeline metadata collection
        if instance:
            pipeline_metadata = extract_pipeline_metadata(instance, args, kwargs)

            # Apply pipeline metadata using new semantic conventions
            if pipeline_metadata.get("component_count"):
                span.set_attribute(
                    SemanticConvention.GEN_AI_FRAMEWORK_PIPELINE_COMPONENT_COUNT,
                    pipeline_metadata["component_count"],
                )

            if pipeline_metadata.get("max_runs_per_component"):
                span.set_attribute(
                    SemanticConvention.GEN_AI_FRAMEWORK_PIPELINE_MAX_RUNS,
                    pipeline_metadata["max_runs_per_component"],
                )

            if pipeline_metadata.get("connections"):
                span.set_attribute(
                    SemanticConvention.GEN_AI_FRAMEWORK_COMPONENT_CONNECTIONS,
                    json.dumps(pipeline_metadata["connections"]),
                )

            if pipeline_metadata.get("components"):
                span.set_attribute(
                    SemanticConvention.GEN_AI_FRAMEWORK_PIPELINE_METADATA,
                    json.dumps(pipeline_metadata["components"]),
                )

            if pipeline_metadata.get("input_data"):
                span.set_attribute(
                    SemanticConvention.GEN_AI_FRAMEWORK_PIPELINE_INPUT_DATA,
                    json.dumps(pipeline_metadata["input_data"]),
                )

        # Pipeline output data
        if response:
            # Sanitize output data for telemetry
            sanitized_output = {}
            for key, value in response.items():
                if isinstance(value, (str, int, float, bool)):
                    sanitized_output[key] = value
                elif isinstance(value, dict) and "replies" in value:
                    sanitized_output[key] = f"{len(value['replies'])} replies"
                else:
                    sanitized_output[key] = str(type(value)).__name__
            span.set_attribute(
                SemanticConvention.GEN_AI_FRAMEWORK_PIPELINE_OUTPUT_DATA,
                json.dumps(sanitized_output),
            )

        # Fast LLM response extraction
        for key, value in response.items():
            if (
                key in ["llm", "generator"]
                and isinstance(value, dict)
                and "replies" in value
            ):
                replies = value["replies"]
                if replies and capture_message_content:
                    span.set_attribute(
                        SemanticConvention.GEN_AI_CONTENT_COMPLETION,
                        str(replies[0])[:500],
                    )
                break

    # Enhanced telemetry - retriever level
    elif (
        "retriever" in endpoint
        and isinstance(response, dict)
        and "documents" in response
    ):
        docs = response["documents"]
        span.set_attribute(
            SemanticConvention.GEN_AI_FRAMEWORK_RETRIEVAL_COUNT, object_count(docs)
        )
        span.set_attribute(
            SemanticConvention.GEN_AI_FRAMEWORK_DOCUMENTS_COUNT, object_count(docs)
        )

        # Component identification
        if instance:
            span.set_attribute(
                SemanticConvention.GEN_AI_FRAMEWORK_COMPONENT_NAME, endpoint
            )
            span.set_attribute(
                SemanticConvention.GEN_AI_FRAMEWORK_COMPONENT_TYPE, "retriever"
            )

    # Enhanced telemetry - generator level
    elif "generator" in endpoint:
        # Component identification
        if instance:
            span.set_attribute(
                SemanticConvention.GEN_AI_FRAMEWORK_COMPONENT_NAME, endpoint
            )
            span.set_attribute(
                SemanticConvention.GEN_AI_FRAMEWORK_COMPONENT_TYPE, "generator"
            )

        if args and capture_message_content:
            span.set_attribute(SemanticConvention.GEN_AI_PROMPT, str(args[0])[:500])

        if isinstance(response, dict) and "replies" in response:
            replies = response["replies"]
            span.set_attribute(
                SemanticConvention.GEN_AI_FRAMEWORK_CONTEXT_COUNT, object_count(replies)
            )

    # Enhanced telemetry - prompt builder level
    elif endpoint == "prompt_builder":
        # Component identification
        if instance:
            span.set_attribute(
                SemanticConvention.GEN_AI_FRAMEWORK_COMPONENT_NAME, endpoint
            )
            span.set_attribute(
                SemanticConvention.GEN_AI_FRAMEWORK_COMPONENT_TYPE, "prompt_builder"
            )

        if kwargs and capture_message_content:
            for key, value in kwargs.items():
                if key in ["documents", "question"] and value:
                    span.set_attribute(
                        SemanticConvention.GEN_AI_FRAMEWORK_CONTEXT_COUNT,
                        object_count([value]),
                    )
                    break

    # Component visit tracking (simulate component execution count)
    if endpoint != "pipeline" and instance:
        # Simple visit counter - can be enhanced with actual state tracking
        span.set_attribute(SemanticConvention.GEN_AI_FRAMEWORK_COMPONENT_VISITS, 1)

    # Duration and status
    execution_time = end_time - start_time
    span.set_attribute(
        SemanticConvention.GEN_AI_CLIENT_OPERATION_DURATION, execution_time
    )

    # Pipeline execution time tracking
    if endpoint == "pipeline":
        span.set_attribute(
            SemanticConvention.GEN_AI_FRAMEWORK_PIPELINE_EXECUTION_TIME, execution_time
        )

    span.set_status(Status(StatusCode.OK))

    # Metrics
    if not disable_metrics:
        record_framework_metrics(
            metrics,
            operation_type,
            SemanticConvention.GEN_AI_SYSTEM_HAYSTACK,
            server_address,
            server_port,
            environment,
            application_name,
            start_time,
            end_time,
        )

    return response
