# pylint: disable=bare-except, broad-exception-caught
"""
This module has functions to calculate model costs based on tokens and to fetch pricing information.
"""

import os
import json
import logging
from urllib.parse import urlparse
from typing import Any, Dict, List, Tuple
import math
import requests
from opentelemetry.sdk.resources import (
    SERVICE_NAME,
    TELEMETRY_SDK_NAME,
    DEPLOYMENT_ENVIRONMENT,
)
from opentelemetry.trace import Status, StatusCode
from opentelemetry._events import Event
from openlit.semcov import SemanticConvention

# Set up logging
logger = logging.getLogger(__name__)


def parse_exporters(env_var_name):
    """
    Parse comma-separated exporter names from environment variable.
    Returns None if not set (signals to use default behavior).

    Args:
        env_var_name: Name of the environment variable to parse

    Returns:
        List of exporter names (lowercase, stripped) or None if env var not set
    """
    exporters_str = os.getenv(env_var_name)
    if not exporters_str:
        return None
    return [e.strip().lower() for e in exporters_str.split(",") if e.strip()]


def response_as_dict(response):
    """
    Return parsed response as a dict
    """

    # pylint: disable=no-else-return
    if isinstance(response, dict):
        return response
    if hasattr(response, "model_dump"):
        return response.model_dump()
    elif hasattr(response, "parse"):
        return response_as_dict(response.parse())
    else:
        return response


def get_env_variable(name, arg_value, error_message):
    """
    Retrieve an environment variable if the argument is not provided
    """

    if arg_value is not None:
        return arg_value
    value = os.getenv(name)
    if not value:
        logging.error(error_message)
        raise RuntimeError(error_message)
    return value


def general_tokens(text):
    """
    Calculate the number of tokens a given text would take up.
    """

    return math.ceil(len(text) / 2)


def get_chat_model_cost(model, pricing_info, prompt_tokens, completion_tokens):
    """
    Retrieve the cost of processing for a given model based on prompt and tokens.
    """

    try:
        cost = ((prompt_tokens / 1000) * pricing_info["chat"][model]["promptPrice"]) + (
            (completion_tokens / 1000) * pricing_info["chat"][model]["completionPrice"]
        )
    except:
        cost = 0
    return cost


def get_embed_model_cost(model, pricing_info, prompt_tokens):
    """
    Retrieve the cost of processing for a given model based on prompt tokens.
    """

    try:
        cost = (prompt_tokens / 1000) * pricing_info["embeddings"][model]
    except:
        cost = 0
    return cost


def get_image_model_cost(model, pricing_info, size, quality):
    """
    Retrieve the cost of processing for a given model based on image size and quailty.
    """

    try:
        cost = pricing_info["images"][model][quality][size]
    except:
        cost = 0
    return cost


def get_audio_model_cost(model, pricing_info, prompt, duration=None):
    """
    Retrieve the cost of processing for a given model based on prompt.
    """

    try:
        if prompt:
            cost = (len(prompt) / 1000) * pricing_info["audio"][model]
        else:
            cost = duration * pricing_info["audio"][model]
    except:
        cost = 0
    return cost


def fetch_pricing_info(pricing_json=None):
    """
    Fetches pricing information from a specified URL or File Path.
    """

    if pricing_json:
        is_url = urlparse(pricing_json).scheme != ""
        if is_url:
            pricing_url = pricing_json
        else:
            try:
                with open(pricing_json, mode="r", encoding="utf-8") as f:
                    return json.load(f)
            except FileNotFoundError:
                logger.error("Pricing information file not found: %s", pricing_json)
            except json.JSONDecodeError:
                logger.error("Error decoding JSON from file: %s", pricing_json)
            except Exception as file_err:
                logger.error(
                    "Unexpected error occurred while reading file: %s", file_err
                )
            return {}
    else:
        pricing_url = (
            "https://raw.githubusercontent.com/openlit/openlit/main/assets/pricing.json"
        )
    try:
        # Set a timeout of 10 seconds for both the connection and the read
        response = requests.get(pricing_url, timeout=20)
        response.raise_for_status()
        return response.json()
    except requests.HTTPError as http_err:
        logger.error("HTTP error occured while fetching pricing info: %s", http_err)
    except Exception as err:
        logger.error("Unexpected error occurred while fetching pricing info: %s", err)
    return {}


