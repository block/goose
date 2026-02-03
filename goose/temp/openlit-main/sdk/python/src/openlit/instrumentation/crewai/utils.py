"""
CrewAI utilities for comprehensive telemetry processing and business intelligence
"""

import time
import json
from urllib.parse import urlparse
from opentelemetry.trace import Status, StatusCode
from openlit.__helpers import (
    common_framework_span_attributes,
    handle_exception,
)
from openlit.semcov import SemanticConvention

# === OPERATION MAPPING - Framework Guide Compliant ===
OPERATION_MAP = {
    # === STANDARD OPENTELEMETRY OPERATION NAMES ===
    # Crew Operations (workflow management)
    "crew_kickoff": "invoke_agent",
    "crew_train": "invoke_agent",
    "crew_replay": "invoke_agent",
    "crew_test": "invoke_agent",
    # Agent Operations (core agent functions)
    "agent___init__": "create_agent",
    "agent_execute_task": "invoke_agent",
    "agent_backstory_property": "invoke_agent",
    # Task Operations (task execution)
    "task_execute": "invoke_agent",
    "task_execute_async": "invoke_agent",
    "task_execute_core": "invoke_agent",
    # Tool Operations (tool execution)
    "tool_run": "execute_tool",
    "tool___call__": "execute_tool",
    "tool_execute": "execute_tool",
    # Memory Operations (knowledge management)
    "memory_save": "invoke_agent",
    "memory_search": "invoke_agent",
    "memory_reset": "invoke_agent",
}


def set_server_address_and_port(instance):
    """
    Extract server information from CrewAI instance.

    Args:
        instance: CrewAI instance (Crew, Agent, Task, etc.)

    Returns:
        tuple: (server_address, server_port)
    """
    server_address = "localhost"
    server_port = 8080

    # Try to extract LLM endpoint information
    try:
        if hasattr(instance, "llm") and hasattr(instance.llm, "api_base"):
            parsed = urlparse(instance.llm.api_base)
            server_address = parsed.hostname or "localhost"
            server_port = parsed.port or 443
        elif hasattr(instance, "agent") and hasattr(instance.agent, "llm"):
            # For tasks that have an agent with LLM
            if hasattr(instance.agent.llm, "api_base"):
                parsed = urlparse(instance.agent.llm.api_base)
                server_address = parsed.hostname or "localhost"
                server_port = parsed.port or 443
    except Exception:
        # Graceful degradation
        pass

    return server_address, server_port


