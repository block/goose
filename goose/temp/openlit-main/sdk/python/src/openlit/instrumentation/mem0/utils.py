# pylint: disable=duplicate-code
"""
Shared utilities for mem0 instrumentation.
"""

import concurrent.futures
import functools
import json
import logging
from typing import Any, Dict, Optional

from opentelemetry import context
from opentelemetry.sdk.resources import (
    SERVICE_NAME,
    TELEMETRY_SDK_NAME,
    DEPLOYMENT_ENVIRONMENT,
)
from openlit.semcov import SemanticConvention

# Initialize logger
logger = logging.getLogger(__name__)


# Threading context propagation utility
def patch_concurrent_futures_context(current_context):
    """
    Patches ThreadPoolExecutor.submit to propagate OpenTelemetry context
    across thread boundaries for proper mem0 span hierarchy.

    Args:
        current_context: OpenTelemetry context to propagate

    Returns:
        Function to restore original ThreadPoolExecutor.submit
    """
    original_submit = concurrent.futures.ThreadPoolExecutor.submit

    def patched_submit(executor_self, fn, *args, **kwargs):
        @functools.wraps(fn)
        def context_wrapped_fn(*fn_args, **fn_kwargs):
            token = context.attach(current_context)
            try:
                return fn(*fn_args, **fn_kwargs)
            finally:
                context.detach(token)

        return original_submit(executor_self, context_wrapped_fn, *args, **kwargs)

    concurrent.futures.ThreadPoolExecutor.submit = patched_submit
    return lambda: setattr(
        concurrent.futures.ThreadPoolExecutor, "submit", original_submit
    )


class Mem0Context:
    """Optimized context object with lazy loading and __slots__ for performance."""

    __slots__ = (
        "instance",
        "args",
        "kwargs",
        "version",
        "environment",
        "application_name",
        "_user_id",
        "_agent_id",
        "_run_id",
        "_metadata",
        "_memory_type",
        "_messages",
    )

    def __init__(self, instance, args, kwargs, version, environment, application_name):
        self.instance = instance
        self.args = args
        self.kwargs = kwargs
        self.version = version
        self.environment = environment
        self.application_name = application_name

        # Lazy-loaded cached properties
        self._user_id = None
        self._agent_id = None
        self._run_id = None
        self._metadata = None
        self._memory_type = None
        self._messages = None

    @property
    def user_id(self) -> Optional[str]:
        """Get user_id with lazy loading."""
        if self._user_id is None:
            self._user_id = self.kwargs.get("user_id")
        return self._user_id

    @property
    def agent_id(self) -> Optional[str]:
        """Get agent_id with lazy loading."""
        if self._agent_id is None:
            self._agent_id = self.kwargs.get("agent_id")
        return self._agent_id

    @property
    def run_id(self) -> Optional[str]:
        """Get run_id with lazy loading."""
        if self._run_id is None:
            self._run_id = self.kwargs.get("run_id")
        return self._run_id

    @property
    def metadata(self) -> Optional[Dict[str, Any]]:
        """Get metadata with lazy loading."""
        if self._metadata is None:
            self._metadata = self.kwargs.get("metadata")
        return self._metadata

    @property
    def memory_type(self) -> Optional[str]:
        """Get memory_type with lazy loading."""
        if self._memory_type is None:
            self._memory_type = self.kwargs.get("memory_type")
        return self._memory_type

    @property
    def messages(self) -> Any:
        """Get messages with lazy loading."""
        if self._messages is None:
            self._messages = self.args[0] if self.args else self.kwargs.get("messages")
        return self._messages