def handle_exception(span, e):
    """Handles Exception when LLM Function fails or trace creation fails."""

    span.record_exception(e)
    span.set_status(Status(StatusCode.ERROR))


def calculate_ttft(timestamps: List[float], start_time: float) -> float:
    """
    Calculate the time to the first tokens.
    """

    if timestamps:
        return timestamps[0] - start_time
    return 0.0


def calculate_tbt(timestamps: List[float]) -> float:
    """
    Calculate the average time between tokens.
    """

    if len(timestamps) > 1:
        time_diffs = [
            timestamps[i] - timestamps[i - 1] for i in range(1, len(timestamps))
        ]
        return sum(time_diffs) / len(time_diffs)
    return 0.0


def create_metrics_attributes(
    service_name: str,
    deployment_environment: str,
    operation: str,
    system: str,
    request_model: str,
    server_address: str,
    server_port: int,
    response_model: str,
) -> Dict[Any, Any]:
    """
    Returns OTel metrics attributes
    """

    return {
        TELEMETRY_SDK_NAME: "openlit",
        SERVICE_NAME: service_name,
        DEPLOYMENT_ENVIRONMENT: deployment_environment,
        SemanticConvention.GEN_AI_OPERATION: operation,
        SemanticConvention.GEN_AI_SYSTEM: system,
        SemanticConvention.GEN_AI_REQUEST_MODEL: request_model,
        SemanticConvention.SERVER_ADDRESS: server_address,
        SemanticConvention.SERVER_PORT: server_port,
        SemanticConvention.GEN_AI_RESPONSE_MODEL: response_model,
    }


def create_db_metrics_attributes(
    service_name: str,
    deployment_environment: str,
    db_system: str,
    db_operation: str,
    server_address: str,
    server_port: int,
) -> Dict[Any, Any]:
    """
    Returns OTel metrics attributes for database operations
    """

    return {
        TELEMETRY_SDK_NAME: "openlit",
        SERVICE_NAME: service_name,
        DEPLOYMENT_ENVIRONMENT: deployment_environment,
        SemanticConvention.DB_SYSTEM_NAME: db_system,
        SemanticConvention.DB_OPERATION_NAME: db_operation,
        SemanticConvention.SERVER_ADDRESS: server_address,
        SemanticConvention.SERVER_PORT: server_port,
    }


def set_server_address_and_port(
    client_instance: Any, default_server_address: str, default_server_port: int
) -> Tuple[str, int]:
    """
    Determines and returns the server address and port based on the provided client's `base_url`,
    using defaults if none found or values are None.
    """

    # Try getting base_url from multiple potential attributes
    base_client = getattr(client_instance, "_client", None)
    base_url = getattr(base_client, "base_url", None)

    if not base_url:
        # Attempt to get endpoint from instance._config.endpoint if base_url is not set
        config = getattr(client_instance, "_config", None)
        base_url = getattr(config, "endpoint", None)

    if not base_url:
        # Attempt to get server_url from instance.sdk_configuration.server_url
        config = getattr(client_instance, "sdk_configuration", None)
        base_url = getattr(config, "server_url", None)

    if not base_url:
        # Attempt to get host from instance.config.host (used by Pinecone and other vector DBs)
        config = getattr(client_instance, "config", None)
        base_url = getattr(config, "host", None)

    if base_url:
        if isinstance(base_url, str):
            # Check if it's a full URL or just a hostname
            if base_url.startswith(("http://", "https://")):
                url = urlparse(base_url)
                server_address = url.hostname or default_server_address
                server_port = url.port if url.port is not None else default_server_port
            else:
                # If it's just a hostname (like Pinecone's case), use it directly
                server_address = base_url
                server_port = default_server_port
        else:  # base_url might not be a str; handle as an object.
            server_address = getattr(base_url, "host", None) or default_server_address
            port_attr = getattr(base_url, "port", None)
            server_port = port_attr if port_attr is not None else default_server_port
    else:  # no base_url or endpoint provided; use defaults.
        server_address = default_server_address
        server_port = default_server_port

    return server_address, server_port


def otel_event(name, attributes, body):
    """
    Returns an OpenTelemetry Event object
    """

    return Event(
        name=name,
        attributes=attributes,
        body=body,
    )


