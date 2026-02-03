"""
Module for monitoring AG2 API calls.
"""

import time
from opentelemetry.trace import SpanKind
from openlit.__helpers import handle_exception, set_server_address_and_port
from openlit.instrumentation.ag2.utils import (
    process_agent_creation,
    process_agent_run,
    process_agent_generate_reply,
    process_agent_receive,
    process_agent_send,
    process_groupchat_operation,
    process_speaker_selection,
)
from openlit.semcov import SemanticConvention


def extract_agent_name(instance, fallback="unknown_agent"):
    """
    Extract agent name from AG2 instance with intelligent fallbacks.

    Args:
        instance: AG2 instance (Agent, GroupChat, etc.)
        fallback: Default name if no name can be extracted

    Returns:
        str: Agent name or meaningful fallback
    """
    # Try to get the name attribute first
    agent_name = getattr(instance, "name", None)
    if agent_name:
        return agent_name

    # Try to get from class name and make it meaningful
    class_name = getattr(instance, "__class__", type(instance)).__name__.lower()

    # Map common AG2 class names to meaningful names
    class_name_map = {
        "conversableagent": "conversable_agent",
        "groupchat": "group_chat",
        "groupchatmanager": "group_chat_manager",
        "agent": "agent",
    }

    return class_name_map.get(class_name, fallback)


