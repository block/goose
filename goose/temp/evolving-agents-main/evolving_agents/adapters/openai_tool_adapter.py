# evolving_agents/adapters/openai_tool_adapter.py

import logging
import inspect
import json
from typing import Any, Callable, Dict, Optional, List

from agents import function_tool
from agents.run_context import RunContextWrapper

# Import when available, otherwise handle gracefully
try:
    from beeai_framework.tools.tool import Tool, StringToolOutput
    from evolving_agents.tools.tool_factory import DynamicTool
    BEEAI_AVAILABLE = True
except ImportError:
    BEEAI_AVAILABLE = False
    DynamicTool = type('DynamicTool', (), {})  # dummy class

logger = logging.getLogger(__name__)

class OpenAIToolAdapter:
    """Adapter for converting between Evolving Agents tools and OpenAI Agents SDK tools"""
    
    @staticmethod
    def convert_evolving_tool_to_openai(tool: Any) -> Any:
        """
        Convert an Evolving Agents tool to an OpenAI Agents SDK tool
        
        Args:
            tool: The Evolving Agents tool to convert
            
        Returns:
            OpenAI Agents SDK compatible tool
        """
        # If it's already an OpenAI tool, return as is
        if (hasattr(tool, "name") and hasattr(tool, "on_invoke_tool") and callable(tool.on_invoke_tool)):
            return tool
        
        # If BeeAI is available, handle BeeAI-specific tool types
        if BEEAI_AVAILABLE:
            # If it's a DynamicTool, create a function_tool that executes its code
            if isinstance(tool, DynamicTool):
                @function_tool(
                    name_override=tool.name,
                    description_override=tool.description
                )
                async def dynamic_tool_wrapper(ctx: RunContextWrapper, input_text: str) -> str:
                    # Use the DynamicTool's _run method
                    result = await tool._run({"text": input_text}, None)
                    
                    # Convert result to string
                    if hasattr(result, "get_text_content"):
                        return result.get_text_content()
                    return str(result)
                
                return dynamic_tool_wrapper
                
            # For BeeAI tools, create a wrapper that calls the tool's run method
            elif hasattr(tool, "run") and inspect.iscoroutinefunction(tool.run):
                @function_tool(
                    name_override=getattr(tool, "name", "unknown_tool"),
                    description_override=getattr(tool, "description", "")
                )
                async def beeai_tool_wrapper(ctx: RunContextWrapper, input_text: str) -> str:
                    # Support different input patterns for BeeAI tools
                    if hasattr(tool, "input_schema"):
                        try:
                            # Try to parse the input as JSON for schema-based tools
                            input_data = json.loads(input_text) if input_text else {}
                            
                            # Create an instance of the input schema class
                            input_obj = tool.input_schema(**input_data)
                            
                            # Call with proper input object
                            result = await tool.run(input_obj, None)
                        except json.JSONDecodeError:
                            # Fall back to text input if not valid JSON
                            result = await tool.run({"text": input_text}, None)
                    else:
                        # Simple text input
                        result = await tool.run({"text": input_text}, None)
                    
                    # Convert result to string
                    if hasattr(result, "get_text_content"):
                        return result.get_text_content()
                    return str(result)
                
                return beeai_tool_wrapper
        
        # Generic handling for tools with a callable interface
        if hasattr(tool, "__call__") and callable(tool.__call__):
            @function_tool(
                name_override=getattr(tool, "name", "callable_tool"),
                description_override=getattr(tool, "description", "A callable tool")
            )
            async def callable_tool_wrapper(ctx: RunContextWrapper, input_text: str) -> str:
                if inspect.iscoroutinefunction(tool.__call__):
                    result = await tool(input_text)
                else:
                    result = tool(input_text)
                return str(result)
            
            return callable_tool_wrapper
        
        # Default fallback for unknown tool types
        @function_tool(
            name_override=getattr(tool, "name", "unknown_tool"),
            description_override=getattr(tool, "description", "Unknown tool")
        )
        async def unknown_tool_wrapper(ctx: RunContextWrapper, input_text: str) -> str:
            logger.warning(f"Using unknown tool type: {type(tool)}")
            return f"Unable to execute tool of type {type(tool)}"
        
        return unknown_tool_wrapper
    
    @staticmethod
    def batch_convert_tools(tools: List[Any]) -> List[Any]:
        """
        Convert a list of Evolving Agents tools to OpenAI Agents SDK tools
        
        Args:
            tools: List of Evolving Agents tools
            
        Returns:
            List of OpenAI Agents SDK compatible tools
        """
        return [OpenAIToolAdapter.convert_evolving_tool_to_openai(tool) for tool in tools]
    
    @staticmethod
    def extract_schema_from_openai_tool(openai_tool: Any) -> Dict[str, Any]:
        """
        Extract the schema from an OpenAI function_tool
        
        Args:
            openai_tool: The OpenAI tool
            
        Returns:
            JSON schema dictionary
        """
        if hasattr(openai_tool, "params_json_schema"):
            return openai_tool.params_json_schema
        
        # Default minimal schema
        return {
            "type": "object",
            "properties": {
                "text": {
                    "type": "string",
                    "description": "Input text to process"
                }
            },
            "required": ["text"]
        }