def extract_and_format_input(messages):
    """
    Process a list of messages to extract content and categorize
    them into fixed roles like 'user', 'assistant', 'system', 'tool'.
    """

    fixed_roles = ["user", "assistant", "system", "tool", "developer"]
    formatted_messages = {
        role_key: {"role": "", "content": ""} for role_key in fixed_roles
    }

    # Check if input is a simple string
    if isinstance(messages, str):
        formatted_messages["user"] = {"role": "user", "content": messages}
        return formatted_messages

    for message in messages:
        message = response_as_dict(message)

        role = message.get("role")
        if role not in fixed_roles:
            continue

        content = message.get("content", "")

        # Prepare content as a string, handling both list and str
        if isinstance(content, list):
            content_str = ", ".join(str(item) for item in content)
        else:
            content_str = content

        # Set the role in the formatted message and concatenate content
        if not formatted_messages[role]["role"]:
            formatted_messages[role]["role"] = role

        if formatted_messages[role]["content"]:
            formatted_messages[role]["content"] += " " + content_str
        else:
            formatted_messages[role]["content"] = content_str

    return formatted_messages


# To be removed one the change to log events (from span events) is complete
def concatenate_all_contents(formatted_messages):
    """
    Concatenate all 'content' fields into a single strin
    """
    return " ".join(
        message_data["content"]
        for message_data in formatted_messages.values()
        if message_data["content"]
    )


def format_and_concatenate(messages):
    """
    Process a list of messages to extract content, categorize them by role,
    and concatenate all 'content' fields into a single string with role: content format.
    """

    formatted_messages = {}

    # Check if input is a simple string
    if isinstance(messages, str):
        formatted_messages["user"] = {"role": "user", "content": messages}
    elif isinstance(messages, list) and all(isinstance(m, str) for m in messages):
        # If it's a list of strings, each string is 'user' input
        user_content = " ".join(messages)
        formatted_messages["user"] = {"role": "user", "content": user_content}
    else:
        for message in messages:
            message = response_as_dict(message)
            role = message.get(
                "role", "unknown"
            )  # Default to 'unknown' if no role is specified
            content = message.get("content", "")

            # Initialize role in formatted messages if not present
            if role not in formatted_messages:
                formatted_messages[role] = {"role": role, "content": ""}

            # Handle list of dictionaries in content
            if isinstance(content, list):
                content_str = []
                for item in content:
                    if isinstance(item, dict):
                        # Collect text or other attributes as needed
                        text = item.get("text", "")
                        image_url = item.get("image_url", "")
                        content_str.append(text)
                        content_str.append(image_url)
                content_str = ", ".join(filter(None, content_str))
            else:
                content_str = content

            # Concatenate content
            if formatted_messages[role]["content"]:
                formatted_messages[role]["content"] += " " + content_str
            else:
                formatted_messages[role]["content"] = content_str

    # Concatenate role and content for all messages
    return " ".join(
        f"{message_data['role']}: {message_data['content']}"
        for message_data in formatted_messages.values()
        if message_data["content"]
    )


def common_span_attributes(
    scope,
    gen_ai_operation,
    gen_ai_system,
    server_address,
    server_port,
    request_model,
    response_model,
    environment,
    application_name,
    is_stream,
    tbt,
    ttft,
    version,
):
    """
    Set common span attributes for both chat and RAG operations.
    """

    scope._span.set_attribute(TELEMETRY_SDK_NAME, "openlit")
    scope._span.set_attribute(SemanticConvention.GEN_AI_OPERATION, gen_ai_operation)
    scope._span.set_attribute(SemanticConvention.GEN_AI_SYSTEM, gen_ai_system)
    scope._span.set_attribute(SemanticConvention.SERVER_ADDRESS, server_address)
    scope._span.set_attribute(SemanticConvention.SERVER_PORT, server_port)
    if request_model:
        scope._span.set_attribute(
            SemanticConvention.GEN_AI_REQUEST_MODEL, request_model
        )
    if response_model:
        scope._span.set_attribute(
            SemanticConvention.GEN_AI_RESPONSE_MODEL, response_model
        )
    scope._span.set_attribute(DEPLOYMENT_ENVIRONMENT, environment)
    scope._span.set_attribute(SERVICE_NAME, application_name)
    scope._span.set_attribute(SemanticConvention.GEN_AI_REQUEST_IS_STREAM, is_stream)
    scope._span.set_attribute(SemanticConvention.GEN_AI_SERVER_TBT, tbt)
    scope._span.set_attribute(SemanticConvention.GEN_AI_SERVER_TTFT, ttft)
    scope._span.set_attribute(SemanticConvention.GEN_AI_SDK_VERSION, version)


