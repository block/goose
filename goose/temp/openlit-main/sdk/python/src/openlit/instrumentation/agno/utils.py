"""
Utility functions for Agno instrumentation following OpenLIT patterns.
"""

import importlib
import logging
import time
from openlit.__helpers import (
    common_span_attributes,
)
from openlit.semcov import SemanticConvention

# Initialize logger
logger = logging.getLogger(__name__)


# Simple utility functions following OpenLIT patterns


class _ScopeWrapper:
    """
    A scope wrapper class for common_span_attributes compatibility.

    This class wraps the necessary parameters required by the common_span_attributes
    function to maintain consistency with OpenLIT instrumentation patterns.
    """

    def __init__(self, span, instance, kwargs, args, start_time):
        """Initialize the scope wrapper with required parameters."""
        self._span = span
        self._instance = instance
        self._kwargs = kwargs
        self._args = args
        self._start_time = start_time
        self._end_time = time.time()

    def get_span(self):
        """Get the associated span."""
        return self._span

    def get_instance(self):
        """Get the instrumented instance."""
        return self._instance


def process_agent_request(
    span,
    instance,
    args,
    kwargs,
    response,
    start_time,
    pricing_info,
    environment,
    application_name,
    metrics,
    capture_message_content,
    disable_metrics,
    version,
    operation_type=SemanticConvention.GEN_AI_OPERATION_TYPE_AGENT,
):
    """
    Process agent request and generate ALL telemetry attributes from semcov (CrewAI pattern)
    """

    # Calculate execution time
    execution_time = time.time() - start_time

    # Extract agent information
    agent_name = (
        getattr(instance, "name", None)
        or getattr(instance, "agent_id", None)
        or "default_agent"
    )
    request_model = None

    # Get model information
    if hasattr(instance, "model"):
        model = instance.model
        if hasattr(model, "id"):
            request_model = str(model.id)
        elif hasattr(model, "name"):
            request_model = str(model.name)

    # Create scope object for common_span_attributes compatibility
    scope = _ScopeWrapper(span, instance, kwargs, args, start_time)

    # Use common span attributes helper from __helpers
    common_span_attributes(
        scope,
        operation_type,
        SemanticConvention.GEN_AI_SYSTEM_AGNO,
        "localhost",  # server_address
        80,  # server_port
        request_model,
        request_model,  # response_model same as request
        environment,
        application_name,
        kwargs.get("stream", False),  # is_stream
        0,  # tbt
        0,  # ttft
        version,
    )

    # Set agno-specific attributes using semantic conventions
    span.set_attribute(SemanticConvention.GEN_AI_AGENT_NAME, agent_name)

    # Set lifecycle phase based on operation
    if operation_type == SemanticConvention.GEN_AI_OPERATION_TYPE_AGENT:
        span.set_attribute(
            SemanticConvention.GEN_AI_AGENT_LIFECYCLE_PHASE,
            SemanticConvention.GEN_AI_AGENT_LIFECYCLE_PHASE_EXECUTE,
        )
    elif operation_type == SemanticConvention.GEN_AI_OPERATION_TYPE_MEMORY:
        span.set_attribute(
            SemanticConvention.GEN_AI_AGENT_LIFECYCLE_PHASE,
            SemanticConvention.GEN_AI_AGENT_LIFECYCLE_PHASE_EXECUTE,
        )
    elif operation_type == SemanticConvention.GEN_AI_OPERATION_TYPE_TOOLS:
        span.set_attribute(
            SemanticConvention.GEN_AI_AGENT_LIFECYCLE_PHASE,
            SemanticConvention.GEN_AI_AGENT_LIFECYCLE_PHASE_TOOL_EXECUTION,
        )

    # Add detailed agent information if available
    if hasattr(instance, "instructions") and instance.instructions:
        span.set_attribute(
            SemanticConvention.GEN_AI_AGENT_INSTRUCTIONS,
            str(instance.instructions)[:500],
        )

    if hasattr(instance, "agent_id") and instance.agent_id:
        span.set_attribute(SemanticConvention.GEN_AI_AGENT_ID, instance.agent_id)

    if hasattr(instance, "description") and instance.description:
        span.set_attribute(
            SemanticConvention.GEN_AI_AGENT_DESCRIPTION, str(instance.description)[:500]
        )

    if hasattr(instance, "introduction") and instance.introduction:
        span.set_attribute(
            SemanticConvention.GEN_AI_AGENT_INTRODUCTION,
            str(instance.introduction)[:500],
        )

    # Add request-specific attributes from args and kwargs
    if args and args[0]:
        if capture_message_content:
            span.set_attribute(
                SemanticConvention.GEN_AI_CONTENT_PROMPT, str(args[0])[:1000]
            )

    # agno 2.x versions agent.run use input instead of args[0]
    if kwargs and kwargs.get("input", None) and capture_message_content:
        span.set_attribute(
            SemanticConvention.GEN_AI_CONTENT_PROMPT, str(kwargs["input"])[:1000]
        )
    # agno 2.x versions team.run use input_message instead of args[0]
    if kwargs and kwargs.get("input_message", None) and capture_message_content:
        span.set_attribute(
            SemanticConvention.GEN_AI_CONTENT_PROMPT,
            str(kwargs["input_message"])[:1000],
        )

    # User and session information
    if "user_id" in kwargs and kwargs["user_id"]:
        span.set_attribute(SemanticConvention.GEN_AI_REQUEST_USER, kwargs["user_id"])
    elif hasattr(instance, "user_id") and instance.user_id:
        span.set_attribute(SemanticConvention.GEN_AI_REQUEST_USER, instance.user_id)

    if "session_id" in kwargs and kwargs["session_id"]:
        span.set_attribute(SemanticConvention.GEN_AI_SESSION_ID, kwargs["session_id"])

    # agno 2.x versions use session instead of session_id
    if (
        "session" in kwargs
        and kwargs["session"]
        and hasattr(kwargs["session"], "session_id")
    ):
        span.set_attribute(
            SemanticConvention.GEN_AI_SESSION_ID, kwargs["session"].session_id
        )

    # Stream and reasoning attributes
    if "stream" in kwargs:
        span.set_attribute(
            SemanticConvention.GEN_AI_REQUEST_IS_STREAM, bool(kwargs["stream"])
        )

    if "show_full_reasoning" in kwargs:
        span.set_attribute(
            SemanticConvention.GEN_AI_AGENT_SHOW_REASONING,
            bool(kwargs["show_full_reasoning"]),
        )

    if "stream_intermediate_steps" in kwargs:
        span.set_attribute(
            SemanticConvention.GEN_AI_AGENT_STREAM_INTERMEDIATE_STEPS,
            bool(kwargs["stream_intermediate_steps"]),
        )

    # Set execution time using semantic convention
    span.set_attribute(SemanticConvention.GEN_AI_AGENT_EXECUTION_TIME, execution_time)

    # Process response with comprehensive output capture
    if response:
        if capture_message_content:
            if hasattr(response, "content") and response.content:
                content = str(response.content)[:2000]
                span.set_attribute(
                    SemanticConvention.GEN_AI_CONTENT_COMPLETION, content
                )
            elif hasattr(response, "message") and response.message:
                content = str(response.message)[:2000]
                span.set_attribute(
                    SemanticConvention.GEN_AI_CONTENT_COMPLETION, content
                )

        # Capture additional response metadata
        if hasattr(response, "metrics"):
            span.set_attribute(
                SemanticConvention.GEN_AI_AGENT_RESPONSE_TIME, execution_time
            )


