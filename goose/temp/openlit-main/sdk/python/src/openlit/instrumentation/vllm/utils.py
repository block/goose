"""
vLLM OpenTelemetry instrumentation utility functions
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


def get_inference_config(args, kwargs):
    """
    Safely extract inference configuration from args or kwargs.
    """

    if "sampling_params" in kwargs:
        return kwargs["sampling_params"]
    if len(args) > 1:
        return args[1]
    return None


def format_content(prompts):
    """
    Process a list of prompts to extract content.
    """

    if isinstance(prompts, str):
        return prompts
    elif isinstance(prompts, list):
        return "\n".join(str(prompt) for prompt in prompts)
    else:
        return str(prompts)


def common_chat_logic(
    scope,
    pricing_info,
    environment,
    application_name,
    metrics,
    capture_message_content,
    disable_metrics,
    version,
    is_stream,
):
    """
    Process chat request and generate Telemetry
    """

    request_model = scope._request_model

    # Extract prompts and completions from vLLM response
    input_tokens = 0
    output_tokens = 0
    prompt = ""
    completion = ""

    for output in scope._response:
        prompt += output.prompt + "\n"
        if output.outputs and len(output.outputs) > 0:
            completion += output.outputs[0].text + "\n"
        input_tokens += general_tokens(output.prompt)
        output_tokens += general_tokens(output.outputs[0].text)

    cost = get_chat_model_cost(request_model, pricing_info, input_tokens, output_tokens)

    # Common Span Attributes
    common_span_attributes(
        scope,
        SemanticConvention.GEN_AI_OPERATION_TYPE_CHAT,
        SemanticConvention.GEN_AI_SYSTEM_VLLM,
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

    # Span Attributes for Request parameters
    inference_config = get_inference_config(scope._args, scope._kwargs)
    if inference_config:
        scope._span.set_attribute(
            SemanticConvention.GEN_AI_REQUEST_MAX_TOKENS,
            getattr(inference_config, "max_tokens", -1),
        )
        scope._span.set_attribute(
            SemanticConvention.GEN_AI_REQUEST_STOP_SEQUENCES,
            getattr(inference_config, "stop_sequences", []),
        )
        scope._span.set_attribute(
            SemanticConvention.GEN_AI_REQUEST_TEMPERATURE,
            getattr(inference_config, "temperature", 1.0),
        )
        scope._span.set_attribute(
            SemanticConvention.GEN_AI_REQUEST_TOP_P,
            getattr(inference_config, "top_p", 1.0),
        )
        scope._span.set_attribute(
            SemanticConvention.GEN_AI_REQUEST_TOP_K,
            getattr(inference_config, "top_k", -1),
        )
        scope._span.set_attribute(
            SemanticConvention.GEN_AI_REQUEST_PRESENCE_PENALTY,
            getattr(inference_config, "presence_penalty", 0.0),
        )
        scope._span.set_attribute(
            SemanticConvention.GEN_AI_REQUEST_FREQUENCY_PENALTY,
            getattr(inference_config, "frequency_penalty", 0.0),
        )

    # Span Attributes for Response parameters
    scope._span.set_attribute(SemanticConvention.GEN_AI_OUTPUT_TYPE, "text")

    # Span Attributes for Cost and Tokens
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
        scope._span.set_attribute(SemanticConvention.GEN_AI_CONTENT_PROMPT, prompt)
        scope._span.set_attribute(
            SemanticConvention.GEN_AI_CONTENT_COMPLETION, completion
        )

        # To be removed once the change to span_attributes (from span events) is complete
        scope._span.add_event(
            name=SemanticConvention.GEN_AI_CONTENT_PROMPT_EVENT,
            attributes={
                SemanticConvention.GEN_AI_CONTENT_PROMPT: prompt,
            },
        )
        scope._span.add_event(
            name=SemanticConvention.GEN_AI_CONTENT_COMPLETION_EVENT,
            attributes={
                SemanticConvention.GEN_AI_CONTENT_COMPLETION: completion,
            },
        )

    scope._span.set_status(Status(StatusCode.OK))

    # Metrics
    if not disable_metrics:
        record_completion_metrics(
            metrics,
            SemanticConvention.GEN_AI_OPERATION_TYPE_CHAT,
            SemanticConvention.GEN_AI_SYSTEM_VLLM,
            scope._server_address,
            scope._server_port,
            request_model,
            request_model,
            environment,
            application_name,
            scope._start_time,
            scope._end_time,
            input_tokens,
            output_tokens,
            cost,
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

    # Create scope object
    scope = type("GenericScope", (), {})()

    scope._response = response
    scope._start_time = start_time
    scope._end_time = time.time()
    scope._span = span
    scope._ttft, scope._tbt = scope._end_time - scope._start_time, 0
    scope._server_address = server_address
    scope._server_port = server_port
    scope._request_model = request_model
    scope._timestamps = []
    scope._args = args
    scope._kwargs = kwargs

    common_chat_logic(
        scope,
        pricing_info,
        environment,
        application_name,
        metrics,
        capture_message_content,
        disable_metrics,
        version,
        is_stream=False,
    )

    return response