def record_completion_metrics(
    metrics,
    gen_ai_operation,
    gen_ai_system,
    server_address,
    server_port,
    request_model,
    response_model,
    environment,
    application_name,
    start_time,
    end_time,
    input_tokens,
    output_tokens,
    cost,
    tbt,
    ttft,
):
    """
    Record completion metrics for the operation.
    """

    attributes = create_metrics_attributes(
        operation=gen_ai_operation,
        system=gen_ai_system,
        server_address=server_address,
        server_port=server_port,
        request_model=request_model,
        response_model=response_model,
        service_name=application_name,
        deployment_environment=environment,
    )
    metrics["genai_client_usage_tokens"].record(
        input_tokens + output_tokens, attributes
    )
    metrics["genai_client_operation_duration"].record(end_time - start_time, attributes)
    metrics["genai_server_tbt"].record(tbt, attributes)
    metrics["genai_server_ttft"].record(ttft, attributes)
    metrics["genai_requests"].add(1, attributes)
    metrics["genai_completion_tokens"].add(output_tokens, attributes)
    metrics["genai_prompt_tokens"].add(input_tokens, attributes)
    metrics["genai_cost"].record(cost, attributes)


def record_embedding_metrics(
    metrics,
    gen_ai_operation,
    gen_ai_system,
    server_address,
    server_port,
    request_model,
    response_model,
    environment,
    application_name,
    start_time,
    end_time,
    input_tokens,
    cost,
):
    """
    Record embedding-specific metrics for the operation.
    """

    attributes = create_metrics_attributes(
        operation=gen_ai_operation,
        system=gen_ai_system,
        server_address=server_address,
        server_port=server_port,
        request_model=request_model,
        response_model=response_model,
        service_name=application_name,
        deployment_environment=environment,
    )
    metrics["genai_client_usage_tokens"].record(input_tokens, attributes)
    metrics["genai_client_operation_duration"].record(end_time - start_time, attributes)
    metrics["genai_requests"].add(1, attributes)
    metrics["genai_prompt_tokens"].add(input_tokens, attributes)
    metrics["genai_cost"].record(cost, attributes)


def record_audio_metrics(
    metrics,
    gen_ai_operation,
    gen_ai_system,
    server_address,
    server_port,
    request_model,
    response_model,
    environment,
    application_name,
    start_time,
    end_time,
    cost,
):
    """
    Record audio-specific metrics for the operation.
    """

    attributes = create_metrics_attributes(
        operation=gen_ai_operation,
        system=gen_ai_system,
        server_address=server_address,
        server_port=server_port,
        request_model=request_model,
        response_model=response_model,
        service_name=application_name,
        deployment_environment=environment,
    )
    metrics["genai_client_operation_duration"].record(end_time - start_time, attributes)
    metrics["genai_requests"].add(1, attributes)
    metrics["genai_cost"].record(cost, attributes)


def record_image_metrics(
    metrics,
    gen_ai_operation,
    gen_ai_system,
    server_address,
    server_port,
    request_model,
    response_model,
    environment,
    application_name,
    start_time,
    end_time,
    cost,
):
    """
    Record image-specific metrics for the operation.
    """

    attributes = create_metrics_attributes(
        operation=gen_ai_operation,
        system=gen_ai_system,
        server_address=server_address,
        server_port=server_port,
        request_model=request_model,
        response_model=response_model,
        service_name=application_name,
        deployment_environment=environment,
    )
    metrics["genai_client_operation_duration"].record(end_time - start_time, attributes)
    metrics["genai_requests"].add(1, attributes)
    metrics["genai_cost"].record(cost, attributes)


