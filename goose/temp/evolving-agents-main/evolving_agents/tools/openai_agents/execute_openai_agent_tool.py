# evolving_agents/tools/openai_agents/execute_openai_agent_tool.py

from typing import Dict, Any, List, Optional, Union, Tuple
from pydantic import BaseModel, Field
import json
import asyncio
import logging

from beeai_framework.tools.tool import Tool, StringToolOutput
from beeai_framework.context import RunContext
from beeai_framework.emitter.emitter import Emitter

from evolving_agents.smart_library.smart_library import SmartLibrary
from evolving_agents.providers.openai_agents_provider import OpenAIAgentsProvider
from evolving_agents.adapters.openai_tool_adapter import OpenAIToolAdapter

# Import OpenAI Agents SDK
from agents import Agent as OpenAIAgent, Runner
from agents.run_context import RunContextWrapper

logger = logging.getLogger(__name__)

class ExecuteOpenAIAgentInput(BaseModel):
    """Input schema for the ExecuteOpenAIAgentTool."""
    agent_id_or_name: str = Field(description="ID or name of the agent to execute")
    input_text: str = Field(description="Input text to send to the agent")
    tools_to_use: Optional[List[str]] = Field(None, description="Optional list of tool names to provide to the agent")
    max_turns: int = Field(10, description="Maximum number of agent turns")
    apply_firmware: bool = Field(True, description="Whether to apply firmware guardrails")

class ExecuteOpenAIAgentTool(Tool[ExecuteOpenAIAgentInput, None, StringToolOutput]):
    """
    Tool for executing OpenAI agents from the Smart Library.
    This tool loads and runs agents created with the OpenAI Agents SDK.
    """
    name = "ExecuteOpenAIAgentTool"
    description = "Execute OpenAI agents stored in the Smart Library with input text and optional tools"
    input_schema = ExecuteOpenAIAgentInput
    
    def __init__(
        self, 
        smart_library: SmartLibrary, 
        openai_provider: Optional[OpenAIAgentsProvider] = None,
        tool_factory = None,  # We'll use this to create tools
        options: Optional[Dict[str, Any]] = None
    ):
        super().__init__(options=options or {})
        self.library = smart_library
        self.provider = openai_provider or OpenAIAgentsProvider()
        self.tool_factory = tool_factory
        self.active_agents = {}
    
    def _create_emitter(self) -> Emitter:
        return Emitter.root().child(
            namespace=["tool", "openai", "execute_agent"],
            creator=self,
        )
    
    async def _run(self, input: ExecuteOpenAIAgentInput, options: Optional[Dict[str, Any]] = None, context: Optional[RunContext] = None) -> StringToolOutput:
        """
        Execute an OpenAI agent from the Smart Library.
        
        Args:
            input: The execution parameters
        
        Returns:
            StringToolOutput containing the execution result
        """
        try:
            # 1. Get the agent record
            agent_record = await self._get_agent_record(input.agent_id_or_name)
            if not agent_record:
                return StringToolOutput(json.dumps({
                    "status": "error",
                    "message": f"Agent '{input.agent_id_or_name}' not found"
                }, indent=2))
            
            # 2. Verify it's an OpenAI agent
            metadata = agent_record.get("metadata", {})
            framework = metadata.get("framework", "").lower()
            
            if framework != "openai-agents":
                return StringToolOutput(json.dumps({
                    "status": "error",
                    "message": f"Agent '{input.agent_id_or_name}' is not an OpenAI agent"
                }, indent=2))
            
            # 3. Get or create the agent instance
            agent, tools = await self._get_or_create_agent(agent_record, input.tools_to_use, input.apply_firmware)
            
            # 4. Execute the agent
            result = await self._execute_agent(agent, input.input_text, input.max_turns)
            
            # 5. Format and return the result
            return StringToolOutput(json.dumps({
                "status": "success",
                "agent_name": agent_record["name"],
                "input": input.input_text,
                "output": result["result"],
                "tools_used": [t.name for t in tools] if tools else [],
                "execution_details": {
                    "max_turns": input.max_turns,
                    "firmware_applied": input.apply_firmware
                }
            }, indent=2))
            
        except Exception as e:
            import traceback
            logger.error(f"Error executing OpenAI agent: {str(e)}")
            logger.error(traceback.format_exc())
            
            return StringToolOutput(json.dumps({
                "status": "error",
                "message": f"Error executing OpenAI agent: {str(e)}",
                "details": traceback.format_exc()
            }, indent=2))
    
    async def _get_agent_record(self, agent_id_or_name: str) -> Optional[Dict[str, Any]]:
        """Get an agent record by ID or name."""
        # Try by ID first
        record = await self.library.find_record_by_id(agent_id_or_name)
        if record:
            return record
        
        # Try by name for AGENT type
        return await self.library.find_record_by_name(agent_id_or_name, "AGENT")
    
    async def _get_or_create_agent(
        self, 
        record: Dict[str, Any], 
        tool_names: Optional[List[str]] = None,
        apply_firmware: bool = True
    ) -> Tuple[OpenAIAgent, List[Any]]:
        """Get an existing agent instance or create a new one."""
        # Check if we already have an active instance
        if record["id"] in self.active_agents:
            return self.active_agents[record["id"]]["instance"], self.active_agents[record["id"]]["tools"]
        
        # Resolve tools if names provided
        tools = []
        if tool_names and self.tool_factory:
            for tool_name in tool_names:
                # Find tool record
                tool_record = await self.library.find_record_by_name(tool_name, "TOOL")
                if tool_record:
                    # Create tool instance
                    tool = await self.tool_factory.create_tool(tool_record)
                    tools.append(tool)
        
        # Convert tools to OpenAI format
        openai_tools = []
        if tools:
            openai_tools = [OpenAIToolAdapter.convert_evolving_tool_to_openai(t) for t in tools]
        
        # Create the agent using the provider
        agent = await self.provider.create_agent(
            record=record,
            tools=openai_tools,
            firmware_content="Apply governance rules" if apply_firmware else None,
            config={"apply_firmware": apply_firmware}
        )
        
        # Store in active agents
        self.active_agents[record["id"]] = {
            "instance": agent,
            "tools": tools,
            "record": record
        }
        
        return agent, tools
    
    async def _execute_agent(
        self, 
        agent: OpenAIAgent, 
        input_text: str,
        max_turns: int = 10
    ) -> Dict[str, Any]:
        """Execute an OpenAI agent with input text."""
        try:
            # Create context for execution
            context = {}
            
            # Execute the agent
            result = await Runner.run(
                agent,
                input_text,
                context=context,
                max_turns=max_turns
            )
            
            return {
                "status": "success",
                "result": str(result.final_output),
                "raw_result": result
            }
            
        except Exception as e:
            logger.error(f"Error in _execute_agent: {str(e)}")
            import traceback
            logger.error(traceback.format_exc())
            
            raise