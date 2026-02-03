# evolving_agents/tools/agent_bus/discover_agent_tool.py

from typing import Dict, Any, List, Optional, Union
from pydantic import BaseModel, Field
import json

from beeai_framework.tools.tool import Tool, StringToolOutput
from beeai_framework.context import RunContext
from beeai_framework.emitter.emitter import Emitter

class DiscoverInput(BaseModel):
    """Input schema for the DiscoverAgentTool."""
    query: Optional[str] = Field(None, description="Search query for capabilities")
    capability_id: Optional[str] = Field(None, description="Specific capability ID to look for")
    agent_type: Optional[str] = Field(None, description="Filter by agent type")
    min_confidence: float = Field(0.5, description="Minimum confidence level required (0.0-1.0)")
    limit: int = Field(10, description="Maximum number of results to return")

class DiscoverAgentTool(Tool[DiscoverInput, None, StringToolOutput]):
    """
    Tool for discovering agents and their capabilities in the Agent Bus.
    """
    name = "DiscoverAgentTool"
    description = "Discover available agents and their capabilities in the ecosystem"
    input_schema = DiscoverInput
    
    def __init__(
        self, 
        agent_bus,
        options: Optional[Dict[str, Any]] = None
    ):
        super().__init__(options=options or {})
        self.agent_bus = agent_bus
    
    def _create_emitter(self) -> Emitter:
        return Emitter.root().child(
            namespace=["tool", "agent_bus", "discover"],
            creator=self,
        )
    
    async def _run(self, input: DiscoverInput, options: Optional[Dict[str, Any]] = None, context: Optional[RunContext] = None) -> StringToolOutput:
        """
        Discover agents and capabilities in the Agent Bus.
        """
        try:
            if input.capability_id:
                # Find agents with specific capability
                agents = await self.agent_bus.find_agents_for_capability(
                    capability=input.capability_id,
                    min_confidence=input.min_confidence,
                    agent_type=input.agent_type
                )
                
                result = {
                    "status": "success",
                    "capability": input.capability_id,
                    "agent_count": len(agents),
                    "agents": []
                }
                
                for agent in agents:
                    capability_details = None
                    for cap in agent.get("capabilities", []):
                        if cap.get("id") == input.capability_id:
                            capability_details = cap
                            break
                    
                    result["agents"].append({
                        "id": agent["id"],
                        "name": agent["name"],
                        "type": agent.get("type", "AGENT"),
                        "description": agent.get("description", "No description"),
                        "capability_confidence": capability_details.get("confidence", 0.0) if capability_details else 0.0
                    })
                
            elif input.query:
                # Search for capabilities matching the query
                capabilities = await self.agent_bus.search_capabilities(
                    query=input.query,
                    min_confidence=input.min_confidence,
                    agent_type=input.agent_type,
                    limit=input.limit
                )
                
                result = {
                    "status": "success",
                    "query": input.query,
                    "capability_count": len(capabilities),
                    "capabilities": []
                }
                
                for capability in capabilities:
                    result["capabilities"].append({
                        "id": capability["id"],
                        "name": capability["name"],
                        "description": capability.get("description", "No description"),
                        "agents": [
                            {
                                "id": a["id"],
                                "name": a["name"],
                                "confidence": a.get("confidence", 0.0)
                            }
                            for a in capability.get("agents", [])
                        ]
                    })
                
            else:
                # List all capabilities
                all_capabilities = await self.agent_bus.list_all_capabilities(
                    agent_type=input.agent_type,
                    min_confidence=input.min_confidence,
                    limit=input.limit
                )
                
                result = {
                    "status": "success",
                    "capability_count": len(all_capabilities),
                    "capabilities": []
                }
                
                for capability in all_capabilities:
                    result["capabilities"].append({
                        "id": capability["id"],
                        "name": capability["name"],
                        "description": capability.get("description", "No description"),
                        "agent_count": len(capability.get("agents", []))
                    })
            
            return StringToolOutput(json.dumps(result, indent=2))
            
        except Exception as e:
            import traceback
            return StringToolOutput(json.dumps({
                "status": "error",
                "message": f"Error discovering capabilities: {str(e)}",
                "details": traceback.format_exc()
            }, indent=2))
    
    async def list_agents(self, agent_type: Optional[str] = None) -> List[Dict[str, Any]]:
        """List all registered agents."""
        return await self.agent_bus.list_all_agents(agent_type)
    
    async def find_agent(self, capability: str, min_confidence: float = 0.5) -> Optional[Dict[str, Any]]:
        """Find an agent for a specific capability."""
        agents = await self.agent_bus.find_agents_for_capability(
            capability=capability,
            min_confidence=min_confidence
        )
        
        if not agents:
            return None
            
        return max(agents, key=lambda a: next(
            (c.get("confidence", 0.0) for c in a.get("capabilities", []) 
             if c.get("id") == capability), 
            0.0
        ))
    
    async def get_capabilities(self, agent_id: str) -> List[Dict[str, Any]]:
        """Get all capabilities for a specific agent."""
        agent = await self.agent_bus.get_agent(agent_id)
        if not agent:
            return []
        return agent.get("capabilities", [])