def common_db_span_attributes(
    scope,
    db_system,
    server_address,
    server_port,
    environment,
    application_name,
    version,
):
    """
    Set common span attributes for database operations.
    """

    scope._span.set_attribute(TELEMETRY_SDK_NAME, "openlit")
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_OPERATION,
        SemanticConvention.GEN_AI_OPERATION_TYPE_VECTORDB,
    )
    scope._span.set_attribute(SemanticConvention.DB_SYSTEM_NAME, db_system)
    scope._span.set_attribute(SemanticConvention.SERVER_ADDRESS, server_address)
    scope._span.set_attribute(SemanticConvention.SERVER_PORT, server_port)
    scope._span.set_attribute(DEPLOYMENT_ENVIRONMENT, environment)
    scope._span.set_attribute(SERVICE_NAME, application_name)
    scope._span.set_attribute(SemanticConvention.DB_SDK_VERSION, version)


def common_framework_span_attributes(
    scope,
    framework_system,
    server_address,
    server_port,
    environment,
    application_name,
    version,
    endpoint,
    instance=None,
):
    """
    Set common span attributes for GenAI framework operations.
    """

    scope._span.set_attribute(TELEMETRY_SDK_NAME, "openlit")
    scope._span.set_attribute(SemanticConvention.GEN_AI_SDK_VERSION, version)
    scope._span.set_attribute(SemanticConvention.GEN_AI_SYSTEM, framework_system)
    scope._span.set_attribute(SemanticConvention.GEN_AI_OPERATION, endpoint)
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_MODEL,
        getattr(instance, "model_name", "unknown") if instance else "unknown",
    )
    scope._span.set_attribute(SemanticConvention.SERVER_ADDRESS, server_address)
    scope._span.set_attribute(SemanticConvention.SERVER_PORT, server_port)
    scope._span.set_attribute(DEPLOYMENT_ENVIRONMENT, environment)
    scope._span.set_attribute(SERVICE_NAME, application_name)
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_CLIENT_OPERATION_DURATION,
        scope._end_time - scope._start_time,
    )


