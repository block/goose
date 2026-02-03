# evolving_agents/adapters/openai_tracing_adapter.py

import logging
from typing import Any, Dict, List, Optional

from agents.tracing import add_trace_processor, TracingProcessor

logger = logging.getLogger(__name__)

class EvolvingAgentsTraceProcessor(TracingProcessor):
    """
    Trace processor that integrates OpenAI Agents SDK traces with Evolving Agents monitoring
    """
    
    def process_trace(self, trace: Dict[str, Any]) -> None:
        """
        Process an OpenAI trace and store it in Evolving Agents format
        
        Args:
            trace: The OpenAI trace to process
        """
        try:
            trace_id = trace.get("id", "unknown")
            start_time = trace.get("start_time", 0)
            end_time = trace.get("end_time", 0)
            
            logger.info(f"Processing OpenAI trace {trace_id}")
            
            # Extract and log all spans
            spans = trace.get("spans", [])
            for span in spans:
                span_type = span.get("type", "unknown")
                span_name = span.get("name", "unnamed_span")
                
                if span_type == "tool_call":
                    self._process_tool_call_span(span)
                elif span_type == "handoff":
                    self._process_handoff_span(span)
                elif span_type == "llm_gen":
                    self._process_llm_gen_span(span)
                elif span_type == "guardrail":
                    self._process_guardrail_span(span)
        except Exception as e:
            logger.error(f"Error processing trace: {str(e)}")
    
    def _process_tool_call_span(self, span: Dict[str, Any]) -> None:
        """Process a tool call span"""
        span_name = span.get("name", "unknown_tool_call")
        tool_name = span.get("data", {}).get("name", "unknown_tool")
        tool_args = span.get("data", {}).get("args", {})
        
        # Don't log potentially sensitive data in production
        if "output" in span.get("data", {}):
            tool_result_preview = str(span["data"]["output"])[:50] + "..." if len(str(span["data"]["output"])) > 50 else span["data"]["output"]
        else:
            tool_result_preview = "<no output>"
        
        logger.debug(f"Tool Call: {tool_name}, Args: {tool_args}, Result Preview: {tool_result_preview}")
        
        # Here you would integrate with your existing tracing system
        # For example, storing to a database or emitting metrics
    
    def _process_handoff_span(self, span: Dict[str, Any]) -> None:
        """Process a handoff span"""
        from_agent = span.get("data", {}).get("from_agent", "unknown")
        to_agent = span.get("data", {}).get("to_agent", "unknown")
        
        logger.debug(f"Handoff: {from_agent} → {to_agent}")
        
        # Integrate with your existing tracing system
    
    def _process_llm_gen_span(self, span: Dict[str, Any]) -> None:
        """Process an LLM generation span"""
        model = span.get("data", {}).get("model", "unknown")
        input_tokens = span.get("data", {}).get("input_tokens", 0)
        output_tokens = span.get("data", {}).get("output_tokens", 0)
        
        logger.debug(f"LLM Generation: {model}, Input tokens: {input_tokens}, Output tokens: {output_tokens}")
        
        # Integrate with your existing tracing system
    
    def _process_guardrail_span(self, span: Dict[str, Any]) -> None:
        """Process a guardrail span"""
        guardrail_name = span.get("data", {}).get("name", "unknown_guardrail")
        guardrail_type = span.get("data", {}).get("type", "unknown_type")
        triggered = span.get("data", {}).get("tripwire_triggered", False)
        
        logger.debug(f"Guardrail: {guardrail_name} ({guardrail_type}), Triggered: {triggered}")
        
        # Integrate with your existing tracing system

def register_tracing():
    """Register tracing processors with OpenAI Agents SDK"""
    trace_processor = EvolvingAgentsTraceProcessor()
    add_trace_processor(trace_processor)
    logger.info("Registered OpenAI Agents trace processor")