"""
AG2 OpenTelemetry instrumentation utility functions
"""

import time

from opentelemetry.trace import Status, StatusCode

from openlit.__helpers import (
    get_chat_model_cost,
    common_span_attributes,
    record_completion_metrics,
)
from openlit.semcov import SemanticConvention


def calculate_tokens_and_cost(response, request_model, pricing_info):
    """
    Calculate the input, output tokens, and their respective costs from AG2 response.
    """
    input_tokens = 0
    output_tokens = 0

    # Early return if response doesn't have cost data
    if not hasattr(response, "cost") or response.cost is None:
        cost = get_chat_model_cost(
            request_model, pricing_info, input_tokens, output_tokens
        )
        return input_tokens, output_tokens, cost

    try:
        input_tokens, output_tokens = _extract_tokens_from_cost(response.cost)
    except (AttributeError, TypeError):
        # If theres any issue accessing cost data, default to 0 tokens
        input_tokens = 0
        output_tokens = 0

    cost = get_chat_model_cost(request_model, pricing_info, input_tokens, output_tokens)
    return input_tokens, output_tokens, cost


def _extract_tokens_from_cost(cost_data):
    """
    Extract input and output tokens from AG2 cost data structure.
    """
    input_tokens = 0
    output_tokens = 0

    for usage_data in cost_data.values():
        if not isinstance(usage_data, dict):
            continue

        for model_data in usage_data.values():
            if isinstance(model_data, dict):
                input_tokens += model_data.get("prompt_tokens", 0)
                output_tokens += model_data.get("completion_tokens", 0)

    return input_tokens, output_tokens


def format_content(chat_history):
    """
    Format the chat history into a string for span events.
    """
    if not chat_history:
        return ""

    formatted_messages = []
    for chat in chat_history:
        role = chat.get("role", "user")
        content = chat.get("content", "")
        formatted_messages.append(f"{role}: {content}")

    return "\n".join(formatted_messages)


def common_agent_logic(
    scope,
    pricing_info,
    environment,
    application_name,
    metrics,
    capture_message_content,
    disable_metrics,
    version,
    operation_type,
):
    """
    Process agent request and generate Telemetry
    """

    # Common Span Attributes
    common_span_attributes(
        scope,
        operation_type,
        SemanticConvention.GEN_AI_SYSTEM_AG2,
        scope._server_address,
        scope._server_port,
        scope._request_model,
        scope._response_model,
        environment,
        application_name,
        False,
        0,
        scope._end_time - scope._start_time,
        version,
    )

    # Span Attributes for Agent-specific parameters
    scope._span.set_attribute(SemanticConvention.GEN_AI_AGENT_NAME, scope._agent_name)

    # Span Attributes for Response parameters
    if hasattr(scope, "_input_tokens"):
        scope._span.set_attribute(
            SemanticConvention.GEN_AI_USAGE_INPUT_TOKENS, scope._input_tokens
        )
        scope._span.set_attribute(
            SemanticConvention.GEN_AI_USAGE_OUTPUT_TOKENS, scope._output_tokens
        )
        scope._span.set_attribute(
            SemanticConvention.GEN_AI_CLIENT_TOKEN_USAGE,
            scope._input_tokens + scope._output_tokens,
        )
        scope._span.set_attribute(SemanticConvention.GEN_AI_USAGE_COST, scope._cost)

    # Span Attributes for Content
    if capture_message_content and hasattr(scope, "_chat_history"):
        chat_content = format_content(scope._chat_history)
        scope._span.set_attribute(
            SemanticConvention.GEN_AI_CONTENT_COMPLETION, chat_content
        )

        # To be removed once the change to span_attributes (from span events) is complete
        scope._span.add_event(
            name=SemanticConvention.GEN_AI_CONTENT_COMPLETION_EVENT,
            attributes={
                SemanticConvention.GEN_AI_CONTENT_COMPLETION: chat_content,
            },
        )

    # Set agent description for create agent operation
    if hasattr(scope, "_system_message"):
        scope._span.set_attribute(
            SemanticConvention.GEN_AI_AGENT_DESCRIPTION, scope._system_message
        )

    scope._span.set_status(Status(StatusCode.OK))

    # Metrics
    if not disable_metrics and metrics is not None and hasattr(scope, "_input_tokens"):
        record_completion_metrics(
            metrics,
            operation_type,
            SemanticConvention.GEN_AI_SYSTEM_AG2,
            scope._server_address,
            scope._server_port,
            scope._request_model,
            scope._response_model,
            environment,
            application_name,
            scope._start_time,
            scope._end_time,
            scope._input_tokens,
            scope._output_tokens,
            scope._cost,
            0,
            scope._end_time - scope._start_time,
        )


