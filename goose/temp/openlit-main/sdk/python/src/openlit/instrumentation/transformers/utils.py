"""
HF Transformers OpenTelemetry instrumentation utility functions
"""

import time

from opentelemetry.trace import Status, StatusCode

from openlit.__helpers import (
    general_tokens,
    get_chat_model_cost,
    common_span_attributes,
    record_completion_metrics,
)
from openlit.semcov import SemanticConvention


def format_content(content):
    """
    Format content to a consistent structure.
    """
    if isinstance(content, str):
        return content
    elif isinstance(content, list):
        # Check if its a list of chat messages (like in the test case)
        if (
            len(content) > 0
            and isinstance(content[0], dict)
            and "role" in content[0]
            and "content" in content[0]
        ):
            # Handle chat message format like Groq
            formatted_messages = []
            for message in content:
                role = message["role"]
                msg_content = message["content"]

                if isinstance(msg_content, list):
                    content_str = ", ".join(
                        f"{item['type']}: {item['text'] if 'text' in item else item.get('image_url', str(item))}"
                        if isinstance(item, dict) and "type" in item
                        else str(item)
                        for item in msg_content
                    )
                    formatted_messages.append(f"{role}: {content_str}")
                else:
                    formatted_messages.append(f"{role}: {msg_content}")
            return "\n".join(formatted_messages)
        else:
            # Handle other list formats (transformers responses)
            formatted_content = []
            for item in content:
                if isinstance(item, str):
                    formatted_content.append(item)
                elif isinstance(item, dict):
                    # Handle dict format for transformers
                    if "generated_text" in item:
                        formatted_content.append(str(item["generated_text"]))
                    else:
                        formatted_content.append(str(item))
                else:
                    formatted_content.append(str(item))
            return " ".join(formatted_content)
    else:
        return str(content)


def common_chat_logic(
    scope,
    pricing_info,
    environment,
    application_name,
    metrics,
    capture_message_content,
    disable_metrics,
    version,
    args,
    kwargs,
    is_stream,
):
    """
    Process chat request and generate Telemetry
    """

    scope._end_time = time.time()
    forward_params = scope._instance._forward_params
    request_model = scope._instance.model.config.name_or_path

    input_tokens = general_tokens(scope._prompt)
    output_tokens = general_tokens(scope._completion)

    cost = get_chat_model_cost(request_model, pricing_info, input_tokens, output_tokens)

    # Common Span Attributes
    common_span_attributes(
        scope,
        SemanticConvention.GEN_AI_OPERATION_TYPE_CHAT,
        SemanticConvention.GEN_AI_SYSTEM_HUGGING_FACE,
        scope._server_address,
        scope._server_port,
        request_model,
        request_model,
        environment,
        application_name,
        is_stream,
        scope._tbt,
        scope._ttft,
        version,
    )

    # Set request parameters from forward_params
    if forward_params.get("temperature") is not None:
        scope._span.set_attribute(
            SemanticConvention.GEN_AI_REQUEST_TEMPERATURE, forward_params["temperature"]
        )
    if forward_params.get("top_k") is not None:
        scope._span.set_attribute(
            SemanticConvention.GEN_AI_REQUEST_TOP_K, forward_params["top_k"]
        )
    if forward_params.get("top_p") is not None:
        scope._span.set_attribute(
            SemanticConvention.GEN_AI_REQUEST_TOP_P, forward_params["top_p"]
        )
    if forward_params.get("max_length") is not None:
        scope._span.set_attribute(
            SemanticConvention.GEN_AI_REQUEST_MAX_TOKENS, forward_params["max_length"]
        )

    # Set token usage and cost attributes
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_USAGE_INPUT_TOKENS, input_tokens
    )
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_USAGE_OUTPUT_TOKENS, output_tokens
    )
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_CLIENT_TOKEN_USAGE, input_tokens + output_tokens
    )
    scope._span.set_attribute(SemanticConvention.GEN_AI_USAGE_COST, cost)

    # Span Attributes for Content
    if capture_message_content:
        scope._span.set_attribute(
            SemanticConvention.GEN_AI_CONTENT_PROMPT, scope._prompt
        )
        scope._span.set_attribute(
            SemanticConvention.GEN_AI_CONTENT_COMPLETION, scope._completion
        )

        # To be removed once the change to span_attributes (from span events) is complete
        scope._span.add_event(
            name=SemanticConvention.GEN_AI_CONTENT_PROMPT_EVENT,
            attributes={
                SemanticConvention.GEN_AI_CONTENT_PROMPT: scope._prompt,
            },
        )
        scope._span.add_event(
            name=SemanticConvention.GEN_AI_CONTENT_COMPLETION_EVENT,
            attributes={
                SemanticConvention.GEN_AI_CONTENT_COMPLETION: scope._completion,
            },
        )

    scope._span.set_status(Status(StatusCode.OK))

    # Record metrics using the standardized helper function
    if not disable_metrics:
        record_completion_metrics(
            metrics,
            SemanticConvention.GEN_AI_OPERATION_TYPE_CHAT,
            SemanticConvention.GEN_AI_SYSTEM_HUGGING_FACE,
            scope._server_address,
            scope._server_port,
            request_model,
            request_model,
            environment,
            application_name,
            scope._start_time,
            scope._end_time,
            cost,
            input_tokens,
            output_tokens,
            scope._tbt,
            scope._ttft,
        )


