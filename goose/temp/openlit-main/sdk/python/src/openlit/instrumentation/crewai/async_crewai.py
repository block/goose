"""
CrewAI async wrapper using modern async_general_wrap pattern
"""

import time
from opentelemetry.trace import SpanKind
from opentelemetry import context as context_api
from openlit.__helpers import handle_exception
from openlit.instrumentation.crewai.utils import (
    process_crewai_response,
    OPERATION_MAP,
    set_server_address_and_port,
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
    Modern async wrapper for CrewAI operations following Framework Instrumentation Guide patterns.
    """

    async def wrapper(wrapped, instance, args, kwargs):
        """
        Wraps the async CrewAI operation call with comprehensive telemetry.
        """

        # CRITICAL: Suppression check
        if context_api.get_value(context_api._SUPPRESS_INSTRUMENTATION_KEY):
            return await wrapped(*args, **kwargs)

        # Get server address and port using the standard helper
        server_address, server_port = set_server_address_and_port(instance)

        # Get operation type from mapping
        operation_type = OPERATION_MAP.get(gen_ai_endpoint, "framework")

        # Generate span name following {operation_type} {operation_name} pattern
        span_name = _generate_span_name(
            operation_type, gen_ai_endpoint, instance, args, kwargs
        )

        with tracer.start_as_current_span(span_name, kind=SpanKind.CLIENT) as span:
            start_time = time.time()
            response = await wrapped(*args, **kwargs)

            try:
                # Process response and generate comprehensive telemetry
                response = process_crewai_response(
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

            except Exception as e:
                handle_exception(span, e)

            return response

    return wrapper


def _generate_span_name(operation_type, endpoint, instance, args, kwargs):
    """
    Generate proper span names following {operation_type} {operation_name} convention.
    """

    # Crew-level operations
    if endpoint.startswith("crew_"):
        crew_name = getattr(instance, "name", None) or "CrewAI Workflow"
        if endpoint == "crew_kickoff_async":
            return f"{operation_type} {crew_name}"
        elif endpoint == "crew_kickoff_for_each_async":
            return f"{operation_type} {crew_name} Batch"
        else:
            return f"{operation_type} {crew_name}"

    # Agent-level operations
    elif endpoint.startswith("agent_"):
        agent_role = getattr(instance, "role", None) or "Agent"
        return f"{operation_type} {agent_role}"

    # Task-level operations
    elif endpoint.startswith("task_"):
        task_description = getattr(instance, "description", None)
        if task_description and len(task_description) < 50:
            return f"{operation_type} {task_description}"
        else:
            return f"{operation_type} Task"

    # Default naming for async operations
    else:
        return f"{operation_type} {endpoint}"
