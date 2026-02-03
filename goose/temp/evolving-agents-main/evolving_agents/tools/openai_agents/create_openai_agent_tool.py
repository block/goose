# evolving_agents/tools/openai_agents/create_openai_agent_tool.py

from typing import Dict, Any, List, Optional, Union
from pydantic import BaseModel, Field
import json

from beeai_framework.tools.tool import Tool, StringToolOutput
from beeai_framework.context import RunContext
from beeai_framework.emitter.emitter import Emitter

from evolving_agents.smart_library.smart_library import SmartLibrary
from evolving_agents.core.llm_service import LLMService
from evolving_agents.firmware.firmware import Firmware

class CreateOpenAIAgentInput(BaseModel):
    """Input schema for the CreateOpenAIAgentTool."""
    name: str = Field(description="Name of the agent to create")
    domain: str = Field(description="Domain for the agent")
    description: str = Field(description="Description of the agent's purpose and capabilities")
    required_tools: Optional[List[str]] = Field(None, description="List of tool names this agent should use")
    model: str = Field("gpt-4o", description="OpenAI model to use")
    temperature: float = Field(0.7, description="LLM temperature (0.0 to 1.0)")
    guardrails_enabled: bool = Field(True, description="Whether to enable firmware guardrails")
    tags: Optional[List[str]] = Field(None, description="Optional tags for the agent")
    metadata: Optional[Dict[str, Any]] = Field(None, description="Additional metadata")

class CreateOpenAIAgentTool(Tool[CreateOpenAIAgentInput, None, StringToolOutput]):
    """
    Tool for creating OpenAI agents in the Smart Library.
    This tool creates agent records specifically designed to work with OpenAI's Agents SDK.
    """
    name = "CreateOpenAIAgentTool"
    description = "Create OpenAI agents with the OpenAI Agents SDK, configuring model, tools, and governance"
    input_schema = CreateOpenAIAgentInput
    
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
            namespace=["tool", "openai", "create_agent"],
            creator=self,
        )
    
    async def _run(self, input: CreateOpenAIAgentInput, options: Optional[Dict[str, Any]] = None, context: Optional[RunContext] = None) -> StringToolOutput:
        """
        Create a new OpenAI agent in the Smart Library.
        
        Args:
            input: The agent creation parameters
        
        Returns:
            StringToolOutput containing the creation result in JSON format
        """
        try:
            # Create code snippet for the agent
            code_snippet = f"""
from agents import Agent, Runner
from agents.model_settings import ModelSettings

# Create an OpenAI Agent
agent = Agent(
    name="{input.name}",
    instructions=\"\"\"
{input.description}
\"\"\",
    model="{input.model}",
    model_settings=ModelSettings(
        temperature={input.temperature}
    )
)

# Example usage with async
async def run_agent(input_text):
    result = await Runner.run(agent, input_text)
    return result.final_output

# Example usage with sync
def run_agent_sync(input_text):
    result = Runner.run_sync(agent, input_text)
    return result.final_output
"""
            
            # Create metadata with OpenAI-specific settings
            base_metadata = input.metadata or {}
            metadata = {
                **base_metadata,
                "framework": "openai-agents",
                "model": input.model,
                "model_settings": {
                    "temperature": input.temperature
                },
                "guardrails_enabled": input.guardrails_enabled
            }
            
            # Add tools reference if specified
            if input.required_tools:
                metadata["required_tools"] = input.required_tools
            
            # Create the record
            record = await self.library.create_record(
                name=input.name,
                record_type="AGENT",
                domain=input.domain,
                description=input.description,
                code_snippet=code_snippet,
                tags=input.tags or ["openai", input.domain, "agent"],
                metadata=metadata
            )
            
            # Return the success response
            return StringToolOutput(json.dumps({
                "status": "success",
                "message": f"Created new OpenAI agent '{input.name}'",
                "record_id": record["id"],
                "record": {
                    "name": record["name"],
                    "type": record["record_type"],
                    "domain": record["domain"],
                    "description": record["description"],
                    "version": record["version"],
                    "created_at": record["created_at"],
                    "framework": "openai-agents",
                    "model": input.model
                },
                "next_steps": [
                    "Use the OpenAI agent with appropriate tools",
                    "Execute the agent with the OpenAIAgentExecutionTool",
                    "Consider adding guardrails with the AddOpenAIGuardrailsTool"
                ]
            }, indent=2))
            
        except Exception as e:
            import traceback
            return StringToolOutput(json.dumps({
                "status": "error",
                "message": f"Error creating OpenAI agent: {str(e)}",
                "details": traceback.format_exc()
            }, indent=2))