def conversable_agent(
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
    Generates a telemetry wrapper for AG2 conversable agent creation.
    """

    def wrapper(wrapped, instance, args, kwargs):
        """
        Wraps the AG2 conversable agent creation call.
        """

        server_address, server_port = set_server_address_and_port(
            instance, "127.0.0.1", 80
        )
        agent_name = kwargs.get("name", "unknown_agent")
        llm_config = kwargs.get("llm_config", {})
        system_message = kwargs.get("system_message", "")

        span_name = (
            f"{SemanticConvention.GEN_AI_OPERATION_TYPE_CREATE_AGENT} {agent_name}"
        )

        with tracer.start_as_current_span(span_name, kind=SpanKind.CLIENT) as span:
            start_time = time.time()
            response = wrapped(*args, **kwargs)

            try:
                process_agent_creation(
                    agent_name=agent_name,
                    llm_config=llm_config,
                    system_message=system_message,
                    pricing_info=pricing_info,
                    server_port=server_port,
                    server_address=server_address,
                    environment=environment,
                    application_name=application_name,
                    metrics=metrics,
                    start_time=start_time,
                    span=span,
                    capture_message_content=capture_message_content,
                    disable_metrics=disable_metrics,
                    version=version,
                )

            except Exception as e:
                handle_exception(span, e)

            return response

    return wrapper


def agent_run(
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
    Generates a telemetry wrapper for AG2 agent run execution.
    """

    def wrapper(wrapped, instance, args, kwargs):
        """
        Wraps the AG2 agent run execution call.
        """

        server_address, server_port = set_server_address_and_port(
            instance, "127.0.0.1", 80
        )

        # Extract agent name from instance
        agent_name = extract_agent_name(instance)

        # Extract model from instance llm_config
        request_model = "unknown"
        if hasattr(instance, "llm_config") and isinstance(instance.llm_config, dict):
            request_model = instance.llm_config.get("model", "unknown")

        span_name = f"{SemanticConvention.GEN_AI_OPERATION_TYPE_EXECUTE_AGENT_TASK} {agent_name}"

        with tracer.start_as_current_span(span_name, kind=SpanKind.CLIENT) as span:
            start_time = time.time()
            response = wrapped(*args, **kwargs)

            try:
                response = process_agent_run(
                    response=response,
                    agent_name=agent_name,
                    request_model=request_model,
                    pricing_info=pricing_info,
                    server_port=server_port,
                    server_address=server_address,
                    environment=environment,
                    application_name=application_name,
                    metrics=metrics,
                    start_time=start_time,
                    span=span,
                    capture_message_content=capture_message_content,
                    disable_metrics=disable_metrics,
                    version=version,
                )

            except Exception as e:
                handle_exception(span, e)

            return response

    return wrapper


def agent_generate_reply(
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
    Generates a telemetry wrapper for AG2 ConversableAgent.generate_reply.
    """

    def wrapper(wrapped, instance, args, kwargs):
        """
        Wraps the AG2 ConversableAgent.generate_reply call.
        """

        server_address, server_port = set_server_address_and_port(
            instance, "127.0.0.1", 80
        )

        # Extract agent name from instance
        agent_name = extract_agent_name(instance)

        # Extract model from instance llm_config
        request_model = "unknown"
        if hasattr(instance, "llm_config") and isinstance(instance.llm_config, dict):
            request_model = instance.llm_config.get("model", "unknown")

        span_name = f"{SemanticConvention.GEN_AI_OPERATION_TYPE_EXECUTE_AGENT_TASK} {agent_name}"

        with tracer.start_as_current_span(span_name, kind=SpanKind.CLIENT) as span:
            start_time = time.time()
            response = wrapped(*args, **kwargs)

            try:
                response = process_agent_generate_reply(
                    response=response,
                    agent_name=agent_name,
                    request_model=request_model,
                    messages=args[0] if args else kwargs.get("messages", []),
                    sender=args[1] if len(args) > 1 else kwargs.get("sender", None),
                    pricing_info=pricing_info,
                    server_port=server_port,
                    server_address=server_address,
                    environment=environment,
                    application_name=application_name,
                    metrics=metrics,
                    start_time=start_time,
                    span=span,
                    capture_message_content=capture_message_content,
                    disable_metrics=disable_metrics,
                    version=version,
                )

            except Exception as e:
                handle_exception(span, e)

            return response

    return wrapper


def agent_receive(
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
    Generates a telemetry wrapper for AG2 ConversableAgent.receive.
    """

    def wrapper(wrapped, instance, args, kwargs):
        """
        Wraps the AG2 ConversableAgent.receive call.
        """

        server_address, server_port = set_server_address_and_port(
            instance, "127.0.0.1", 80
        )

        # Extract agent name from instance
        agent_name = extract_agent_name(instance)

        # Extract sender information
        sender = args[0] if args else kwargs.get("sender", None)
        sender_name = getattr(sender, "name", "Unknown") if sender else "Unknown"

        # Extract message
        message = args[1] if len(args) > 1 else kwargs.get("message", "")

        span_name = f"{SemanticConvention.GEN_AI_OPERATION_TYPE_EXECUTE_AGENT_TASK} {agent_name}"

        with tracer.start_as_current_span(span_name, kind=SpanKind.CLIENT) as span:
            start_time = time.time()
            response = wrapped(*args, **kwargs)

            try:
                process_agent_receive(
                    message=message,
                    agent_name=agent_name,
                    sender_name=sender_name,
                    agent_instance=instance,
                    pricing_info=pricing_info,
                    server_port=server_port,
                    server_address=server_address,
                    environment=environment,
                    application_name=application_name,
                    metrics=metrics,
                    start_time=start_time,
                    span=span,
                    capture_message_content=capture_message_content,
                    disable_metrics=disable_metrics,
                    version=version,
                )

            except Exception as e:
                handle_exception(span, e)

            return response

    return wrapper


def agent_send(
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
    Generates a telemetry wrapper for AG2 ConversableAgent.send.
    """

    def wrapper(wrapped, instance, args, kwargs):
        """
        Wraps the AG2 ConversableAgent.send call.
        """

        server_address, server_port = set_server_address_and_port(
            instance, "127.0.0.1", 80
        )

        # Extract agent name from instance
        agent_name = extract_agent_name(instance)

        # Extract recipient information
        recipient = args[0] if args else kwargs.get("recipient", None)
        recipient_name = (
            getattr(recipient, "name", "Unknown") if recipient else "Unknown"
        )

        # Extract message
        message = args[1] if len(args) > 1 else kwargs.get("message", "")

        span_name = f"{SemanticConvention.GEN_AI_OPERATION_TYPE_EXECUTE_AGENT_TASK} {agent_name}"

        with tracer.start_as_current_span(span_name, kind=SpanKind.CLIENT) as span:
            start_time = time.time()
            response = wrapped(*args, **kwargs)

            try:
                process_agent_send(
                    message=message,
                    agent_name=agent_name,
                    recipient_name=recipient_name,
                    agent_instance=instance,
                    pricing_info=pricing_info,
                    server_port=server_port,
                    server_address=server_address,
                    environment=environment,
                    application_name=application_name,
                    metrics=metrics,
                    start_time=start_time,
                    span=span,
                    capture_message_content=capture_message_content,
                    disable_metrics=disable_metrics,
                    version=version,
                )

            except Exception as e:
                handle_exception(span, e)

            return response

    return wrapper


def groupchat_manager_run_chat(
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
    Generates a telemetry wrapper for AG2 GroupChatManager.run_chat.
    """

    def wrapper(wrapped, instance, args, kwargs):
        """
        Wraps the AG2 GroupChatManager.run_chat call.
        """

        server_address, server_port = set_server_address_and_port(
            instance, "127.0.0.1", 80
        )

        # Extract groupchat information
        groupchat = getattr(instance, "groupchat", None)
        if groupchat:
            participants = [agent.name for agent in groupchat.agents]
            group_name = f"GroupChat_{len(participants)}_agents"
        else:
            participants = []
            group_name = "UnknownGroupChat"

        # Extract model information from GroupChatManager
        request_model = "unknown"  # Default fallback
        if hasattr(instance, "llm_config") and isinstance(instance.llm_config, dict):
            request_model = instance.llm_config.get("model", "unknown")

        # Try to get more specific model from groupchat
        if groupchat and hasattr(groupchat, "select_speaker_auto_llm_config"):
            llm_config = groupchat.select_speaker_auto_llm_config
            if isinstance(llm_config, dict):
                request_model = llm_config.get("model", request_model)
            elif hasattr(llm_config, "model"):
                request_model = llm_config.model

        # Extract sender information
        sender = kwargs.get("sender", None)

        # Extract messages
        messages = args[0] if args else kwargs.get("messages", [])

        span_name = f"{SemanticConvention.GEN_AI_OPERATION_TYPE_FRAMEWORK} {group_name}"

        with tracer.start_as_current_span(span_name, kind=SpanKind.CLIENT) as span:
            start_time = time.time()
            response = wrapped(*args, **kwargs)

            try:
                process_groupchat_operation(
                    group_name=group_name,
                    participants=participants,
                    messages=messages,
                    sender=sender,
                    max_turns=None,  # Not available in new API
                    request_model=request_model,
                    pricing_info=pricing_info,
                    server_port=server_port,
                    server_address=server_address,
                    environment=environment,
                    application_name=application_name,
                    metrics=metrics,
                    start_time=start_time,
                    span=span,
                    capture_message_content=capture_message_content,
                    disable_metrics=disable_metrics,
                    version=version,
                )

            except Exception as e:
                handle_exception(span, e)

            return response

    return wrapper


def groupchat_select_speaker(
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
    Generates a telemetry wrapper for AG2 GroupChat.select_speaker.
    """

    def wrapper(wrapped, instance, args, kwargs):
        """
        Wraps the AG2 GroupChat.select_speaker call.
        """

        server_address, server_port = set_server_address_and_port(
            instance, "127.0.0.1", 80
        )

        # Extract speaker information
        last_speaker = args[0] if args else kwargs.get("last_speaker", None)
        selector = args[1] if len(args) > 1 else kwargs.get("selector", None)

        last_speaker_name = (
            getattr(last_speaker, "name", "Unknown") if last_speaker else "Unknown"
        )

        # Extract agents list
        agents = getattr(instance, "agents", [])

        # Extract model information from GroupChat instance
        request_model = "unknown"  # Default fallback
        # Check for speaker selection specific config
        if hasattr(instance, "select_speaker_auto_llm_config"):
            llm_config = instance.select_speaker_auto_llm_config
            if isinstance(llm_config, dict):
                request_model = llm_config.get("model", "unknown")
            elif hasattr(llm_config, "model"):
                request_model = llm_config.model

        # Try to get model from selector if available
        if (
            selector
            and hasattr(selector, "llm_config")
            and isinstance(selector.llm_config, dict)
        ):
            request_model = selector.llm_config.get("model", request_model)

        # Try to get model from agents if still unknown
        if request_model == "unknown" and agents:
            for agent in agents:
                if hasattr(agent, "llm_config") and isinstance(agent.llm_config, dict):
                    model = agent.llm_config.get("model")
                    if model:
                        request_model = model
                        break

        span_name = (
            f"{SemanticConvention.GEN_AI_OPERATION_TYPE_AGENT} speaker_selection"
        )

        with tracer.start_as_current_span(span_name, kind=SpanKind.CLIENT) as span:
            start_time = time.time()
            response = wrapped(*args, **kwargs)

            try:
                selected_speaker_name = (
                    getattr(response, "name", "Unknown") if response else "Unknown"
                )

                process_speaker_selection(
                    last_speaker=last_speaker_name,
                    selected_speaker=selected_speaker_name,
                    selector=selector,
                    agents=agents,
                    request_model=request_model,
                    pricing_info=pricing_info,
                    server_port=server_port,
                    server_address=server_address,
                    environment=environment,
                    application_name=application_name,
                    metrics=metrics,
                    start_time=start_time,
                    span=span,
                    capture_message_content=capture_message_content,
                    disable_metrics=disable_metrics,
                    version=version,
                )

            except Exception as e:
                handle_exception(span, e)

            return response

    return wrapper
