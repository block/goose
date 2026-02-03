# evolving_agents/tools/smart_library/create_component_tool.py

from typing import Dict, Any, List, Optional, Union
from pydantic import BaseModel, Field
import json # Keep for loading, but use safe_json_dumps for output
from datetime import datetime, timezone # For timestamp

from beeai_framework.tools.tool import Tool, StringToolOutput
from beeai_framework.context import RunContext
from beeai_framework.emitter.emitter import Emitter

from evolving_agents.smart_library.smart_library import SmartLibrary
from evolving_agents.core.llm_service import LLMService
from evolving_agents.firmware.firmware import Firmware
# Import the corrected safe_json_dumps
from evolving_agents.workflow.process_workflow_tool import safe_json_dumps


class CreateComponentInput(BaseModel):
    """Input schema for the CreateComponentTool."""
    name: str = Field(description="Name of the component to create")
    record_type: str = Field(description="Type of record to create (AGENT or TOOL)")
    domain: str = Field(description="Domain for the component")
    description: str = Field(description="Description of the component")
    requirements: Optional[str] = Field(None, description="Natural language requirements for code generation")
    code_snippet: Optional[str] = Field(None, description="Code snippet if provided directly")
    tags: Optional[List[str]] = Field(None, description="Tags for the component")
    metadata: Optional[Dict[str, Any]] = Field(None, description="Additional metadata")
    framework: Optional[str] = Field("beeai", description="Framework to use for code generation (default: beeai)")

