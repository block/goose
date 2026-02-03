# evolving_agents/tools/openai_agents/convert_tool_format_tool.py

from typing import Dict, Any, List, Optional, Union
from pydantic import BaseModel, Field
import json
import inspect

from beeai_framework.tools.tool import Tool, StringToolOutput
from beeai_framework.context import RunContext
from beeai_framework.emitter.emitter import Emitter

from evolving_agents.smart_library.smart_library import SmartLibrary
from evolving_agents.adapters.openai_tool_adapter import OpenAIToolAdapter

class ConvertToolFormatInput(BaseModel):
    """Input schema for the ConvertToolFormatTool."""
    tool_id_or_name: str = Field(description="ID or name of the tool to convert")
    target_format: str = Field(description="Target format ('openai' or 'evolving')")
    save_to_library: bool = Field(False, description="Whether to save the converted tool to the library")
    new_name: Optional[str] = Field(None, description="New name for the converted tool if saving to library")

class ConvertToolFormatTool(Tool[ConvertToolFormatInput, None, StringToolOutput]):
    """
    Tool for converting between Evolving Agents tools and OpenAI Agents SDK tools.
    This tool can convert in both directions and optionally save the converted tool to the library.
    """
    name = "ConvertToolFormatTool"
    description = "Convert tools between Evolving Agents format and OpenAI Agents SDK format"
    input_schema = ConvertToolFormatInput
    
    def __init__(
        self, 
        smart_library: SmartLibrary, 
        tool_factory = None,
        options: Optional[Dict[str, Any]] = None
    ):
        super().__init__(options=options or {})
        self.library = smart_library
        self.tool_factory = tool_factory
    
    def _create_emitter(self) -> Emitter:
        return Emitter.root().child(
            namespace=["tool", "openai", "convert_tool"],
            creator=self,
        )
    
    async def _run(self, input: ConvertToolFormatInput, options: Optional[Dict[str, Any]] = None, context: Optional[RunContext] = None) -> StringToolOutput:
        """
        Convert a tool between formats.
        
        Args:
            input: The conversion parameters
        
        Returns:
            StringToolOutput containing the conversion result
        """
        try:
            # 1. Get the tool record
            tool_record = await self._get_tool_record(input.tool_id_or_name)
            if not tool_record:
                return StringToolOutput(json.dumps({
                    "status": "error",
                    "message": f"Tool '{input.tool_id_or_name}' not found"
                }, indent=2))
            
            # 2. Create the tool instance
            if not self.tool_factory:
                return StringToolOutput(json.dumps({
                    "status": "error",
                    "message": "Tool factory not available for conversion"
                }, indent=2))
                
            original_tool = await self.tool_factory.create_tool(tool_record)
            
            # 3. Convert the tool
            if input.target_format.lower() == "openai":
                # Convert Evolving Agents tool to OpenAI tool
                converted_tool = OpenAIToolAdapter.convert_evolving_tool_to_openai(original_tool)
                conversion_direction = "Evolving Agents → OpenAI"
                
                # Generate code for the converted tool
                converted_code = self._generate_openai_tool_code(
                    tool_record["name"],
                    tool_record["description"],
                    converted_tool
                )
                
            else:
                # This direction is more complex and would require implementation
                # in OpenAIToolAdapter to convert from OpenAI format to Evolving Agents format
                return StringToolOutput(json.dumps({
                    "status": "error",
                    "message": "Conversion from OpenAI to Evolving Agents format is not yet implemented"
                }, indent=2))
            
            # 4. Save to library if requested
            saved_record = None
            if input.save_to_library:
                new_name = input.new_name or f"{tool_record['name']}_openai"
                
                # Create a new record
                saved_record = await self.library.create_record(
                    name=new_name,
                    record_type="TOOL",
                    domain=tool_record.get("domain", "general"),
                    description=f"OpenAI version of {tool_record['name']}: {tool_record['description']}",
                    code_snippet=converted_code,
                    tags=tool_record.get("tags", []) + ["openai", "converted"],
                    metadata={
                        "framework": "openai-agents",
                        "converted_from": tool_record["id"],
                        "original_name": tool_record["name"],
                        "conversion_direction": conversion_direction
                    }
                )
            
            # 5. Return the result
            result = {
                "status": "success",
                "original_tool": tool_record["name"],
                "conversion_direction": conversion_direction,
                "converted_code": converted_code,
            }
            
            if saved_record:
                result["saved"] = {
                    "id": saved_record["id"],
                    "name": saved_record["name"],
                    "message": f"Converted tool saved to library as '{saved_record['name']}'"
                }
            else:
                result["saved"] = False
                
            return StringToolOutput(json.dumps(result, indent=2))
            
        except Exception as e:
            import traceback
            return StringToolOutput(json.dumps({
                "status": "error",
                "message": f"Error converting tool: {str(e)}",
                "details": traceback.format_exc()
            }, indent=2))
    
    async def _get_tool_record(self, tool_id_or_name: str) -> Optional[Dict[str, Any]]:
        """Get a tool record by ID or name."""
        # Try by ID first
        record = await self.library.find_record_by_id(tool_id_or_name)
        if record:
            return record
        
        # Try by name for TOOL type
        return await self.library.find_record_by_name(tool_id_or_name, "TOOL")
    
    def _generate_openai_tool_code(self, name: str, description: str, openai_tool: Any) -> str:
        """Generate OpenAI tool code."""
        if hasattr(openai_tool, "name") and hasattr(openai_tool, "description"):
            # It's already an OpenAI function_tool
            return f"""
from agents import function_tool
from agents.run_context import RunContextWrapper

@function_tool(
    name_override="{openai_tool.name}",
    description_override="{openai_tool.description}"
)
async def {name.lower()}_tool(context: RunContextWrapper, input_text: str) -> str:
    \"\"\"
    {description}
    
    Args:
        context: The run context
        input_text: Input text to process
        
    Returns:
        Processing result
    \"\"\"
    # Implementation would call the original tool
    return f"Processed: {{input_text}}"
"""
        else:
            # Generic conversion code
            return f"""
from agents import function_tool
from agents.run_context import RunContextWrapper

@function_tool
async def {name.lower()}_tool(context: RunContextWrapper, input_text: str) -> str:
    \"\"\"
    {description}
    
    Args:
        context: The run context
        input_text: Input text to process
        
    Returns:
        Processing result
    \"\"\"
    # Implementation would call the original tool
    return f"Processed: {{input_text}}"
"""