def process_tool_request(
    span,
    instance,
    args,
    kwargs,
    response,
    start_time,
    pricing_info,
    environment,
    application_name,
    metrics,
    capture_message_content,
    disable_metrics,
    version,
    tool_name,
):
    """
    Process tool execution request and generate ALL telemetry attributes from semcov
    """

    execution_time = time.time() - start_time

    # Create scope object for common_span_attributes compatibility
    scope = _ScopeWrapper(span, instance, kwargs, args, start_time)

    # Get model info from instance if available
    request_model = getattr(instance, "model", None)
    if request_model and hasattr(request_model, "id"):
        model_name = request_model.id
    else:
        model_name = "unknown"

    # Use common span attributes for tool operations
    common_span_attributes(
        scope,
        SemanticConvention.GEN_AI_OPERATION_TYPE_EXECUTE_AGENT_TASK,
        SemanticConvention.GEN_AI_SYSTEM_AGNO,
        "localhost",  # server_address
        80,  # server_port
        model_name,  # request_model
        model_name,  # response_model
        environment,
        application_name,
        False,  # is_stream
        0,  # tbt
        0,  # ttft
        version,
    )

    # Set tool-specific attributes using semcov
    span.set_attribute(SemanticConvention.GEN_AI_TOOL_NAME, tool_name)
    span.set_attribute(
        SemanticConvention.GEN_AI_AGENT_LIFECYCLE_PHASE,
        SemanticConvention.GEN_AI_AGENT_LIFECYCLE_PHASE_TOOL_EXECUTION,
    )

    # Set tool-specific attributes that aren't in the process function
    if (
        hasattr(instance, "function")
        and hasattr(instance.function, "description")
        and instance.function.description
    ):
        span.set_attribute(
            SemanticConvention.GEN_AI_TOOL_DESCRIPTION,
            str(instance.function.description)[:500],
        )

    if hasattr(instance, "arguments") and instance.arguments:
        span.set_attribute(
            SemanticConvention.GEN_AI_TOOL_PARAMETERS, str(instance.arguments)[:1000]
        )

    # Set execution metrics
    span.set_attribute(
        SemanticConvention.GEN_AI_TOOL_EXECUTION_DURATION, execution_time
    )

    # Process result
    if response and hasattr(response, "status"):
        span.set_attribute(
            SemanticConvention.GEN_AI_TOOL_EXECUTION_SUCCESS,
            response.status == "success",
        )
        if capture_message_content:
            if hasattr(response, "result") and response.result:
                span.set_attribute(
                    SemanticConvention.GEN_AI_TOOL_OUTPUT, str(response.result)[:1000]
                )
        if hasattr(response, "error") and response.error:
            span.set_attribute(
                SemanticConvention.GEN_AI_TOOL_ERROR, str(response.error)[:500]
            )