def process_chat_response(
    instance,
    response,
    request_model,
    pricing_info,
    server_port,
    server_address,
    environment,
    application_name,
    metrics,
    start_time,
    span,
    args,
    kwargs,
    capture_message_content=False,
    disable_metrics=False,
    version="1.0.0",
):
    """
    Process chat request and generate Telemetry
    """

    scope = type("GenericScope", (), {})()
    scope._instance = instance
    scope._start_time = start_time
    scope._end_time = time.time()
    scope._span = span
    scope._server_address = server_address
    scope._server_port = server_port
    scope._kwargs = kwargs
    scope._args = args

    # Extract prompt from args or kwargs
    if args and len(args) > 0:
        scope._prompt = args[0]
    else:
        scope._prompt = (
            kwargs.get("text_inputs")
            or (
                kwargs.get("image")
                and kwargs.get("question")
                and (
                    "image: "
                    + kwargs.get("image")
                    + " question:"
                    + kwargs.get("question")
                )
            )
            or kwargs.get("fallback")
            or ""
        )
    scope._prompt = format_content(scope._prompt)

    # Process response based on task type
    task = kwargs.get("task", "text-generation")

    if task == "text-generation":
        # Handle text generation responses
        if isinstance(response, list) and len(response) > 0:
            first_entry = response[0]
            if isinstance(first_entry, dict):
                if isinstance(first_entry.get("generated_text"), list):
                    # Handle nested list format
                    last_element = first_entry.get("generated_text")[-1]
                    scope._completion = last_element.get("content", str(last_element))
                else:
                    # Handle standard format
                    scope._completion = first_entry.get("generated_text", "")
            else:
                scope._completion = str(first_entry)
        else:
            scope._completion = ""

    elif task == "automatic-speech-recognition":
        scope._completion = (
            response.get("text", "") if isinstance(response, dict) else ""
        )

    elif task == "image-classification":
        scope._completion = (
            str(response[0]) if isinstance(response, list) and len(response) > 0 else ""
        )

    elif task == "visual-question-answering":
        if (
            isinstance(response, list)
            and len(response) > 0
            and isinstance(response[0], dict)
        ):
            scope._completion = response[0].get("answer", "")
        else:
            scope._completion = ""
    else:
        # Default handling for other tasks
        scope._completion = format_content(response)

    # Initialize timing attributes
    scope._tbt = 0
    scope._ttft = scope._end_time - scope._start_time

    common_chat_logic(
        scope,
        pricing_info,
        environment,
        application_name,
        metrics,
        capture_message_content,
        disable_metrics,
        version,
        args,
        kwargs,
        is_stream=False,
    )

    return response