def set_mem0_span_attributes(
    span, operation_name: str, ctx: Mem0Context, response: Any = None
):
    """Set optimized span attributes with batched operations."""

    # Batch core attributes for performance
    core_attributes = {
        TELEMETRY_SDK_NAME: "openlit",
        SemanticConvention.GEN_AI_SYSTEM: SemanticConvention.GEN_AI_SYSTEM_MEM0,
        SemanticConvention.GEN_AI_ENDPOINT: operation_name,
        DEPLOYMENT_ENVIRONMENT: ctx.environment,
        SERVICE_NAME: ctx.application_name,
        SemanticConvention.GEN_AI_SDK_VERSION: ctx.version,
    }

    # Set core attributes in batch
    for key, value in core_attributes.items():
        span.set_attribute(key, value)

    # Determine operation type efficiently
    is_internal = any(
        marker in operation_name
        for marker in ["_", "vector_store", "graph", "create_memory", "procedural"]
    )

    operation_type = (
        SemanticConvention.GEN_AI_OPERATION_TYPE_FRAMEWORK
        if is_internal
        else SemanticConvention.GEN_AI_OPERATION_TYPE_MEMORY
    )
    span.set_attribute(SemanticConvention.GEN_AI_OPERATION, operation_type)

    # Component type mapping for internal operations
    if is_internal:
        component_map = {
            "vector_store": "vector_store",
            "graph": "graph_store",
            "create_memory": "memory_creation",
            "procedural": "procedural_memory",
        }
        for key, component_type in component_map.items():
            if key in operation_name:
                span.set_attribute(
                    SemanticConvention.GEN_AI_FRAMEWORK_COMPONENT_TYPE, component_type
                )
                break

    # Session attributes
    session_attrs = {}
    if ctx.user_id:
        session_attrs[SemanticConvention.GEN_AI_USER_ID] = ctx.user_id
    if ctx.agent_id:
        session_attrs[SemanticConvention.GEN_AI_AGENT_ID] = ctx.agent_id
    if ctx.run_id:
        session_attrs[SemanticConvention.GEN_AI_RUN_ID] = ctx.run_id

    for key, value in session_attrs.items():
        span.set_attribute(key, value)

    # Memory-specific attributes
    if ctx.memory_type:
        span.set_attribute(SemanticConvention.GEN_AI_MEMORY_TYPE, ctx.memory_type)

    if ctx.metadata:
        try:
            span.set_attribute(
                SemanticConvention.GEN_AI_MEMORY_METADATA, json.dumps(ctx.metadata)
            )
        except Exception:
            span.set_attribute(
                SemanticConvention.GEN_AI_MEMORY_METADATA, str(ctx.metadata)
            )

    # Operation-specific attributes
    _set_operation_attributes(span, operation_name, ctx)

    # Response attributes
    _set_response_attributes(span, response)


def _set_operation_attributes(span, operation_name: str, ctx: Mem0Context):
    """Set operation-specific attributes efficiently."""
    if "add" in operation_name:
        if "infer" in ctx.kwargs:
            span.set_attribute(
                SemanticConvention.GEN_AI_MEMORY_INFER, ctx.kwargs["infer"]
            )
        if ctx.messages:
            try:
                count = (
                    len(ctx.messages) if isinstance(ctx.messages, (list, tuple)) else 1
                )
                span.set_attribute(SemanticConvention.GEN_AI_MEMORY_COUNT, count)
            except Exception:
                pass

    elif "search" in operation_name:
        search_attrs = {}
        if "query" in ctx.kwargs:
            search_attrs[SemanticConvention.GEN_AI_MEMORY_SEARCH_QUERY] = ctx.kwargs[
                "query"
            ]
        if "limit" in ctx.kwargs:
            search_attrs[SemanticConvention.GEN_AI_MEMORY_SEARCH_LIMIT] = ctx.kwargs[
                "limit"
            ]
        if "threshold" in ctx.kwargs:
            search_attrs[SemanticConvention.GEN_AI_MEMORY_SEARCH_THRESHOLD] = (
                ctx.kwargs["threshold"]
            )

        for key, value in search_attrs.items():
            span.set_attribute(key, value)

    elif "update" in operation_name and "memory_id" in ctx.kwargs:
        span.set_attribute(SemanticConvention.DB_UPDATE_ID, ctx.kwargs["memory_id"])

    elif "delete" in operation_name and "memory_id" in ctx.kwargs:
        span.set_attribute(SemanticConvention.DB_DELETE_ID, ctx.kwargs["memory_id"])

    elif "get" in operation_name and "memory_id" in ctx.kwargs:
        span.set_attribute(SemanticConvention.DB_OPERATION_ID, ctx.kwargs["memory_id"])


def _set_response_attributes(span, response: Any):
    """Set response attributes for business intelligence."""
    if not response:
        return

    try:
        if isinstance(response, (list, tuple)):
            count = len(response)
        elif isinstance(response, dict):
            count = len(response.get("results", [])) if response.get("results") else 1
        else:
            count = 1

        span.set_attribute(
            SemanticConvention.GEN_AI_MEMORY_OPERATION_RESULT_COUNT, count
        )
        span.set_attribute(SemanticConvention.GEN_AI_DATA_SOURCES, count)
    except Exception:
        pass


def set_mem0_content_attributes(
    span, operation_name: str, ctx: Mem0Context, response: Any, capture_content: bool
):
    """Set content attributes if enabled."""
    if not capture_content:
        return

    try:
        if ctx.messages and "add" in operation_name:
            content = (
                json.dumps(ctx.messages)
                if isinstance(ctx.messages, (list, dict))
                else str(ctx.messages)
            )
            span.set_attribute(SemanticConvention.GEN_AI_CONTENT_PROMPT, content)

        if response and "search" in operation_name:
            content = (
                json.dumps(response)
                if isinstance(response, (dict, list))
                else str(response)
            )
            span.set_attribute(SemanticConvention.GEN_AI_CONTENT_COMPLETION, content)
    except Exception as e:
        logger.debug("Failed to capture content: %s", e)