def process_memory_request(
    span,
    instance,
    args,
    kwargs,
    response,
    start_time,
    pricing_info,
    environment,
    application_name,
    metrics,
    capture_message_content,
    disable_metrics,
    version,
    memory_operation,
):
    """
    Process memory operation request and generate ALL telemetry attributes from semcov
    """

    execution_time = time.time() - start_time

    # Create scope object for common_span_attributes compatibility
    scope = _ScopeWrapper(span, instance, kwargs, args, start_time)

    # Use common span attributes for memory operations
    common_span_attributes(
        scope,
        SemanticConvention.GEN_AI_OPERATION_TYPE_MEMORY,
        SemanticConvention.GEN_AI_SYSTEM_AGNO,
        "localhost",  # server_address
        80,  # server_port
        None,  # request_model
        None,  # response_model
        environment,
        application_name,
        False,  # is_stream
        0,  # tbt
        0,  # ttft
        version,
    )

    # Set memory-specific attributes using semcov
    span.set_attribute(SemanticConvention.GEN_AI_MEMORY_OPERATION, memory_operation)
    span.set_attribute(
        SemanticConvention.GEN_AI_AGENT_LIFECYCLE_PHASE,
        SemanticConvention.GEN_AI_AGENT_LIFECYCLE_PHASE_EXECUTE,
    )

    # Set memory-specific attributes that aren't in the process function
    if hasattr(instance, "db") and instance.db:
        span.set_attribute("gen_ai.memory.db_type", type(instance.db).__name__)

    if hasattr(instance, "table_name"):
        span.set_attribute("gen_ai.memory.table_name", str(instance.table_name))

    # Add input data
    if args and capture_message_content:
        span.set_attribute(
            "gen_ai.memory.input", str(args[0])[:1000] if args[0] else ""
        )

    # Add metadata from kwargs
    if "user_id" in kwargs:
        span.set_attribute("gen_ai.memory.user_id", kwargs["user_id"])
    if "agent_id" in kwargs:
        span.set_attribute("gen_ai.memory.agent_id", kwargs["agent_id"])
    if "metadata" in kwargs:
        span.set_attribute("gen_ai.memory.metadata", str(kwargs["metadata"])[:500])

    # Set execution metrics
    span.set_attribute("gen_ai.memory.operation.duration", execution_time)

    # Process result
    if response:
        if hasattr(response, "id"):
            span.set_attribute("gen_ai.memory.result_id", str(response.id))
        span.set_attribute("gen_ai.memory.operation.success", True)
    else:
        span.set_attribute("gen_ai.memory.operation.success", False)


