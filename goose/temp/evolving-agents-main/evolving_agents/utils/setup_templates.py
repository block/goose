# evolving_agents/utils/setup_templates.py

import os
import logging

logger = logging.getLogger(__name__)

def setup_templates():
    """Set up template files for BeeAI agents and tools."""
    
    # Get path to templates directory
    templates_dir = os.path.join(os.path.dirname(os.path.dirname(__file__)), "templates")
    
    # Make sure the templates directory exists
    os.makedirs(templates_dir, exist_ok=True)
    
    # Define template content - avoid triple quotes by using single quotes for docstrings
    bee_agent_template = '''from beeai_framework.agents.react import ReActAgent
from beeai_framework.agents.types import AgentMeta
from beeai_framework.memory import TokenMemory, UnconstrainedMemory
from beeai_framework.backend.chat import ChatModel
from beeai_framework.tools.tool import Tool

class WeatherAgentInitializer:
    """Agent that provides weather information for locations.
    This agent can answer questions about current weather, forecasts, and historical weather data."""
    
    @staticmethod
    def create_agent(llm: ChatModel, tools: List[Tool] = None) -> ReActAgent:
        # Define which tools the agent will use (if they're not provided)
        if tools is None:
            # No tools available - this is just an example
            tools = []
        
        # Create agent metadata
        meta = AgentMeta(
            name="WeatherAgent",
            description=(
                "I am a weather assistant that can provide current weather conditions, "
                "forecasts, and historical weather data for locations around the world."
            ),
            tools=tools
        )
        
        # Create the agent with proper memory
        agent = ReActAgent(
            llm=llm,
            tools=tools,
            memory=TokenMemory(llm),
            meta=meta
        )
        
        return agent
'''

    bee_tool_template = '''from typing import Dict, Any
from pydantic import BaseModel, Field

from beeai_framework.context import RunContext
from beeai_framework.emitter.emitter import Emitter
from beeai_framework.tools.tool import StringToolOutput, Tool, ToolRunOptions

class WeatherToolInput(BaseModel):
    location: str = Field(description="Location to get weather information for (city, country)")
    days: int = Field(description="Number of days for forecast (1-7)", default=1, ge=1, le=7)

class WeatherTool(Tool[WeatherToolInput, ToolRunOptions, StringToolOutput]):
    """Retrieves weather information for a specified location."""
    name = "WeatherTool"
    description = "Get current weather and forecast information for locations worldwide"
    input_schema = WeatherToolInput

    def _create_emitter(self) -> Emitter:
        return Emitter.root().child(
            namespace=["tool", "weather"],
            creator=self,
        )
    
    async def _run(self, input: WeatherToolInput, options: ToolRunOptions | None, context: RunContext) -> StringToolOutput:
        try:
            # In a real implementation, you would call a weather API here
            location = input.location
            days = input.days
            
            # Mock response for demonstration
            weather_data = {
                "location": location,
                "current": {
                    "temperature": 22,
                    "condition": "Sunny",
                    "humidity": 60,
                    "wind_speed": 5
                },
                "forecast": [
                    {"day": i+1, "condition": "Sunny", "max_temp": 24, "min_temp": 18}
                    for i in range(days)
                ]
            }
            
            # Return formatted response
            import json
            return StringToolOutput(json.dumps(weather_data, indent=2))
            
        except Exception as e:
            return StringToolOutput(f"Error retrieving weather information: {str(e)}")
'''

    # Write template files
    agent_template_path = os.path.join(templates_dir, "beeai_agent_template.txt")
    tool_template_path = os.path.join(templates_dir, "beeai_tool_template.txt")
    
    try:
        with open(agent_template_path, 'w') as f:
            f.write(bee_agent_template)
        logger.info(f"Created BeeAI agent template at {agent_template_path}")
        
        with open(tool_template_path, 'w') as f:
            f.write(bee_tool_template)
        logger.info(f"Created BeeAI tool template at {tool_template_path}")
        
        return True
    except Exception as e:
        logger.error(f"Error creating template files: {str(e)}")
        return False