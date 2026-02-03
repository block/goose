"""
OpenLIT OpenAI Agents Instrumentation
"""

from typing import Collection
import importlib.metadata
from opentelemetry.instrumentation.instrumentor import BaseInstrumentor

from openlit.instrumentation.openai_agents.processor import OpenLITTracingProcessor

_instruments = ("openai-agents >= 0.0.3",)


class OpenAIAgentsInstrumentor(BaseInstrumentor):
    """OpenLIT instrumentor for OpenAI Agents using native tracing system"""

    def instrumentation_dependencies(self) -> Collection[str]:
        return _instruments

    def _instrument(self, **kwargs):
        version = importlib.metadata.version("openai-agents")
        environment = kwargs.get("environment", "default")
        application_name = kwargs.get("application_name", "default")
        tracer = kwargs.get("tracer")
        pricing_info = kwargs.get("pricing_info", {})
        capture_message_content = kwargs.get("capture_message_content", False)
        metrics = kwargs.get("metrics_dict")
        disable_metrics = kwargs.get("disable_metrics")
        detailed_tracing = kwargs.get("detailed_tracing", False)

        # Create our processor with OpenLIT enhancements
        processor = OpenLITTracingProcessor(
            tracer=tracer,
            version=version,
            environment=environment,
            application_name=application_name,
            pricing_info=pricing_info,
            capture_message_content=capture_message_content,
            metrics=metrics,
            disable_metrics=disable_metrics,
            detailed_tracing=detailed_tracing,
        )

        # Integrate with OpenAI Agents' native tracing system
        try:
            from agents import set_trace_processors

            # Replace existing processors with our enhanced processor
            set_trace_processors([processor])
        except ImportError:
            # Fallback: Add our processor to existing ones
            try:
                from agents import add_trace_processor

                add_trace_processor(processor)
            except ImportError:
                pass  # Agents package may not have tracing

    def _uninstrument(self, **kwargs):
        # Clear our processors
        try:
            from agents import set_trace_processors

            set_trace_processors([])
        except ImportError:
            pass
