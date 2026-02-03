# evolving_agents/tools/agent_bus/request_agent_tool.py

from typing import Dict, Any, List, Optional, Union
from pydantic import BaseModel, Field
import json

from beeai_framework.tools.tool import Tool, StringToolOutput
from beeai_framework.context import RunContext
from beeai_framework.emitter.emitter import Emitter

class RequestInput(BaseModel):
    """Input schema for the RequestAgentTool."""
    capability: str = Field(description="Capability to request")
    content: Union[str, Dict[str, Any]] = Field(description="Content of the request")
    specific_agent: Optional[str] = Field(None, description="Specific agent to request from")
    min_confidence: float = Field(0.5, description="Minimum confidence level required (0.0-1.0)")
    metadata: Optional[Dict[str, Any]] = Field(None, description="Additional request metadata")

class RequestAgentTool(Tool[RequestInput, None, StringToolOutput]):
    """
    Tool for requesting capabilities from agents through the Agent Bus.
    """
    name = "RequestAgentTool"
    description = "Request capabilities from agents in the ecosystem"
    input_schema = RequestInput
    
    def __init__(
        self, 
        agent_bus,
        options: Optional[Dict[str, Any]] = None
    ):
        super().__init__(options=options or {})
        self.agent_bus = agent_bus
    
    def _create_emitter(self) -> Emitter:
        return Emitter.root().child(
            namespace=["tool", "agent_bus", "request"],
            creator=self,
        )
    
    async def _run(self, input: RequestInput, options: Optional[Dict[str, Any]] = None, context: Optional[RunContext] = None) -> StringToolOutput:
        """
        Request a capability from an agent.
        """
        try:
            if isinstance(input.content, dict):
                content = input.content
            else:
                content = {"text": input.content}
            
            if input.metadata:
                content["metadata"] = input.metadata
            
            response = await self.agent_bus.request_capability(
                capability=input.capability,
                content=content,
                agent_id=input.specific_agent,
                min_confidence=input.min_confidence
            )
            
            return StringToolOutput(json.dumps({
                "status": "success",
                "capability": input.capability,
                "agent": response.get("agent_id", "unknown"),
                "agent_name": response.get("agent_name", "Unknown Agent"),
                "content": response.get("content", {}),
                "confidence": response.get("confidence", 0.0)
            }, indent=2))
            
        except Exception as e:
            import traceback
            return StringToolOutput(json.dumps({
                "status": "error",
                "message": f"Error requesting capability: {str(e)}",
                "details": traceback.format_exc()
            }, indent=2))
    
    async def request(self, capability: str, content: Union[str, Dict[str, Any]], specific_agent: Optional[str] = None) -> Dict[str, Any]:
        """Request a capability from an agent."""
        if isinstance(content, dict):
            content_dict = content
        else:
            content_dict = {"text": content}
        
        return await self.agent_bus.request_capability(
            capability=capability,
            content=content_dict,
            agent_id=specific_agent
        )
    
    async def call_agent(self, agent_name: str, input_text: str, metadata: Optional[Dict[str, Any]] = None) -> Dict[str, Any]:
        """Call a specific agent directly by name."""
        content = {"text": input_text}
        if metadata:
            content["metadata"] = metadata
        
        agent = await self.agent_bus.find_agent_by_name(agent_name)
        if not agent:
            raise ValueError(f"Agent '{agent_name}' not found")
        
        return await self.agent_bus.request_capability(
            capability="process_input",
            content=content,
            agent_id=agent["id"]
        )
    
    async def send_message(self, to_agent: str, message: str, from_agent: Optional[str] = None) -> Dict[str, Any]:
        """Send a message between agents."""
        content = {
            "message": message,
            "from": from_agent or "SystemAgent"
        }
        
        recipient = await self.agent_bus.find_agent_by_name(to_agent)
        if not recipient:
            raise ValueError(f"Agent '{to_agent}' not found")
        
        return await self.agent_bus.request_capability(
            capability="receive_message",
            content=content,
            agent_id=recipient["id"]
        )