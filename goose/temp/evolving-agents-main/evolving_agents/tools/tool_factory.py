# evolving_agents/tools/tool_factory.py (improved version)

import logging
import traceback
import re
import importlib.util
import sys
import tempfile
import os
from typing import Dict, Any, Optional, Type
from pydantic import BaseModel, Field

from beeai_framework.tools.tool import Tool, StringToolOutput
from beeai_framework.emitter.emitter import Emitter
from beeai_framework.utils.strings import to_safe_word
from beeai_framework.context import RunContext, RunContextInput, RunInstance

from evolving_agents.smart_library.smart_library import SmartLibrary
from evolving_agents.core.llm_service import LLMService

logger = logging.getLogger(__name__)

class ToolFactory:
    """
    Factory for creating and executing tools from library records.
    """
    def __init__(self, smart_library: SmartLibrary, llm_service: LLMService):
        self.library = smart_library
        self.llm = llm_service
        self.active_tools = {}
    
    async def create_tool(
        self, 
        record: Dict[str, Any],
        firmware_content: Optional[str] = None
    ) -> Tool:
        """
        Create a tool instance from a library record.
        
        Args:
            record: Tool record from the Smart Library
            firmware_content: Optional firmware content to inject
            
        Returns:
            Instantiated tool
        """
        logger.info(f"Creating tool {record['name']} from record {record['id']}")
        
        try:
            # First, try to create a real BeeAI tool by importing from the code
            tool = await self._create_real_beeai_tool(record)
            
            if tool:
                # Store in active tools
                self.active_tools[record["name"]] = {
                    "record": record,
                    "instance": tool,
                    "type": "TOOL"
                }
                return tool
            
            # Fall back to DynamicTool if needed
            tool = self._create_dynamic_tool(record)
            
            # Store in active tools
            self.active_tools[record["name"]] = {
                "record": record,
                "instance": tool,
                "type": "TOOL"
            }
            
            return tool
            
        except Exception as e:
            logger.error(f"Error creating tool {record['name']}: {str(e)}")
            logger.error(traceback.format_exc())
            
            # Create a fallback tool that returns the error
            fallback_code = f"""
            # Error occurred during tool creation
            
            def fallback_function(input):
                return "Error creating tool: {str(e)}"
                
            result = fallback_function(input)
            """
            
            fallback_tool = self._create_dynamic_tool(
                record,
                override_code=fallback_code
            )
            
            # Store fallback in active tools
            self.active_tools[record["name"]] = {
                "record": record,
                "instance": fallback_tool,
                "type": "TOOL"
            }
            
            return fallback_tool
    
    async def _create_real_beeai_tool(self, record: Dict[str, Any]) -> Optional[Tool]:
        """Attempts to create a real BeeAI tool from the code snippet."""
        code_snippet = record["code_snippet"]
        
        # Try to find a class that extends Tool
        class_match = re.search(r"class\s+(\w+)\(Tool\[", code_snippet)
        if not class_match:
            return None
        
        tool_class_name = class_match.group(1)
        
        # Create a temporary module to load the code
        try:
            # Create a unique module name
            module_name = f"dynamic_tool_{record['id'].replace('-', '_')}"
            
            # Write the code to a temporary file
            with tempfile.NamedTemporaryFile(suffix='.py', delete=False, mode='w') as f:
                f.write(code_snippet)
                temp_file = f.name
            
            try:
                # Create a module spec
                spec = importlib.util.spec_from_file_location(module_name, temp_file)
                if not spec or not spec.loader:
                    return None
                
                # Import the module
                module = importlib.util.module_from_spec(spec)
                sys.modules[module_name] = module
                spec.loader.exec_module(module)
                
                # Get the tool class
                if hasattr(module, tool_class_name):
                    tool_class = getattr(module, tool_class_name)
                    
                    # Check if it's a proper Tool subclass
                    if issubclass(tool_class, Tool):
                        # Create an instance of the tool
                        tool_instance = tool_class()
                        return tool_instance
            finally:
                # Clean up the temporary file
                os.unlink(temp_file)
                
                # Remove the module from sys.modules
                if module_name in sys.modules:
                    del sys.modules[module_name]
        
        except Exception as e:
            logger.error(f"Error creating real BeeAI tool: {str(e)}")
            logger.error(traceback.format_exc())
            
        return None
    
    def _create_dynamic_tool(self, record: Dict[str, Any], override_code: Optional[str] = None) -> Tool:
        """Fallback to create a DynamicTool when direct import fails."""
        code_snippet = override_code or record["code_snippet"]
        
        # Process the code snippet to ensure it's executable
        processed_code = self._process_code_snippet(code_snippet)
        
        # Create the dynamic tool
        tool = DynamicTool(
            name=record["name"],
            description=record["description"],
            code_snippet=processed_code
        )
        
        return tool
    
    def _process_code_snippet(self, code_snippet: str) -> str:
        """Process the code snippet to ensure it's executable."""
        # Log the original code for debugging
        logger.debug(f"Original code snippet:\n{code_snippet}")
        
        # Remove markdown code blocks
        # First, try to extract Python code between ```python and ``` tags
        python_blocks = re.findall(r"```python(.*?)```", code_snippet, re.DOTALL)
        if python_blocks:
            # Use the first Python code block found
            code_snippet = python_blocks[0]
        else:
            # Remove any generic code blocks
            code_blocks = re.findall(r"```(.*?)```", code_snippet, re.DOTALL)
            if code_blocks:
                # Use the first code block found
                code_snippet = code_blocks[0]
                
        # Clean up the code - remove leading/trailing whitespace
        code_snippet = code_snippet.strip()
        
        # Ensure the code has proper input handling
        if "input" not in code_snippet:
            logger.warning(f"Code snippet doesn't reference input parameter. Adding fallback input handling.")
            code_snippet = "# Input is available as 'input' variable\n" + code_snippet
        
        # Ensure the code sets a 'result' variable
        if "result =" not in code_snippet and not code_snippet.strip().endswith("result = "):
            logger.warning(f"Code snippet doesn't set 'result' variable. Adding fallback result assignment.")
            code_snippet += "\n\n# Ensure result is set\nif 'result' not in locals():\n    result = 'Tool executed but did not set a result'"
        
        # Log the processed code
        logger.debug(f"Processed code snippet:\n{code_snippet}")
        
        return code_snippet
    
    async def execute_tool(self, tool_instance: Tool, input_text: str) -> str:
        """
        Execute a tool with input text.
        
        Args:
            tool_instance: The tool instance
            input_text: Input text
            
        Returns:
            Tool output as string
        """
        logger.info(f"Executing tool with input: {input_text[:50]}...")
        
        try:
            if isinstance(tool_instance, DynamicTool):
                # Execute dynamic tool directly
                input_obj = tool_instance.input_schema(text=input_text)
                result = tool_instance._run(input=input_obj, options=None)
                
                # Convert result to string
                if hasattr(result, "get_text_content"):
                    return result.get_text_content()
                elif hasattr(result, "result"):
                    return result.result
                else:
                    return str(result)
            else:
                # Real BeeAI tool - use its run method
                # Create a RunContext for the tool execution
                emitter = Emitter.root().child(
                    namespace=["tool", "execution"],
                    creator=self,
                )
                
                run_instance = RunInstance(emitter=emitter)
                run_context_input = RunContextInput(params=[input_text])
                
                # Run the tool with a run context
                result = await tool_instance.run(
                    {"text": input_text},  # Pass as dict to match the usual input schema
                    None  # No specific options
                )
                
                # Extract the result from the tool output
                return result.get_text_content()
                
        except Exception as e:
            logger.error(f"Error executing tool: {str(e)}")
            logger.error(traceback.format_exc())
            return f"Execution error: {str(e)}"


