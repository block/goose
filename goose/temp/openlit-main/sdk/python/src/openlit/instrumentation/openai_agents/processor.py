"""
OpenLIT OpenAI Agents Instrumentation - Native TracingProcessor Implementation
"""

import time
from typing import Any, Dict, TYPE_CHECKING

from opentelemetry.trace import SpanKind, Status, StatusCode, set_span_in_context

from openlit.__helpers import (
    common_framework_span_attributes,
    handle_exception,
    get_chat_model_cost,
)
from openlit.semcov import SemanticConvention

# Try to import agents framework components with fallback
try:
    from agents import TracingProcessor

    if TYPE_CHECKING:
        from agents import Trace, Span
    TRACING_AVAILABLE = True
except ImportError:
    # Create dummy class for when agents is not available
    class TracingProcessor:
        """Dummy TracingProcessor class for when agents is not available"""

        def force_flush(self):
            """Dummy force_flush method"""
            return None

        def shutdown(self):
            """Dummy shutdown method"""
            return None

    if TYPE_CHECKING:
        # Type hints only - these don't exist at runtime when agents unavailable
        Trace = Any
        Span = Any

    TRACING_AVAILABLE = False


class OpenLITTracingProcessor(TracingProcessor):
    """
    OpenAI Agents tracing processor that integrates with OpenLIT observability.

    This processor enhances OpenAI Agents' native tracing system with OpenLIT's
    comprehensive observability features including business intelligence,
    cost tracking, and performance metrics.
    """

    def __init__(
        self,
        tracer,
        version,
        environment,
        application_name,
        pricing_info,
        capture_message_content,
        metrics,
        disable_metrics,
        detailed_tracing,
        **kwargs,
    ):
        """Initialize the OpenLIT tracing processor."""
        super().__init__()

        # Core configuration
        self.tracer = tracer
        self.version = version
        self.environment = environment
        self.application_name = application_name
        self.pricing_info = pricing_info
        self.capture_message_content = capture_message_content
        self.metrics = metrics
        self.disable_metrics = disable_metrics
        self.detailed_tracing = detailed_tracing

        # Internal tracking
        self.active_spans = {}
        self.span_stack = []

    def start_trace(self, trace_id: str, name: str, **kwargs):
        """
        Start a new trace with OpenLIT enhancements.

        Args:
            trace_id: Unique trace identifier
            name: Trace name
            **kwargs: Additional trace metadata
        """
        try:
            # Generate span name using OpenTelemetry conventions
            span_name = self._get_span_name(name, **kwargs)

            # Start root span with OpenLIT context
            span = self.tracer.start_as_current_span(
                span_name,
                kind=SpanKind.CLIENT,
                attributes={
                    SemanticConvention.GEN_AI_SYSTEM: "openai_agents",
                    SemanticConvention.GEN_AI_OPERATION: SemanticConvention.GEN_AI_OPERATION_TYPE_FRAMEWORK,
                    "trace.id": trace_id,
                    "trace.name": name,
                },
            )

            # Create scope for common attributes
            scope = type("GenericScope", (), {})()
            scope._span = span  # pylint: disable=protected-access
            scope._start_time = time.time()  # pylint: disable=protected-access
            scope._end_time = None  # pylint: disable=protected-access

            # Apply common framework attributes
            common_framework_span_attributes(
                scope,
                "openai_agents",
                "api.openai.com",
                443,
                self.environment,
                self.application_name,
                self.version,
                name,
            )

            # Track active span
            self.active_spans[trace_id] = span
            self.span_stack.append(span)

            return span

        except Exception as e:  # pylint: disable=broad-exception-caught
            # Graceful degradation
            handle_exception(None, e)
            return None

    def end_trace(self, trace_id: str, **kwargs):
        """
        End an active trace.

        Args:
            trace_id: Trace identifier to end
            **kwargs: Additional metadata
        """
        try:
            span = self.active_spans.get(trace_id)
            if span:
                # Set final attributes and status
                span.set_status(Status(StatusCode.OK))

                # End span
                span.end()

                # Cleanup tracking
                if trace_id in self.active_spans:
                    del self.active_spans[trace_id]
                if span in self.span_stack:
                    self.span_stack.remove(span)

        except Exception as e:  # pylint: disable=broad-exception-caught
            handle_exception(span if span else None, e)

    def _get_span_name(self, operation_name: str, **metadata) -> str:
        """
        Generate OpenTelemetry-compliant span names.

        Args:
            operation_name: Base operation name
            **metadata: Additional context for naming

        Returns:
            Formatted span name following semantic conventions
        """
        # Extract context for naming
        agent_name = metadata.get("agent_name", "")
        model_name = metadata.get("model_name", "")
        tool_name = metadata.get("tool_name", "")
        workflow_name = metadata.get("workflow_name", "")

        # Apply OpenTelemetry semantic conventions for GenAI agents
        if "agent" in operation_name.lower():
            if agent_name:
                return f"invoke_agent {agent_name}"
            return "invoke_agent"
        if "chat" in operation_name.lower():
            if model_name:
                return f"chat {model_name}"
            return "chat response"
        if "tool" in operation_name.lower():
            if tool_name:
                return f"execute_tool {tool_name}"
            return "execute_tool"
        if "handoff" in operation_name.lower():
            target_agent = metadata.get("target_agent", "unknown")
            return f"invoke_agent {target_agent}"
        if "workflow" in operation_name.lower():
            if workflow_name:
                return f"workflow {workflow_name}"
            return "workflow"

        # Default case
        return operation_name

    def span_start(self, span_data, trace_id: str):
        """
        Handle span start events from OpenAI Agents.

        Args:
            span_data: Span data from agents framework
            trace_id: Associated trace identifier
        """
        try:
            # Extract span information
            span_name = getattr(span_data, "name", "unknown_operation")
            span_type = getattr(span_data, "type", "unknown")

            # Generate enhanced span name
            enhanced_name = self._get_span_name(
                span_name,
                agent_name=getattr(span_data, "agent_name", None),
                model_name=getattr(span_data, "model_name", None),
                tool_name=getattr(span_data, "tool_name", None),
            )

            # Determine span operation type
            operation_type = self._get_operation_type(span_type, span_name)

            # Start span with proper context
            parent_span = self.span_stack[-1] if self.span_stack else None
            context = set_span_in_context(parent_span) if parent_span else None

            span = self.tracer.start_as_current_span(
                enhanced_name,
                kind=SpanKind.CLIENT,
                context=context,
                attributes={
                    SemanticConvention.GEN_AI_SYSTEM: "openai_agents",
                    SemanticConvention.GEN_AI_OPERATION: operation_type,
                    "span.type": span_type,
                    "span.id": getattr(span_data, "span_id", ""),
                },
            )

            # Process specific span types
            self._process_span_attributes(span, span_data, span_type)

            # Track span
            span_id = getattr(span_data, "span_id", len(self.span_stack))
            self.active_spans[f"{trace_id}:{span_id}"] = span
            self.span_stack.append(span)

        except Exception as e:  # pylint: disable=broad-exception-caught
            handle_exception(None, e)

    def _get_operation_type(self, span_type: str, span_name: str) -> str:
        """Get operation type based on span characteristics."""
        type_mapping = {
            "agent": SemanticConvention.GEN_AI_OPERATION_TYPE_AGENT,
            "generation": SemanticConvention.GEN_AI_OPERATION_TYPE_CHAT,
            "function": SemanticConvention.GEN_AI_OPERATION_TYPE_CHAT,
            "tool": SemanticConvention.GEN_AI_OPERATION_TYPE_CHAT,
            "handoff": SemanticConvention.GEN_AI_OPERATION_TYPE_CHAT,
        }

        # Check span type first
        for key, operation in type_mapping.items():
            if key in span_type.lower():
                return operation

        # Check span name
        for key, operation in type_mapping.items():
            if key in span_name.lower():
                return operation

        return SemanticConvention.GEN_AI_OPERATION_TYPE_CHAT

    def _process_span_attributes(self, span, span_data, span_type: str):
        """Process and set span attributes based on span type."""
        try:
            # Common attributes
            if hasattr(span_data, "agent_name"):
                span.set_attribute(
                    SemanticConvention.GEN_AI_AGENT_NAME, span_data.agent_name
                )

            if hasattr(span_data, "model_name"):
                span.set_attribute(
                    SemanticConvention.GEN_AI_REQUEST_MODEL, span_data.model_name
                )

            # Agent-specific attributes
            if span_type == "agent":
                self._process_agent_span(span, span_data)

            # Generation-specific attributes
            elif span_type == "generation":
                self._process_generation_span(span, span_data)

            # Function/Tool-specific attributes
            elif span_type in ["function", "tool"]:
                self._process_function_span(span, span_data)

            # Handoff-specific attributes
            elif span_type == "handoff":
                self._process_handoff_span(span, span_data)

        except Exception as e:  # pylint: disable=broad-exception-caught
            handle_exception(span, e)

    def _process_agent_span(self, span, agent_span):
        """Process agent span data (unused parameter)."""
        # Agent-specific processing
        if hasattr(agent_span, "instructions"):
            span.set_attribute(
                SemanticConvention.GEN_AI_AGENT_DESCRIPTION,
                str(agent_span.instructions)[:500],
            )

        if hasattr(agent_span, "model"):
            span.set_attribute(
                SemanticConvention.GEN_AI_REQUEST_MODEL, agent_span.model
            )

    def _process_generation_span(self, span, generation_span):
        """Process generation span data."""
        # Set generation-specific attributes
        if hasattr(generation_span, "prompt"):
            span.set_attribute(
                SemanticConvention.GEN_AI_PROMPT, str(generation_span.prompt)[:1000]
            )

        if hasattr(generation_span, "completion"):
            span.set_attribute(
                SemanticConvention.GEN_AI_COMPLETION,
                str(generation_span.completion)[:1000],
            )

        if hasattr(generation_span, "usage"):
            usage = generation_span.usage
            if hasattr(usage, "prompt_tokens"):
                span.set_attribute(
                    SemanticConvention.GEN_AI_USAGE_PROMPT_TOKENS, usage.prompt_tokens
                )
            if hasattr(usage, "completion_tokens"):
                span.set_attribute(
                    SemanticConvention.GEN_AI_USAGE_COMPLETION_TOKENS,
                    usage.completion_tokens,
                )

    def _process_function_span(self, span, function_span):
        """Process function/tool span data."""
        if hasattr(function_span, "function_name"):
            span.set_attribute(
                SemanticConvention.GEN_AI_TOOL_NAME, function_span.function_name
            )

        if hasattr(function_span, "arguments"):
            span.set_attribute(
                "gen_ai.tool.arguments", str(function_span.arguments)[:500]
            )

        if hasattr(function_span, "result"):
            span.set_attribute("gen_ai.tool.result", str(function_span.result)[:500])

    def _process_handoff_span(self, span, handoff_span):
        """Process handoff span data."""
        if hasattr(handoff_span, "target_agent"):
            span.set_attribute("gen_ai.handoff.target_agent", handoff_span.target_agent)

        if hasattr(handoff_span, "reason"):
            span.set_attribute("gen_ai.handoff.reason", str(handoff_span.reason)[:200])

    def span_end(self, span_data, trace_id: str):
        """Handle span end events."""
        try:
            span_id = getattr(span_data, "span_id", "")
            span_key = f"{trace_id}:{span_id}"

            span = self.active_spans.get(span_key)
            if span:
                # Set final status
                if hasattr(span_data, "error") and span_data.error:
                    span.set_status(Status(StatusCode.ERROR, str(span_data.error)))
                else:
                    span.set_status(Status(StatusCode.OK))

                # End span
                span.end()

                # Cleanup
                if span_key in self.active_spans:
                    del self.active_spans[span_key]
                if span in self.span_stack:
                    self.span_stack.remove(span)

        except Exception as e:  # pylint: disable=broad-exception-caught
            handle_exception(span if "span" in locals() else None, e)

    def force_flush(self):
        """Force flush all pending spans."""
        try:
            # End any remaining spans
            for span in list(self.active_spans.values()):
                span.end()

            self.active_spans.clear()
            self.span_stack.clear()

        except Exception as e:  # pylint: disable=broad-exception-caught
            handle_exception(None, e)

    def shutdown(self):
        """Shutdown the processor."""
        self.force_flush()

    def _extract_model_info(self, span_data) -> Dict[str, Any]:
        """Extract model information from span data."""
        model_info = {}

        if hasattr(span_data, "model"):
            model_info["model"] = span_data.model
        if hasattr(span_data, "model_name"):
            model_info["model"] = span_data.model_name

        return model_info

    def _calculate_cost(
        self, model: str, prompt_tokens: int, completion_tokens: int
    ) -> float:
        """Calculate cost based on token usage."""
        try:
            return get_chat_model_cost(
                model, self.pricing_info, prompt_tokens, completion_tokens
            )
        except Exception:  # pylint: disable=broad-exception-caught
            return 0.0

    # Abstract method implementations required by OpenAI Agents framework
    def on_trace_start(self, trace):
        """Called when a trace starts - required by OpenAI Agents framework"""
        try:
            self.start_trace(
                getattr(trace, "trace_id", "unknown"),
                getattr(trace, "name", "workflow"),
            )
        except Exception:  # pylint: disable=broad-exception-caught
            pass

    def on_trace_end(self, trace):
        """Called when a trace ends - required by OpenAI Agents framework"""
        try:
            self.end_trace(getattr(trace, "trace_id", "unknown"))
        except Exception:  # pylint: disable=broad-exception-caught
            pass

    def on_span_start(self, span):
        """Called when a span starts - required by OpenAI Agents framework"""
        try:
            trace_id = getattr(span, "trace_id", "unknown")
            self.span_start(span, trace_id)
        except Exception:  # pylint: disable=broad-exception-caught
            pass

    def on_span_end(self, span):
        """Called when a span ends - required by OpenAI Agents framework"""
        try:
            trace_id = getattr(span, "trace_id", "unknown")
            self.span_end(span, trace_id)
        except Exception:  # pylint: disable=broad-exception-caught
            pass