def process_agent_creation(
    agent_name,
    llm_config,
    system_message,
    pricing_info,
    server_port,
    server_address,
    environment,
    application_name,
    metrics,
    start_time,
    span,
    capture_message_content=False,
    disable_metrics=False,
    version="1.0.0",
    **kwargs,
):
    """
    Process agent creation and generate Telemetry
    """

    # Create scope object
    scope = type("GenericScope", (), {})()

    scope._start_time = start_time
    scope._end_time = time.time()
    scope._span = span
    scope._agent_name = agent_name
    scope._request_model = llm_config.get("model", "unknown")
    scope._response_model = scope._request_model
    scope._system_message = system_message
    scope._server_address, scope._server_port = server_address, server_port

    common_agent_logic(
        scope,
        pricing_info,
        environment,
        application_name,
        metrics,
        capture_message_content,
        disable_metrics,
        version,
        SemanticConvention.GEN_AI_OPERATION_TYPE_CREATE_AGENT,
    )


def process_agent_run(
    response,
    agent_name,
    request_model,
    pricing_info,
    server_port,
    server_address,
    environment,
    application_name,
    metrics,
    start_time,
    span,
    capture_message_content=False,
    disable_metrics=False,
    version="1.0.0",
    **kwargs,
):
    """
    Process agent run and generate Telemetry
    """

    # Create scope object
    scope = type("GenericScope", (), {})()

    scope._start_time = start_time
    scope._end_time = time.time()
    scope._span = span
    scope._agent_name = agent_name
    scope._request_model = request_model
    scope._chat_history = getattr(response, "chat_history", [])
    scope._server_address, scope._server_port = server_address, server_port

    # Calculate tokens and cost
    scope._input_tokens, scope._output_tokens, scope._cost = calculate_tokens_and_cost(
        response, request_model, pricing_info
    )

    # Extract response model from cost data
    try:
        if hasattr(response, "cost") and response.cost is not None:
            cost_data = response.cost.get("usage_including_cached_inference", {})
            scope._response_model = (
                list(cost_data.keys())[1] if len(cost_data) > 1 else request_model
            )
        else:
            scope._response_model = request_model
    except (AttributeError, IndexError, KeyError, TypeError):
        scope._response_model = request_model

    common_agent_logic(
        scope,
        pricing_info,
        environment,
        application_name,
        metrics,
        capture_message_content,
        disable_metrics,
        version,
        SemanticConvention.GEN_AI_OPERATION_TYPE_EXECUTE_AGENT_TASK,
    )

    return response