def record_mcp_metrics(
    metrics,
    mcp_operation,
    mcp_method,
    mcp_transport_type,
    mcp_tool_name,
    mcp_resource_uri,
    mcp_resource_name,
    mcp_prompt_name,
    environment,
    application_name,
    start_time,
    end_time,
    request_size=None,
    response_size=None,
    is_error=False,
):
    """
    Records MCP-specific metrics for business intelligence and operational insights.

    Args:
        metrics: Dictionary of meter instruments
        mcp_operation: The MCP operation type (tools/list, tools/call, etc.)
        mcp_method: The specific MCP method name
        mcp_transport_type: Transport type (stdio, sse, websocket)
        mcp_tool_name: Tool name for tool operations
        mcp_resource_uri: Resource URI for resource operations
        mcp_resource_name: Resource name for resource operations
        mcp_prompt_name: Prompt name for prompt operations
        environment: Deployment environment
        application_name: Application name
        start_time: Operation start time
        end_time: Operation end time
        request_size: Request payload size in bytes
        response_size: Response payload size in bytes
        is_error: Whether the operation resulted in an error
    """
    if not metrics:
        return

    try:
        # Calculate operation duration
        duration = end_time - start_time

        # Enhanced attributes for all MCP metrics with full business intelligence
        enhanced_attributes = {
            TELEMETRY_SDK_NAME: "openlit",
            SERVICE_NAME: application_name,  # Server application name
            DEPLOYMENT_ENVIRONMENT: environment,
            SemanticConvention.MCP_OPERATION: mcp_operation,
            SemanticConvention.MCP_METHOD: mcp_method,
            SemanticConvention.MCP_SYSTEM: "mcp",
        }

        # Add transport type if available
        if mcp_transport_type:
            enhanced_attributes[SemanticConvention.MCP_TRANSPORT_TYPE] = str(
                mcp_transport_type
            )

        # Add tool name to ALL metrics for complete business intelligence
        if mcp_tool_name:
            enhanced_attributes[SemanticConvention.MCP_TOOL_NAME] = str(mcp_tool_name)

        # Add resource URI and name to ALL metrics when available
        if mcp_resource_uri:
            enhanced_attributes[SemanticConvention.MCP_RESOURCE_URI] = str(
                mcp_resource_uri
            )
        if mcp_resource_name:
            enhanced_attributes[SemanticConvention.MCP_RESOURCE_NAME] = str(
                mcp_resource_name
            )

        # Add prompt name to ALL metrics when available
        if mcp_prompt_name:
            enhanced_attributes[SemanticConvention.MCP_PROMPT_NAME] = str(
                mcp_prompt_name
            )

        # Add client identification when possible
        # Note: In MCP, the "server" generates metrics but serves "clients"
        # We can try to identify the client from transport or add explicit client tracking
        if mcp_transport_type == "stdio":
            # For stdio transport, client spawns server as subprocess
            enhanced_attributes[SemanticConvention.MCP_CLIENT_TYPE] = "external_spawn"
        elif mcp_transport_type in ["websocket", "sse"]:
            # For network transports, client connects to running server
            enhanced_attributes[SemanticConvention.MCP_CLIENT_TYPE] = "network_client"

        # Record general MCP request count (now with tool name when available)
        if "mcp_requests" in metrics:
            metrics["mcp_requests"].add(1, enhanced_attributes)

        # Record operation duration (now with tool name for tool-specific performance analysis)
        if "mcp_client_operation_duration" in metrics:
            metrics["mcp_client_operation_duration"].record(
                duration, enhanced_attributes
            )

        # Record request size (now with tool name for payload analysis by tool)
        if request_size and "mcp_request_size" in metrics:
            metrics["mcp_request_size"].record(request_size, enhanced_attributes)

        # Record response size (now with tool name for response size analysis by tool)
        if response_size and "mcp_response_size" in metrics:
            metrics["mcp_response_size"].record(response_size, enhanced_attributes)

        # Record transport usage (now with enhanced context)
        if "mcp_transport_usage" in metrics and mcp_transport_type:
            metrics["mcp_transport_usage"].add(1, enhanced_attributes)

        # Record tool-specific metrics (tool name already in enhanced_attributes)
        if mcp_tool_name and "mcp_tool_calls" in metrics:
            metrics["mcp_tool_calls"].add(1, enhanced_attributes)

        # Record resource-specific metrics (resource URI already in enhanced_attributes)
        if mcp_resource_uri and "mcp_resource_reads" in metrics:
            metrics["mcp_resource_reads"].add(1, enhanced_attributes)

        # Record prompt-specific metrics (prompt name already in enhanced_attributes)
        if mcp_prompt_name and "mcp_prompt_gets" in metrics:
            metrics["mcp_prompt_gets"].add(1, enhanced_attributes)

        # Record error metrics for ALL operations (1 for error, 0 for success)
        if "mcp_errors" in metrics:
            error_count = 1 if is_error else 0
            error_attributes = {
                **enhanced_attributes,
                "mcp.error": is_error,
            }
            metrics["mcp_errors"].add(error_count, error_attributes)

        # Record success rate (now with tool name for tool-specific success rate analysis)
        if "mcp_operation_success_rate" in metrics:
            success_rate = 0.0 if is_error else 1.0
            metrics["mcp_operation_success_rate"].record(
                success_rate, enhanced_attributes
            )

    except Exception:
        # Silently ignore metrics recording errors
        pass


def record_framework_metrics(
    metrics,
    gen_ai_operation,
    gen_ai_system,
    server_address,
    server_port,
    environment,
    application_name,
    start_time,
    end_time,
):
    """
    Record basic framework metrics for the operation (only gen_ai_requests counter).
    """

    attributes = create_metrics_attributes(
        operation=gen_ai_operation,
        system=gen_ai_system,
        server_address=server_address,
        server_port=server_port,
        request_model="unknown",
        response_model="unknown",
        service_name=application_name,
        deployment_environment=environment,
    )
    metrics["genai_requests"].add(1, attributes)
    metrics["genai_client_operation_duration"].record(end_time - start_time, attributes)


def record_db_metrics(
    metrics,
    db_system,
    server_address,
    server_port,
    environment,
    application_name,
    start_time,
    end_time,
    db_operation,
):
    """
    Record database-specific metrics for the operation.
    """

    attributes = create_db_metrics_attributes(
        service_name=application_name,
        deployment_environment=environment,
        db_system=db_system,
        db_operation=db_operation,
        server_address=server_address,
        server_port=server_port,
    )
    metrics["db_requests"].add(1, attributes)
    metrics["db_client_operation_duration"].record(end_time - start_time, attributes)