def process_reasoning_request(
    span,
    instance,
    args,
    kwargs,
    response,
    start_time,
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
    Process reasoning operation request and generate ALL telemetry attributes from semcov
    """

    execution_time = time.time() - start_time
    agent_name = getattr(instance, "name", None) or "unknown_agent"

    # Create scope object for common_span_attributes compatibility
    scope = _ScopeWrapper(span, instance, kwargs, args, start_time)

    # Get model info from agent instance if available
    request_model = getattr(instance, "model", None)
    if request_model and hasattr(request_model, "id"):
        model_name = request_model.id
    else:
        model_name = "unknown"

    # Use common span attributes for reasoning operations (use agent task operation type)
    common_span_attributes(
        scope,
        SemanticConvention.GEN_AI_OPERATION_TYPE_EXECUTE_AGENT_TASK,
        SemanticConvention.GEN_AI_SYSTEM_AGNO,
        "localhost",  # server_address
        80,  # server_port
        model_name,  # request_model
        model_name,  # response_model
        environment,
        application_name,
        False,  # is_stream
        0,  # tbt
        0,  # ttft
        version,
    )

    # Set reasoning-specific attributes using semcov
    span.set_attribute(
        SemanticConvention.GEN_AI_AGENT_LIFECYCLE_PHASE,
        SemanticConvention.GEN_AI_AGENT_LIFECYCLE_PHASE_EXECUTE,
    )
    span.set_attribute(SemanticConvention.GEN_AI_AGENT_NAME, agent_name)

    # Add reasoning-specific attributes
    if hasattr(instance, "reasoning_min_steps"):
        span.set_attribute(
            SemanticConvention.GEN_AI_REASONING_MIN_STEPS, instance.reasoning_min_steps
        )

    if hasattr(instance, "reasoning_max_steps"):
        span.set_attribute(
            SemanticConvention.GEN_AI_REASONING_MAX_STEPS, instance.reasoning_max_steps
        )

    if hasattr(instance, "reasoning_model") and instance.reasoning_model:
        reasoning_model_name = getattr(
            instance.reasoning_model, "name", str(instance.reasoning_model)
        )
        span.set_attribute(
            SemanticConvention.GEN_AI_REASONING_MODEL, reasoning_model_name
        )

    # Set execution time
    span.set_attribute(
        SemanticConvention.GEN_AI_REASONING_EXECUTION_DURATION, execution_time
    )


def process_vectordb_request(
    span,
    instance,
    args,
    kwargs,
    response,
    start_time,
    pricing_info,
    environment,
    application_name,
    metrics,
    capture_message_content,
    disable_metrics,
    version,
    operation_type="vectordb_search",
):
    """
    Process vectordb operation request and generate ALL telemetry attributes from semcov
    """

    execution_time = time.time() - start_time

    # Create scope object for common_span_attributes compatibility
    scope = _ScopeWrapper(span, instance, kwargs, args, start_time)

    # Use common span attributes for vectordb operations
    common_span_attributes(
        scope,
        f"gen_ai.operation.{operation_type}",
        SemanticConvention.GEN_AI_SYSTEM_AGNO,
        "localhost",  # server_address
        80,  # server_port
        None,  # request_model
        None,  # response_model
        environment,
        application_name,
        False,  # is_stream
        0,  # tbt
        0,  # ttft
        version,
    )

    # Set vectordb-specific attributes using semcov
    if hasattr(instance, "name"):
        span.set_attribute(SemanticConvention.GEN_AI_VECTORDB_NAME, str(instance.name))

    if hasattr(instance, "dimensions"):
        span.set_attribute(
            SemanticConvention.GEN_AI_VECTORDB_DIMENSIONS, instance.dimensions
        )

    # Search-specific attributes
    if operation_type == "vectordb_search" and args:
        query = args[0] if args else None
        if query:
            if isinstance(query, str):
                span.set_attribute(
                    SemanticConvention.GEN_AI_VECTORDB_SEARCH_QUERY, query[:500]
                )
            elif hasattr(query, "__len__"):
                span.set_attribute(
                    SemanticConvention.GEN_AI_VECTORDB_SEARCH_VECTOR_SIZE, len(query)
                )

        limit = kwargs.get("limit", args[1] if len(args) > 1 else None)
        if limit:
            span.set_attribute(SemanticConvention.GEN_AI_VECTORDB_SEARCH_LIMIT, limit)

    # Upsert-specific attributes
    if operation_type == "vectordb_upsert" and args:
        documents = args[0] if args else None
        if documents and hasattr(documents, "__len__"):
            span.set_attribute(
                SemanticConvention.GEN_AI_VECTORDB_UPSERT_DOCUMENT_COUNT, len(documents)
            )

    # Set execution metrics
    span.set_attribute(
        SemanticConvention.GEN_AI_VECTORDB_OPERATION_DURATION, execution_time
    )

    # Process result
    if response:
        span.set_attribute(SemanticConvention.GEN_AI_VECTORDB_OPERATION_SUCCESS, True)
        if operation_type == "vectordb_search" and hasattr(response, "__len__"):
            span.set_attribute(
                SemanticConvention.GEN_AI_VECTORDB_SEARCH_RESULTS_COUNT, len(response)
            )
            if response and hasattr(response[0], "score"):
                scores = [r.score for r in response[:5]]  # Top 5 scores
                span.set_attribute(
                    SemanticConvention.GEN_AI_VECTORDB_SEARCH_TOP_SCORES, scores
                )
    else:
        span.set_attribute(SemanticConvention.GEN_AI_VECTORDB_OPERATION_SUCCESS, False)


def process_knowledge_request(
    span,
    instance,
    args,
    kwargs,
    response,
    start_time,
    pricing_info,
    environment,
    application_name,
    metrics,
    capture_message_content,
    disable_metrics,
    version,
    operation_type="knowledge_search",
):
    """
    Process knowledge operation request and generate ALL telemetry attributes from semcov
    """

    execution_time = time.time() - start_time

    # Create scope object for common_span_attributes compatibility
    scope = _ScopeWrapper(span, instance, kwargs, args, start_time)

    # Use common span attributes for knowledge operations
    common_span_attributes(
        scope,
        f"gen_ai.operation.{operation_type}",
        SemanticConvention.GEN_AI_SYSTEM_AGNO,
        "localhost",  # server_address
        80,  # server_port
        None,  # request_model
        None,  # response_model
        environment,
        application_name,
        False,  # is_stream
        0,  # tbt
        0,  # ttft
        version,
    )

    # Search-specific attributes
    if operation_type == "knowledge_search":
        query = args[0] if args else None
        if query and capture_message_content:
            span.set_attribute(
                SemanticConvention.GEN_AI_KNOWLEDGE_SEARCH_QUERY, query[:500]
            )

        limit = kwargs.get("limit", args[1] if len(args) > 1 else 5)
        span.set_attribute(SemanticConvention.GEN_AI_KNOWLEDGE_SEARCH_LIMIT, limit)

    # Add-specific attributes
    if operation_type == "knowledge_add" and args:
        documents = args[0] if args else None
        if documents:
            if hasattr(documents, "__len__"):
                span.set_attribute(
                    SemanticConvention.GEN_AI_KNOWLEDGE_ADD_DOCUMENT_COUNT,
                    len(documents),
                )
            else:
                span.set_attribute(
                    SemanticConvention.GEN_AI_KNOWLEDGE_ADD_CONTENT_LENGTH,
                    len(str(documents)),
                )

    # Set execution metrics
    span.set_attribute(
        SemanticConvention.GEN_AI_KNOWLEDGE_OPERATION_DURATION, execution_time
    )

    # Process result
    if response:
        span.set_attribute(SemanticConvention.GEN_AI_KNOWLEDGE_OPERATION_SUCCESS, True)
        if operation_type == "knowledge_search" and hasattr(response, "__len__"):
            span.set_attribute(
                SemanticConvention.GEN_AI_KNOWLEDGE_SEARCH_RESULTS_COUNT, len(response)
            )
    else:
        span.set_attribute(SemanticConvention.GEN_AI_KNOWLEDGE_OPERATION_SUCCESS, False)


def process_workflow_request(
    span,
    instance,
    args,
    kwargs,
    response,
    start_time,
    pricing_info,
    environment,
    application_name,
    metrics,
    capture_message_content,
    disable_metrics,
    version,
):
    """
    Process workflow operation request and generate ALL telemetry attributes from semcov
    """

    execution_time = time.time() - start_time
    workflow_name = getattr(instance, "name", "unknown_workflow")

    # Create scope object for common_span_attributes compatibility
    scope = _ScopeWrapper(span, instance, kwargs, args, start_time)

    # Use common span attributes for workflow operations
    common_span_attributes(
        scope,
        SemanticConvention.GEN_AI_OPERATION_TYPE_FRAMEWORK,
        SemanticConvention.GEN_AI_SYSTEM_AGNO,
        "localhost",  # server_address
        80,  # server_port
        None,  # request_model
        None,  # response_model
        environment,
        application_name,
        False,  # is_stream
        0,  # tbt
        0,  # ttft
        version,
    )

    # Set workflow-specific attributes using semcov
    span.set_attribute(SemanticConvention.GEN_AI_WORKFLOW_NAME, workflow_name)

    if hasattr(instance, "agents") and instance.agents:
        span.set_attribute(
            SemanticConvention.GEN_AI_WORKFLOW_AGENT_COUNT, len(instance.agents)
        )

    if hasattr(instance, "description") and instance.description:
        span.set_attribute(
            SemanticConvention.GEN_AI_WORKFLOW_DESCRIPTION,
            str(instance.description)[:300],
        )

    # Set execution metrics
    span.set_attribute(
        SemanticConvention.GEN_AI_WORKFLOW_EXECUTION_DURATION, execution_time
    )
    span.set_attribute(
        SemanticConvention.GEN_AI_WORKFLOW_OPERATION_SUCCESS, response is not None
    )


def process_team_request(
    span,
    instance,
    args,
    kwargs,
    response,
    start_time,
    pricing_info,
    environment,
    application_name,
    metrics,
    capture_message_content,
    disable_metrics,
    version,
):
    """
    Process team operation request and generate ALL telemetry attributes from semcov
    """

    execution_time = time.time() - start_time
    team_name = getattr(instance, "name", "unknown_team")

    # Create scope object for common_span_attributes compatibility
    scope = _ScopeWrapper(span, instance, kwargs, args, start_time)

    # Use common span attributes for team operations
    common_span_attributes(
        scope,
        SemanticConvention.GEN_AI_OPERATION_TYPE_TEAM,
        SemanticConvention.GEN_AI_SYSTEM_AGNO,
        "localhost",  # server_address
        80,  # server_port
        None,  # request_model
        None,  # response_model
        environment,
        application_name,
        False,  # is_stream
        0,  # tbt
        0,  # ttft
        version,
    )

    # Set team-specific attributes using semcov
    span.set_attribute(SemanticConvention.GEN_AI_TEAM_NAME, team_name)

    if hasattr(instance, "agents") and instance.agents:
        agent_names = [getattr(agent, "name", "unknown") for agent in instance.agents]
        span.set_attribute(
            SemanticConvention.GEN_AI_TEAM_AGENTS, agent_names[:10]
        )  # Limit to 10
        span.set_attribute(
            SemanticConvention.GEN_AI_TEAM_AGENT_COUNT, len(instance.agents)
        )

    if hasattr(instance, "members") and instance.members:
        agent_names = [getattr(agent, "name", "unknown") for agent in instance.members]
        span.set_attribute(
            SemanticConvention.GEN_AI_TEAM_AGENTS, agent_names[:10]
        )  # Limit to 10
        span.set_attribute(
            SemanticConvention.GEN_AI_TEAM_AGENT_COUNT, len(instance.members)
        )

    # Set execution metrics
    span.set_attribute(
        SemanticConvention.GEN_AI_TEAM_EXECUTION_DURATION, execution_time
    )
    span.set_attribute(
        SemanticConvention.GEN_AI_TEAM_OPERATION_SUCCESS, response is not None
    )


def resolve_agno_memory_target():
    """
    Resolve the appropriate Agno memory target class.

    Attempts to find and import the correct memory class from Agno framework.
    Tries multiple candidate modules in order of preference.

    Returns:
        tuple: (module_name, class_name) if successful, (None, None) if no valid target found

    Example:
        >>> module, class_name = resolve_agno_memory_target()
        >>> if module:
        ...     print(f"Found memory class: {class_name} in {module}")
    """
    return resolve_target(
        "agno.memory",
        (
            ("agno.memory.v2.memory", "Memory"),
            ("agno.memory.manager", "MemoryManager"),
        ),
    )


def resolve_agno_knowledge_target():
    """
    Resolve the appropriate Agno knowledge target class.

    Attempts to find and import the correct knowledge class from Agno framework.
    Tries multiple candidate modules in order of preference.

    Returns:
        tuple: (module_name, class_name) if successful, (None, None) if no valid target found

    Example:
        >>> module, class_name = resolve_agno_knowledge_target()
        >>> if module:
        ...     print(f"Found knowledge class: {class_name} in {module}")
    """
    return resolve_target(
        "agno.knowledge",
        (
            ("agno.knowledge.agent", "AgentKnowledge"),
            ("agno.knowledge.knowledge", "Knowledge"),
        ),
    )


def resolve_target(target_name, candidates):
    """
    Resolve a target class by trying multiple candidate modules.

    This function attempts to import and find a specific class from a list of candidate
    modules. It tries each candidate in order until it finds a valid module and class.

    Args:
        target_name (str): Name of the target being resolved (used for logging)
        candidates (tuple): Tuple of (module_name, class_name) pairs to try

    Returns:
        tuple: (module_name, class_name) if successful, (None, None) if no valid target found

    Example:
        >>> candidates = (
        ...     ("agno.memory.v2.memory", "Memory"),
        ...     ("agno.memory.manager", "MemoryManager")
        ... )
        >>> module, class_name = resolve_target("agno.memory", candidates)
        >>> if module:
        ...     print(f"Found: {class_name} in {module}")
    """
    for module_name, class_name in candidates:
        try:
            module = importlib.import_module(module_name)
            if hasattr(module, class_name):
                return module_name, class_name
        except ModuleNotFoundError:
            continue
        except Exception as e:
            logger.info(f"Skip {target_name} candidate %s due to: %s", module_name, e)
            continue
    return None, None