def process_agent_generate_reply(
    response,
    agent_name,
    request_model,
    messages,
    sender,
    pricing_info,
    server_port,
    server_address,
    environment,
    application_name,
    metrics,
    start_time,
    span,
    capture_message_content=False,
    disable_metrics=False,
    version="1.0.0",
    **kwargs,
):
    """
    Process agent generate_reply and generate Telemetry
    """

    # Create scope object
    scope = type("GenericScope", (), {})()

    scope._start_time = start_time
    scope._end_time = time.time()
    scope._span = span
    scope._agent_name = agent_name
    scope._request_model = request_model
    scope._response_model = request_model
    scope._server_address, scope._server_port = server_address, server_port
    scope._messages = messages
    scope._sender_name = getattr(sender, "name", "Unknown") if sender else "Unknown"

    # Set agent-specific attributes
    span.set_attribute(SemanticConvention.GEN_AI_AGENT_MESSAGE_TYPE, "generate_reply")
    span.set_attribute(SemanticConvention.GEN_AI_AGENT_SENDER, scope._sender_name)

    # Process response content
    if response and isinstance(response, str):
        scope._response_content = response
    elif response and hasattr(response, "content"):
        scope._response_content = response.content
    else:
        scope._response_content = str(response) if response else ""

    # Try to extract token information if available
    try:
        # Mock token calculation for generate_reply
        scope._input_tokens = len(str(messages)) // 4 if messages else 0
        scope._output_tokens = len(scope._response_content) // 4
        scope._cost = get_chat_model_cost(
            request_model, pricing_info, scope._input_tokens, scope._output_tokens
        )
    except Exception:
        scope._input_tokens = 0
        scope._output_tokens = 0
        scope._cost = 0.0

    common_agent_logic(
        scope,
        pricing_info,
        environment,
        application_name,
        metrics,
        capture_message_content,
        disable_metrics,
        version,
        SemanticConvention.GEN_AI_OPERATION_TYPE_CHAT,
    )

    return response


def process_agent_receive(
    message,
    agent_name,
    sender_name,
    agent_instance,
    pricing_info,
    server_port,
    server_address,
    environment,
    application_name,
    metrics,
    start_time,
    span,
    capture_message_content=False,
    disable_metrics=False,
    version="1.0.0",
    **kwargs,
):
    """
    Process agent receive and generate Telemetry
    """

    # Create scope object
    scope = type("GenericScope", (), {})()

    scope._start_time = start_time
    scope._end_time = time.time()
    scope._span = span
    scope._agent_name = agent_name
    scope._sender_name = sender_name
    scope._server_address, scope._server_port = server_address, server_port
    scope._message = message

    # Extract model from agent instance
    if hasattr(agent_instance, "llm_config") and isinstance(
        agent_instance.llm_config, dict
    ):
        scope._request_model = agent_instance.llm_config.get("model", "unknown")
    else:
        scope._request_model = "unknown"
    scope._response_model = scope._request_model

    # Set agent-specific attributes
    span.set_attribute(SemanticConvention.GEN_AI_AGENT_MESSAGE_TYPE, "receive")
    span.set_attribute(SemanticConvention.GEN_AI_AGENT_SENDER, sender_name)

    # Content capture for received message
    if capture_message_content:
        span.set_attribute(SemanticConvention.GEN_AI_CONTENT_PROMPT, str(message))

    common_agent_logic(
        scope,
        pricing_info,
        environment,
        application_name,
        metrics,
        capture_message_content,
        disable_metrics,
        version,
        SemanticConvention.GEN_AI_OPERATION_TYPE_CHAT,
    )


def process_agent_send(
    message,
    agent_name,
    recipient_name,
    agent_instance,
    pricing_info,
    server_port,
    server_address,
    environment,
    application_name,
    metrics,
    start_time,
    span,
    capture_message_content=False,
    disable_metrics=False,
    version="1.0.0",
    **kwargs,
):
    """
    Process agent send and generate Telemetry
    """

    # Create scope object
    scope = type("GenericScope", (), {})()

    scope._start_time = start_time
    scope._end_time = time.time()
    scope._span = span
    scope._agent_name = agent_name
    scope._recipient_name = recipient_name
    scope._server_address, scope._server_port = server_address, server_port
    scope._message = message

    # Extract model from agent instance
    if hasattr(agent_instance, "llm_config") and isinstance(
        agent_instance.llm_config, dict
    ):
        scope._request_model = agent_instance.llm_config.get("model", "unknown")
    else:
        scope._request_model = "unknown"
    scope._response_model = scope._request_model

    # Set agent-specific attributes
    span.set_attribute(SemanticConvention.GEN_AI_AGENT_MESSAGE_TYPE, "send")
    span.set_attribute(SemanticConvention.GEN_AI_AGENT_RECIPIENT, recipient_name)

    # Content capture for sent message
    if capture_message_content:
        span.set_attribute(SemanticConvention.GEN_AI_CONTENT_COMPLETION, str(message))

    common_agent_logic(
        scope,
        pricing_info,
        environment,
        application_name,
        metrics,
        capture_message_content,
        disable_metrics,
        version,
        SemanticConvention.GEN_AI_OPERATION_TYPE_CHAT,
    )