def process_crewai_response(
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
    Process CrewAI response with comprehensive business intelligence.
    OpenLIT's competitive advantage through superior observability.
    """

    end_time = time.time()
    duration_ms = (end_time - start_time) * 1000

    # Create proper scope object for common_framework_span_attributes
    scope = type("GenericScope", (), {})()
    scope._span = span
    scope._start_time = start_time
    scope._end_time = end_time

    # Get standard operation name from mapping
    standard_operation = OPERATION_MAP.get(endpoint, "invoke_agent")

    # Extract model information from agent's LLM for proper attribution
    request_model = "unknown"
    if instance:
        llm = getattr(instance, "llm", None)
        if llm:
            # Try different model attribute names used by different LLM libraries
            request_model = (
                getattr(llm, "model_name", None)
                or getattr(llm, "model", None)
                or getattr(llm, "_model_name", None)
                or "unknown"
            )
            if request_model != "unknown":
                request_model = str(request_model)

    # Create a wrapper instance that exposes model_name for common_framework_span_attributes
    class ModelWrapper:
        """Wrapper class to expose model_name for framework span attributes."""

        def __init__(self, original_instance, model_name):
            self._original = original_instance
            self.model_name = model_name

        def __getattr__(self, name):
            return getattr(self._original, name)

        def get_original_instance(self):
            """Get the original wrapped instance."""
            return self._original

    model_instance = ModelWrapper(instance, request_model) if instance else None

    # Set common framework span attributes
    common_framework_span_attributes(
        scope,
        SemanticConvention.GEN_AI_SYSTEM_CREWAI,
        server_address,
        server_port,
        environment,
        application_name,
        version,
        endpoint,
        model_instance,
    )

    # Set span name following OpenTelemetry format
    _set_span_name(span, standard_operation, instance, endpoint, args, kwargs)

    # === CORE SEMANTIC ATTRIBUTES ===
    span.set_attribute(SemanticConvention.GEN_AI_OPERATION, standard_operation)
    # Remove gen_ai.endpoint as requested

    # === STANDARD BUSINESS INTELLIGENCE ===
    # Only use standard OpenTelemetry attributes, no framework-specific ones
    _set_agent_business_intelligence(span, instance, endpoint, args, kwargs)
    _set_tool_business_intelligence(span, instance, endpoint, args, kwargs)
    _set_task_business_intelligence(span, instance, endpoint, args, kwargs)
    _set_crew_business_intelligence(span, instance, endpoint, args, kwargs)
    # Remove framework-specific functions: _set_workflow_business_intelligence, _set_memory_business_intelligence

    # === PERFORMANCE INTELLIGENCE ===
    # Use standard OpenTelemetry duration attribute through common_framework_span_attributes

    # === CONTENT CAPTURE ===
    if capture_message_content:
        _capture_content(span, instance, response, endpoint)

    # === COST TRACKING ===
    _track_cost_and_tokens(span, instance, response, endpoint)

    # === RECORD METRICS ===
    if not disable_metrics and metrics:
        _record_crewai_metrics(
            metrics, standard_operation, duration_ms, environment, application_name
        )

    span.set_status(Status(StatusCode.OK))
    return response


def _set_span_name(span, operation_type, instance, endpoint, args, kwargs):
    """Set span name following OpenTelemetry format: '{operation_type} {name}'"""
    try:
        # Get the operation name from our mapping
        operation_name = OPERATION_MAP.get(endpoint, "invoke_agent")

        if endpoint.startswith("crew_"):
            # Crew operations: "invoke_agent {crew_name}"
            crew_name = getattr(instance, "name", None) or "crew"
            span.update_name(f"{operation_name} {crew_name}")

        elif endpoint.startswith("agent_"):
            if "create" in endpoint or endpoint == "agent___init__":
                # Agent creation: "create_agent {agent_name}"
                agent_name = getattr(instance, "name", None) or getattr(
                    instance, "role", "agent"
                )
                span.update_name(f"create_agent {agent_name}")
            else:
                # Agent invocation: "invoke_agent {agent_name}"
                agent_name = getattr(instance, "name", None) or getattr(
                    instance, "role", "agent"
                )
                span.update_name(f"invoke_agent {agent_name}")

        elif endpoint.startswith("task_"):
            # Task operations: "invoke_agent task"
            span.update_name("invoke_agent task")

        elif endpoint.startswith("tool_"):
            # Tool operations: "execute_tool {tool_name}"
            tool_name = (
                getattr(instance, "name", None)
                or getattr(instance, "__class__", type(instance)).__name__
            )
            span.update_name(f"execute_tool {tool_name}")

        elif endpoint.startswith("memory_"):
            # Memory operations: "invoke_agent memory:{operation}"
            memory_op = endpoint.split("_", 1)[1] if "_" in endpoint else "operation"
            span.update_name(f"invoke_agent memory:{memory_op}")

        else:
            # Default fallback
            span.update_name(f"{operation_name} {endpoint}")

    except Exception as e:
        handle_exception(span, e)
        # Fallback naming
        span.update_name(f"invoke_agent {endpoint}")


def _set_agent_business_intelligence(span, instance, endpoint, args, kwargs):
    """Set agent business intelligence using standard OpenTelemetry semantic conventions"""
    if not endpoint.startswith("agent_"):
        return

    try:
        # Standard OpenTelemetry Gen AI Agent attributes
        agent_id = getattr(instance, "id", "")
        if agent_id:
            span.set_attribute(SemanticConvention.GEN_AI_AGENT_ID, str(agent_id))

        agent_name = getattr(instance, "name", None) or getattr(instance, "role", "")
        if agent_name:
            span.set_attribute(SemanticConvention.GEN_AI_AGENT_NAME, agent_name)

        # Agent description - use role + goal as description per OpenTelemetry spec
        agent_role = getattr(instance, "role", "")
        agent_goal = getattr(instance, "goal", "")
        if agent_role and agent_goal:
            description = f"{agent_role}: {agent_goal}"
            span.set_attribute(SemanticConvention.GEN_AI_AGENT_DESCRIPTION, description)
        elif agent_goal:
            span.set_attribute(SemanticConvention.GEN_AI_AGENT_DESCRIPTION, agent_goal)

        # Enhanced Agent Configuration Tracking using SemanticConvention
        max_retry_limit = getattr(instance, "max_retry_limit", None)
        if max_retry_limit is not None:
            span.set_attribute(
                SemanticConvention.GEN_AI_AGENT_MAX_RETRY_LIMIT, max_retry_limit
            )

        allow_delegation = getattr(instance, "allow_delegation", None)
        if allow_delegation is not None:
            span.set_attribute(
                SemanticConvention.GEN_AI_AGENT_ALLOW_DELEGATION, allow_delegation
            )

        allow_code_execution = getattr(instance, "allow_code_execution", None)
        if allow_code_execution is not None:
            span.set_attribute(
                SemanticConvention.GEN_AI_AGENT_ALLOW_CODE_EXECUTION,
                allow_code_execution,
            )

        # Tools tracking using SemanticConvention
        tools = getattr(instance, "tools", [])
        if tools:
            tool_names = [
                getattr(tool, "name", str(tool)) for tool in tools[:5]
            ]  # Limit to first 5
            if tool_names:
                span.set_attribute(
                    SemanticConvention.GEN_AI_AGENT_TOOLS, ", ".join(tool_names)
                )

        # === OpenAI Agent-specific Attributes ===
        _set_openai_agent_attributes(span, instance, endpoint, args, kwargs)

        # === Conversation and Data Source Tracking ===
        _set_conversation_and_data_source_attributes(
            span, instance, endpoint, args, kwargs
        )

    except Exception as e:
        handle_exception(span, e)


def _set_openai_agent_attributes(span, instance, endpoint, args, kwargs):
    """Set OpenAI-specific agent attributes when using OpenAI models"""
    try:
        # Check if agent is using OpenAI LLM
        llm = getattr(instance, "llm", None)
        if llm:
            llm_class = llm.__class__.__name__.lower()
            llm_model = getattr(llm, "model_name", getattr(llm, "model", ""))

            # Set model information
            if llm_model:
                span.set_attribute(SemanticConvention.GEN_AI_REQUEST_MODEL, llm_model)

            # OpenAI-specific attributes (but keep gen_ai.system as crewai)
            if "openai" in llm_class or "gpt" in str(llm_model).lower():
                # OpenAI service tier if available
                service_tier = getattr(llm, "service_tier", None)
                if service_tier:
                    span.set_attribute(
                        SemanticConvention.GEN_AI_OPENAI_REQUEST_SERVICE_TIER,
                        service_tier,
                    )

                # OpenAI Assistant API attributes if available
                assistant_id = getattr(instance, "assistant_id", None) or kwargs.get(
                    "assistant_id"
                )
                if assistant_id:
                    span.set_attribute(
                        SemanticConvention.GEN_AI_OPENAI_ASSISTANT_ID, assistant_id
                    )

                thread_id = getattr(instance, "thread_id", None) or kwargs.get(
                    "thread_id"
                )
                if thread_id:
                    span.set_attribute(
                        SemanticConvention.GEN_AI_OPENAI_THREAD_ID, thread_id
                    )

                run_id = getattr(instance, "run_id", None) or kwargs.get("run_id")
                if run_id:
                    span.set_attribute(SemanticConvention.GEN_AI_OPENAI_RUN_ID, run_id)

            # LiteLLM detection (but keep gen_ai.system as crewai)
            elif "litellm" in llm_class:
                # Could add LiteLLM-specific attributes here if needed
                pass

    except Exception as e:
        handle_exception(span, e)


def _set_conversation_and_data_source_attributes(
    span, instance, endpoint, args, kwargs
):
    """Set conversation tracking and data source attributes"""
    try:
        # Conversation ID for multi-turn interactions
        conversation_id = (
            getattr(instance, "conversation_id", None)
            or getattr(instance, "session_id", None)
            or kwargs.get("conversation_id")
            or kwargs.get("session_id")
        )
        if conversation_id:
            span.set_attribute(
                SemanticConvention.GEN_AI_CONVERSATION_ID, str(conversation_id)
            )

        # Data source tracking for RAG operations
        memory = getattr(instance, "memory", None)
        if memory:
            # Memory as data source
            memory_provider = getattr(memory, "provider", None)
            if memory_provider:
                span.set_attribute(SemanticConvention.GEN_AI_DATA_SOURCE_TYPE, "memory")
                span.set_attribute(
                    SemanticConvention.GEN_AI_DATA_SOURCE_ID, str(memory_provider)
                )

        # Knowledge base or vector store detection
        knowledge_source = getattr(instance, "knowledge_source", None)
        if knowledge_source:
            span.set_attribute(
                SemanticConvention.GEN_AI_DATA_SOURCE_TYPE, "knowledge_base"
            )
            span.set_attribute(
                SemanticConvention.GEN_AI_DATA_SOURCE_ID, str(knowledge_source)
            )

        # Tool-based data sources
        tools = getattr(instance, "tools", [])
        for tool in tools:
            tool_name = getattr(tool, "name", "").lower()
            if any(
                keyword in tool_name
                for keyword in ["search", "retrieval", "database", "vector"]
            ):
                span.set_attribute(
                    SemanticConvention.GEN_AI_DATA_SOURCE_TYPE, "external_tool"
                )
                break

    except Exception as e:
        handle_exception(span, e)


def _set_task_business_intelligence(span, instance, endpoint, args, kwargs):
    """Set task business intelligence using standard OpenTelemetry semantic conventions"""
    if not endpoint.startswith("task_"):
        return

    try:
        # Task ID tracking
        task_id = getattr(instance, "id", None)
        if task_id:
            span.set_attribute(SemanticConvention.GEN_AI_AGENT_TASK_ID, str(task_id))

        # Task description
        task_description = getattr(instance, "description", "")
        if task_description:
            span.set_attribute(
                SemanticConvention.GEN_AI_TASK_DESCRIPTION, task_description
            )

        # Task expected output (keep only essential attributes that have semantic conventions or are critical)
        expected_output = getattr(instance, "expected_output", "")
        if expected_output:
            span.set_attribute(
                SemanticConvention.GEN_AI_TASK_EXPECTED_OUTPUT, expected_output
            )

    except Exception as e:
        handle_exception(span, e)


def _set_crew_business_intelligence(span, instance, endpoint, args, kwargs):
    """Set crew business intelligence using standard OpenTelemetry semantic conventions"""
    if not endpoint.startswith("crew_"):
        return

    try:
        # Only capture essential crew attributes - remove custom ones that don't have semantic conventions
        pass

    except Exception as e:
        handle_exception(span, e)


def _set_tool_business_intelligence(span, instance, endpoint, args, kwargs):
    """Set tool business intelligence using standard OpenTelemetry semantic conventions"""
    if not endpoint.startswith("tool_"):
        return

    try:
        # Standard OpenTelemetry Gen AI Tool attributes
        tool_name = (
            getattr(instance, "name", None)
            or getattr(instance, "__class__", type(instance)).__name__
        )
        if tool_name:
            span.set_attribute(SemanticConvention.GEN_AI_TOOL_NAME, tool_name)

        # Tool call ID if available (for tracking specific tool invocations)
        tool_call_id = kwargs.get("call_id", None) or getattr(instance, "call_id", None)
        if tool_call_id:
            span.set_attribute(
                SemanticConvention.GEN_AI_TOOL_CALL_ID, str(tool_call_id)
            )

        # === OpenAI Function Calling Attributes ===
        _set_openai_tool_attributes(span, instance, endpoint, args, kwargs)

    except Exception as e:
        handle_exception(span, e)


def _set_openai_tool_attributes(span, instance, endpoint, args, kwargs):
    """Set OpenAI function calling specific attributes using standard conventions"""
    try:
        # Standard tool type classification (framework-agnostic)
        tool_class = instance.__class__.__name__.lower()
        if any(keyword in tool_class for keyword in ["search", "web", "browser"]):
            tool_type = "search"
        elif any(keyword in tool_class for keyword in ["file", "read", "write"]):
            tool_type = "file_system"
        elif any(keyword in tool_class for keyword in ["api", "http", "request"]):
            tool_type = "api_client"
        elif any(keyword in tool_class for keyword in ["database", "sql", "query"]):
            tool_type = "database"
        elif any(
            keyword in tool_class for keyword in ["vector", "embedding", "retrieval"]
        ):
            tool_type = "vector_store"
        else:
            tool_type = "custom"

        # Use standard tool type attribute from semcov
        span.set_attribute(SemanticConvention.GEN_AI_TOOL_TYPE, tool_type)

    except Exception as e:
        handle_exception(span, e)


def _capture_content(span, instance, response, endpoint):
    """Capture input/output content with MIME types"""

    try:
        # Capture response content
        if response:
            span.add_event(
                name=SemanticConvention.GEN_AI_CONTENT_COMPLETION_EVENT,
                attributes={
                    SemanticConvention.GEN_AI_CONTENT_COMPLETION: str(response)[
                        :1000
                    ],  # Limit size
                },
            )

        # Capture input content based on operation type
        if endpoint.startswith("task_"):
            task_description = getattr(instance, "description", "")
            if task_description:
                span.add_event(
                    name=SemanticConvention.GEN_AI_CONTENT_PROMPT_EVENT,
                    attributes={
                        SemanticConvention.GEN_AI_CONTENT_PROMPT: task_description[
                            :1000
                        ],
                    },
                )

    except Exception:
        # Graceful degradation
        pass


def _track_cost_and_tokens(span, instance, response, endpoint):
    """Track cost and token usage for business intelligence"""

    try:
        # Token tracking from LLM calls
        if hasattr(instance, "llm") and hasattr(instance.llm, "get_num_tokens"):
            # This would be framework-specific implementation
            pass

        # Response length as a proxy metric and token estimation
        if response:
            response_length = len(str(response))
            # Estimate token count (rough approximation: 4 chars per token)
            estimated_tokens = response_length // 4
            span.set_attribute(
                SemanticConvention.GEN_AI_CLIENT_TOKEN_USAGE, estimated_tokens
            )

        # Cost estimation would require pricing information
        # This could be enhanced with actual cost tracking

    except Exception:
        # Graceful degradation
        pass


def _record_crewai_metrics(
    metrics, operation_type, duration_ms, environment, application_name
):
    """Record CrewAI-specific metrics"""

    try:
        attributes = {
            "gen_ai.operation.name": operation_type,
            "gen_ai.system": SemanticConvention.GEN_AI_SYSTEM_CREWAI,
            "service.name": application_name,
            "deployment.environment": environment,
        }

        # Record operation duration
        if "genai_client_operation_duration" in metrics:
            metrics["genai_client_operation_duration"].record(
                duration_ms / 1000, attributes
            )

        # Record operation count
        if "genai_requests" in metrics:
            metrics["genai_requests"].add(1, attributes)

    except Exception:
        # Graceful degradation
        pass


def _parse_tools(tools):
    """Parse tools list into JSON format"""

    try:
        result = []
        for tool in tools:
            tool_info = {}
            if hasattr(tool, "name") and tool.name is not None:
                tool_info["name"] = tool.name
            if hasattr(tool, "description") and tool.description is not None:
                tool_info["description"] = tool.description
            if tool_info:
                result.append(tool_info)
        return json.dumps(result)
    except Exception:
        return "[]"