class CreateComponentTool(Tool[CreateComponentInput, None, StringToolOutput]):
    """
    Tool for creating new components (agents or tools) in the Smart Library.
    This tool can generate code based on natural language requirements using templates
    appropriate for the component type and framework. It handles all aspects of
    component creation including code generation, metadata, and registration.
    """
    name = "CreateComponentTool"
    description = "Create new agents and tools from requirements or specifications, with automatic code generation"
    input_schema = CreateComponentInput
    
    def __init__(
        self, 
        smart_library: SmartLibrary, 
        llm_service: LLMService,
        firmware: Optional[Firmware] = None,
        options: Optional[Dict[str, Any]] = None
    ):
        super().__init__(options=options or {})
        self.library = smart_library
        self.llm = llm_service
        self.firmware = firmware or Firmware()
    
    def _create_emitter(self) -> Emitter:
        return Emitter.root().child(
            namespace=["tool", "library", "create"],
            creator=self,
        )
    
    async def _run(self, input: CreateComponentInput, options: Optional[Dict[str, Any]] = None, context: Optional[RunContext] = None) -> StringToolOutput:
        """
        Create a new component in the Smart Library.
        
        Args:
            input: The component creation parameters
        
        Returns:
            StringToolOutput containing the creation result in JSON format
        """
        try:
            # If code snippet is not provided but requirements are, generate code
            if not input.code_snippet and input.requirements:
                # Get firmware for this domain
                firmware_content = self.firmware.get_firmware_prompt(input.domain)
                
                # Generate code based on requirements
                code_snippet = await self._generate_code_from_requirements(
                    input.record_type,
                    input.domain,
                    input.name,
                    input.description,
                    input.requirements,
                    firmware_content,
                    input.framework
                )
            else:
                code_snippet = input.code_snippet or "# No code provided"
            
            # Create the record
            metadata = input.metadata or {}
            
            # Add framework metadata if not already specified
            if "framework" not in metadata:
                metadata["framework"] = input.framework
            
            # Extract required tools from code for agents
            if input.record_type == "AGENT":
                required_tools = self._extract_required_tools(code_snippet)
                if required_tools:
                    metadata["required_tools"] = required_tools
            
            # Add creation strategy to metadata
            metadata["creation_strategy"] = {
                "method": "requirements" if input.requirements else "direct_code",
                "timestamp": datetime.now(timezone.utc).isoformat(), # Use consistent timestamp
                "requirements_summary": self._summarize_text(input.requirements) if input.requirements else None
            }
            
            # Create the record in the library
            record = await self.library.create_record(
                name=input.name,
                record_type=input.record_type,
                domain=input.domain,
                description=input.description,
                code_snippet=code_snippet,
                tags=input.tags or [input.domain, input.record_type.lower()],
                metadata=metadata
            )
            
            # Return the success response
            # Ensure record["created_at"] is an ISO string if it's a datetime object for JSON dump
            created_at_iso = record["created_at"]
            if isinstance(created_at_iso, datetime):
                created_at_iso = created_at_iso.isoformat()

            response_data = {
                "status": "success",
                "message": f"Created new {input.record_type} '{input.name}'",
                "record_id": record["id"],
                "record": {
                    "name": record["name"],
                    "type": record["record_type"],
                    "domain": record["domain"],
                    "description": record["description"],
                    "version": record["version"],
                    "created_at": created_at_iso, # Use ISO string
                    "code_size": len(code_snippet),
                    "metadata": metadata
                },
                "next_steps": [
                    "Register this component with the Agent Bus to make it available to other agents",
                    "Test the component with sample inputs to verify functionality",
                    f"Consider evolving this component if it doesn't fully meet requirements"
                ]
            }
            return StringToolOutput(safe_json_dumps(response_data)) # Use safe_json_dumps
            
        except Exception as e:
            import traceback
            return StringToolOutput(safe_json_dumps({ # Use safe_json_dumps
                "status": "error",
                "message": f"Error creating component: {str(e)}",
                "details": traceback.format_exc()
            }))
    
    def _extract_required_tools(self, code_snippet: str) -> List[str]:
        """Extract required tools from agent code."""
        import re
        
        # Look for tool initialization patterns
        tools_pattern = r"tools=\[(.*?)\]"
        tools_match = re.search(tools_pattern, code_snippet, re.DOTALL)
        
        if not tools_match:
            return []
        
        tools_str = tools_match.group(1)
        
        # Extract tool class names - covers patterns like ToolName() or instances of tools
        tool_classes = re.findall(r'(\w+)\(\)', tools_str)
        
        return tool_classes
    
    def _get_current_timestamp(self) -> str:
        """Get current timestamp in ISO format."""
        return datetime.now(timezone.utc).isoformat()
    
    def _summarize_text(self, text: str, max_length: int = 100) -> str:
        """Create a short summary of text."""
        if not text:
            return ""
        
        # Simple truncation for now
        if len(text) <= max_length:
            return text
        
        return text[:max_length-3] + "..."
    
    async def _generate_code_from_requirements(
        self,
        record_type: str,
        domain: str,
        name: str,
        description: str,
        requirements: str,
        firmware_content: str,
        framework: str = "beeai"
    ) -> str:
        """
        Generate code based on natural language requirements.
        
        This method contains the logic for creating component code from requirements,
        using the appropriate templates and frameworks.
        
        Args:
            record_type: Type of component (AGENT or TOOL)
            domain: Domain for the component
            name: Name of the component
            description: Description of the component
            requirements: Natural language requirements
            firmware_content: Firmware content to inject
            framework: Framework to use (default: beeai)
            
        Returns:
            Generated code snippet
        """
        # Define templates directory
        import os
        templates_dir = os.path.join(os.path.dirname(os.path.dirname(os.path.dirname(__file__))), "templates")
        
        # Make sure the templates directory exists
        os.makedirs(templates_dir, exist_ok=True)
        
        # Get template paths based on framework
        if framework.lower() == "beeai":
            agent_template_path = os.path.join(templates_dir, "beeai_agent_template.txt")
            tool_template_path = os.path.join(templates_dir, "beeai_tool_template.txt")
        else:
            # Default to BeeAI templates for now
            agent_template_path = os.path.join(templates_dir, "beeai_agent_template.txt")
            tool_template_path = os.path.join(templates_dir, "beeai_tool_template.txt")
        
        # Read templates from files or use default if files don't exist
        try:
            with open(agent_template_path, 'r') as f:
                agent_template = f.read()
        except FileNotFoundError:
            agent_template = "# Default agent template would be here"
            
        try:
            with open(tool_template_path, 'r') as f:
                tool_template = f.read()
        except FileNotFoundError:
            tool_template = "# Default tool template would be here"
        
        # Build the appropriate prompt based on component type and framework
        if record_type == "AGENT":
            if framework.lower() == "beeai":
                creation_prompt = f"""
                {firmware_content}

                Create a Python agent using the BeeAI framework that fulfills these requirements:
                "{requirements}"

                AGENT NAME: {name}
                DOMAIN: {domain}
                DESCRIPTION: {description}

                IMPORTANT REQUIREMENTS:
                1. The agent must be a properly implemented BeeAI ReActAgent 
                2. Use the following framework imports:
                - from beeai_framework.agents.react import ReActAgent
                - from beeai_framework.agents.types import AgentMeta
                - from beeai_framework.memory import TokenMemory or UnconstrainedMemory

                3. The agent must follow this structure - implementing a class with a create_agent method:

                REFERENCE TEMPLATE FOR A BEEAI AGENT:
                ```python
    {agent_template}
                ```

                YOUR TASK:
                Create a similar agent class for: "{requirements}"
                - Replace the WeatherAgentInitializer with {name}Initializer
                - Adapt the description and functionality for the {domain} domain
                - Include all required disclaimers from the firmware
                - Specify any tools the agent should use
                - The code must be complete and executable

                CODE:
                """
            else:
                # Generic agent template for other frameworks
                creation_prompt = f"""
                {firmware_content}

                Create a Python agent that fulfills these requirements:
                "{requirements}"

                AGENT NAME: {name}
                DOMAIN: {domain}
                DESCRIPTION: {description}

                The agent should be properly implemented with:
                - Clear class and method structure
                - Appropriate error handling
                - Domain-specific functionality for {domain}
                - All required disclaimers from the firmware

                CODE:
                """
        else:  # TOOL
            if framework.lower() == "beeai":
                creation_prompt = f"""
                {firmware_content}

                Create a Python tool using the BeeAI framework that fulfills these requirements:
                "{requirements}"

                TOOL NAME: {name}
                DOMAIN: {domain}
                DESCRIPTION: {description}

                IMPORTANT REQUIREMENTS:
                1. The tool must be a properly implemented BeeAI Tool class
                2. Use the following framework imports:
                - from beeai_framework.tools.tool import Tool, StringToolOutput
                - from beeai_framework.context import RunContext
                - from pydantic import BaseModel, Field

                3. The tool must follow this structure with an input schema and _run method:

                REFERENCE TEMPLATE FOR A BEEAI TOOL:
                ```python
    {tool_template}
                ```

                YOUR TASK:
                Create a similar tool class for: "{requirements}"
                - Use {name} as the class name
                - Create an appropriate input schema class named {name}Input
                - Define proper input fields with descriptions
                - Implement the _run method with appropriate logic
                - Include error handling
                - For domain '{domain}', include all required disclaimers
                - The code must be complete and executable

                CODE:
                """
            else:
                # Generic tool template for other frameworks
                creation_prompt = f"""
                {firmware_content}

                Create a Python tool that fulfills these requirements:
                "{requirements}"

                TOOL NAME: {name}
                DOMAIN: {domain}
                DESCRIPTION: {description}

                The tool should be properly implemented with:
                - Clear input parameters
                - Appropriate error handling
                - Domain-specific functionality for {domain}
                - All required disclaimers from the firmware

                CODE:
                """

        # Generate code using LLM
        return await self.llm.generate(creation_prompt)