def process_groupchat_operation(
    group_name,
    participants,
    messages,
    sender,
    max_turns,
    request_model,
    pricing_info,
    server_port,
    server_address,
    environment,
    application_name,
    metrics,
    start_time,
    span,
    capture_message_content=False,
    disable_metrics=False,
    version="1.0.0",
    **kwargs,
):
    """
    Process GroupChat operation and generate Telemetry
    """

    # Create scope object
    scope = type("GenericScope", (), {})()

    scope._start_time = start_time
    scope._end_time = time.time()
    scope._span = span
    scope._group_name = group_name
    scope._participants = participants
    scope._server_address, scope._server_port = server_address, server_port
    scope._sender_name = getattr(sender, "name", "Unknown") if sender else "Unknown"

    # Add required model attributes for common_agent_logic
    scope._request_model = request_model
    scope._response_model = request_model

    # Set agent name for groupchat
    scope._agent_name = group_name

    # Set GroupChat-specific attributes
    span.set_attribute(
        SemanticConvention.GEN_AI_GROUPCHAT_PARTICIPANTS, ",".join(participants)
    )
    span.set_attribute(
        SemanticConvention.GEN_AI_WORKFLOW_AGENT_COUNT, len(participants)
    )
    span.set_attribute(SemanticConvention.GEN_AI_WORKFLOW_EXECUTION_TYPE, "groupchat")

    if max_turns:
        span.set_attribute(SemanticConvention.GEN_AI_GROUPCHAT_TURN_COUNT, max_turns)

    # Content capture for GroupChat
    if capture_message_content and messages:
        span.set_attribute(SemanticConvention.GEN_AI_CONTENT_PROMPT, str(messages))

    # Use framework operation type for GroupChat
    common_agent_logic(
        scope,
        pricing_info,
        environment,
        application_name,
        metrics,
        capture_message_content,
        disable_metrics,
        version,
        SemanticConvention.GEN_AI_OPERATION_TYPE_FRAMEWORK,
    )


def process_speaker_selection(
    last_speaker,
    selected_speaker,
    selector,
    agents,
    request_model,
    pricing_info,
    server_port,
    server_address,
    environment,
    application_name,
    metrics,
    start_time,
    span,
    capture_message_content=False,
    disable_metrics=False,
    version="1.0.0",
    **kwargs,
):
    """
    Process speaker selection and generate Telemetry
    """

    # Create scope object
    scope = type("GenericScope", (), {})()

    scope._start_time = start_time
    scope._end_time = time.time()
    scope._span = span
    scope._last_speaker = last_speaker
    scope._selected_speaker = selected_speaker
    scope._server_address, scope._server_port = server_address, server_port

    # Add required model attributes for common_agent_logic
    scope._request_model = request_model
    scope._response_model = request_model

    # Set agent name for speaker selection
    scope._agent_name = "speaker_selection"

    # Set speaker selection attributes
    span.set_attribute(
        SemanticConvention.GEN_AI_GROUPCHAT_SPEAKER_SELECTION, selected_speaker
    )
    span.set_attribute(SemanticConvention.GEN_AI_AGENT_SENDER, last_speaker)

    if selector:
        span.set_attribute(SemanticConvention.GEN_AI_AGENT_ROLE, "selector")

    # Set agent count
    if agents:
        span.set_attribute(SemanticConvention.GEN_AI_WORKFLOW_AGENT_COUNT, len(agents))

    # Use agent operation type for speaker selection
    common_agent_logic(
        scope,
        pricing_info,
        environment,
        application_name,
        metrics,
        capture_message_content,
        disable_metrics,
        version,
        SemanticConvention.GEN_AI_OPERATION_TYPE_AGENT,
    )