class DynamicTool(Tool):
    """A dynamic tool implementation compatible with beeai-framework."""
    
    class DynamicToolInput(BaseModel):
        text: str = Field(description="Input text to process")
    
    def __init__(self, name: str, description: str, code_snippet: str, options: Optional[Dict[str, Any]] = None):
        # Initialize with options
        super().__init__(options=options or {})
        
        # Store tool attributes
        self._name_value = name
        self._description_value = description
        
        # Process the code snippet to ensure it's executable
        self.code_snippet = code_snippet
        
        # Initialize emitter
        self.emitter = Emitter.root().child(
            namespace=["tool", "dynamic", to_safe_word(self._name_value)],
            creator=self,
        )
    
    @property
    def name(self) -> str:
        return self._name_value
        
    @property
    def description(self) -> str:
        return self._description_value
    
    @property
    def input_schema(self) -> Type[BaseModel]:
        return self.DynamicToolInput
    
    def _create_emitter(self) -> Emitter:
        return self.emitter
    
    async def _run(self, input: Any, options: Optional[Dict[str, Any]] = None, context: Optional[RunContext] = None) -> Any:
        """Run the tool with the provided input."""
        try:
            # Create a local scope with the input
            local_scope = {"input": input.text, "result": None}
            
            # Log the code being executed
            logger.debug(f"Executing code snippet for {self.name}:\n{self.code_snippet}")
            
            # Execute the code with the local scope
            exec(self.code_snippet, {"print": print}, local_scope)
            
            # Return the result as a StringToolOutput
            if "result" in local_scope and local_scope["result"] is not None:
                result_value = local_scope["result"]
                if isinstance(result_value, dict) or isinstance(result_value, list):
                    import json
                    return StringToolOutput(json.dumps(result_value, indent=2))
                return StringToolOutput(str(result_value))
            else:
                return StringToolOutput("Tool executed but did not produce a result")
        except Exception as e:
            error_msg = f"Error executing tool {self.name}: {str(e)}"
            logger.error(error_msg)
            logger.error(traceback.format_exc())
            return StringToolOutput(f"Execution error: {str(e)}")