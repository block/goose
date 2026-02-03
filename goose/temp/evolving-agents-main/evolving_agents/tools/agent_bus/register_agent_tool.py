# evolving_agents/tools/agent_bus/register_agent_tool.py

from typing import Dict, Any, List, Optional, Union
from pydantic import BaseModel, Field
import json

from beeai_framework.tools.tool import Tool, StringToolOutput
from beeai_framework.context import RunContext
from beeai_framework.emitter.emitter import Emitter

class CapabilityModel(BaseModel):
    """Model for a capability."""
    id: str = Field(description="Unique identifier for the capability")
    name: str = Field(description="Human-readable name for the capability")
    description: str = Field(description="Description of what the capability does")
    confidence: float = Field(0.8, description="Confidence level for this capability (0.0-1.0)")

class RegisterInput(BaseModel):
    """Input schema for the RegisterAgentTool."""
    name: str = Field(description="Name of the agent to register")
    agent_type: Optional[str] = Field(None, description="Type of agent")
    capabilities: List[Union[str, Dict[str, Any], CapabilityModel]] = Field(
        description="List of capabilities provided by this agent"
    )
    description: Optional[str] = Field(None, description="Description of the agent")
    metadata: Optional[Dict[str, Any]] = Field(None, description="Additional metadata")

class RegisterAgentTool(Tool[RegisterInput, None, StringToolOutput]):
    """
    Tool for registering agents with the Agent Bus.
    """
    name = "RegisterAgentTool"
    description = "Register agents and their capabilities with the Agent Bus"
    input_schema = RegisterInput
    
    def __init__(
        self, 
        agent_bus,
        options: Optional[Dict[str, Any]] = None
    ):
        super().__init__(options=options or {})
        self.agent_bus = agent_bus
    
    def _create_emitter(self) -> Emitter:
        return Emitter.root().child(
            namespace=["tool", "agent_bus", "register"],
            creator=self,
        )
    
    async def _run(self, input: RegisterInput, options: Optional[Dict[str, Any]] = None, context: Optional[RunContext] = None) -> StringToolOutput:
        """
        Register an agent with the Agent Bus.
        """
        try:
            processed_capabilities = []
            for cap in input.capabilities:
                if isinstance(cap, str):
                    processed_capabilities.append({
                        "id": cap.lower().replace(" ", "_"),
                        "name": cap,
                        "description": f"Ability to {cap.lower()}",
                        "confidence": 0.8
                    })
                elif isinstance(cap, dict):
                    if "id" not in cap:
                        cap["id"] = cap.get("name", "capability").lower().replace(" ", "_")
                    if "name" not in cap:
                        cap["name"] = cap["id"].replace("_", " ").title()
                    if "description" not in cap:
                        cap["description"] = f"Ability to {cap['name'].lower()}"
                    if "confidence" not in cap:
                        cap["confidence"] = 0.8
                    processed_capabilities.append(cap)
                else:
                    processed_capabilities.append(cap.dict())
            
            agent_id = await self.agent_bus.register_agent(
                name=input.name,
                capabilities=processed_capabilities,
                agent_type=input.agent_type or "GENERIC",
                description=input.description or f"Agent providing {len(processed_capabilities)} capabilities",
                metadata=input.metadata or {}
            )
            
            return StringToolOutput(json.dumps({
                "status": "success",
                "message": f"Successfully registered agent '{input.name}'",
                "agent_id": agent_id,
                "capabilities": processed_capabilities
            }, indent=2))
            
        except Exception as e:
            import traceback
            return StringToolOutput(json.dumps({
                "status": "error",
                "message": f"Error registering agent: {str(e)}",
                "details": traceback.format_exc()
            }, indent=2))
    
    async def register(self, name: str, capabilities: List[Union[str, Dict[str, Any]]]) -> str:
        """Register an agent with its capabilities."""
        processed_capabilities = []
        for cap in capabilities:
            if isinstance(cap, str):
                processed_capabilities.append({
                    "id": cap.lower().replace(" ", "_"),
                    "name": cap,
                    "description": f"Ability to {cap.lower()}",
                    "confidence": 0.8
                })
            else:
                processed_capabilities.append(cap)
        
        return await self.agent_bus.register_agent(
            name=name,
            capabilities=processed_capabilities
        )
    
    async def update_capabilities(self, agent_id: str, capabilities: List[Union[str, Dict[str, Any]]]) -> bool:
        """Update an agent's capabilities."""
        processed_capabilities = []
        for cap in capabilities:
            if isinstance(cap, str):
                processed_capabilities.append({
                    "id": cap.lower().replace(" ", "_"),
                    "name": cap,
                    "description": f"Ability to {cap.lower()}",
                    "confidence": 0.8
                })
            else:
                processed_capabilities.append(cap)
        
        return await self.agent_bus.update_agent_capabilities(
            agent_id=agent_id,
            capabilities=processed_capabilities
        )
    
    async def deregister(self, agent_id: str) -> bool:
        """Deregister an agent from the Agent Bus."""
        return await self.agent_bus.deregister_agent(agent_id)