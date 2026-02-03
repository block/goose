# examples/smart_agent_bus/dual_bus_demo.py

import asyncio
import logging
import os
import json
from typing import Dict, Any, List, Optional

# Configure logging
logging.basicConfig(
    level=logging.DEBUG, # Changed to DEBUG for more detailed output
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)

# Import toolkit components
from evolving_agents.core.dependency_container import DependencyContainer
from evolving_agents.core.llm_service import LLMService
from evolving_agents.smart_library.smart_library import SmartLibrary
from evolving_agents.agent_bus.smart_agent_bus import SmartAgentBus
from evolving_agents.core.system_agent import SystemAgentFactory
from evolving_agents.firmware.firmware import Firmware
from evolving_agents.core.mongodb_client import MongoDBClient

# Additional imports for creating agent instances
from beeai_framework.agents.react import ReActAgent
from beeai_framework.agents.types import AgentMeta
from beeai_framework.memory import TokenMemory

# Create mock agent instances
def create_mock_agent(llm, name, description):
    """Create a simple mock agent for demonstration purposes."""
    meta = AgentMeta(
        name=name,
        description=description,
        tools=[]
    )
    agent = ReActAgent(
        llm=llm,
        tools=[],
        memory=TokenMemory(llm),
        meta=meta
    )
    return agent

# Custom function to format the agent bus logs for display
async def format_bus_logs(agent_bus: SmartAgentBus) -> Dict[str, List[Dict[str, Any]]]:
    """Format agent bus logs to display System Bus vs Data Bus operations."""
    logs = await agent_bus.get_logs(limit=200)
    
    # Split logs by bus type
    system_bus_logs = []
    data_bus_logs = []
    
    for log_entry in logs: # Renamed 'log' to 'log_entry' to avoid conflict
        # Create a simplified log entry
        simplified_log = {
            "timestamp": log_entry.get("timestamp", ""), # Assuming timestamp is already a string or datetime object
            "agent_name": log_entry.get("agent_name", ""),
            "task_description": log_entry.get("task_description", ""),
            "success": log_entry.get("error") is None # Check if 'error' field is None or not present
        }
        
        # Add to the appropriate bus list
        if log_entry.get("bus_type") == "system":
            system_bus_logs.append(simplified_log)
        else:
            data_bus_logs.append(simplified_log)
    
    return {
        "system_bus": system_bus_logs,
        "data_bus": data_bus_logs
    }

# Mock responses for the capability requests
async def mock_weather_response(weather_request):
    """Generate a mock weather response."""
    location = weather_request.get("location", "unknown")
    days = weather_request.get("days", 1)
    
    return {
        "forecast": [
            {
                "day": i+1,
                "condition": "Sunny" if i % 2 == 0 else "Partly Cloudy",
                "temperature": 25 - i,
                "precipitation": f"{i*10}%"
            }
            for i in range(days)
        ],
        "location": location,
        "units": "celsius"
    }

async def mock_translation_response(translation_request):
    """Generate a mock translation response."""
    source_text = translation_request.get("source_text", "")
    target_language = translation_request.get("target_language", "Spanish")
    
    translations = {
        "Hello, how are you today?": {
            "Spanish": "Hola, ¿cómo estás hoy?",
            "French": "Bonjour, comment allez-vous aujourd'hui?",
            "German": "Hallo, wie geht es Ihnen heute?"
        }
    }
    
    if source_text in translations and target_language in translations[source_text]:
        translated_text = translations[source_text][target_language]
    else:
        translated_text = f"[Translated to {target_language}: {source_text}]"
    
    return {
        "translated_text": translated_text,
        "source_language": translation_request.get("source_language", "English"),
        "target_language": target_language,
        "confidence": 0.92
    }

async def agent_bus_demo():
    """Demonstrate the dual bus architecture with System Bus and Data Bus operations."""
    
    mongodb_client = None # Initialize for the finally block
    try:
        # MongoDB Configuration
        mongodb_uri = os.environ.get("EAT_MONGODB_URI", "mongodb://localhost:27017")
        mongodb_db_name = os.environ.get("EAT_MONGODB_DB_NAME", "evolving_agents_db_demo")

        # Start with clean state for circuit breaker file
        files_to_clean = ["agent_bus_circuit_breakers_demo.json"]
        for file_path in files_to_clean: # Renamed 'file' to 'file_path'
            if os.path.exists(file_path):
                os.remove(file_path)
        
        # Setup the environment
        container = DependencyContainer()
        
        # Create dependencies
        llm_service = LLMService(provider="openai", model="gpt-4o") # Ensure you have OPENAI_API_KEY set
        container.register('llm_service', llm_service)

        # MongoDB Client Setup
        logger.info(f"Connecting to MongoDB: URI={mongodb_uri}, DB={mongodb_db_name}")
        mongodb_client = MongoDBClient(uri=mongodb_uri, db_name=mongodb_db_name)
        # No async initialize method in MongoDBClient, connection is attempted in __init__
        # await mongodb_client.ping_server() # Optionally ping to confirm connection early
        container.register('mongodb_client', mongodb_client)
        
        smart_library = SmartLibrary(llm_service=llm_service, container=container)
        container.register('smart_library', smart_library)
        
        firmware = Firmware()
        container.register('firmware', firmware)
        
        # Create the Agent Bus - core component for this demo
        agent_bus = SmartAgentBus(
            container=container,
            circuit_breaker_path="agent_bus_circuit_breakers_demo.json" # New path
        )
        container.register('agent_bus', agent_bus)
        
        # Create system agent
        system_agent = await SystemAgentFactory.create_agent(container=container)
        container.register('system_agent', system_agent)
        
        # Initialize components
        await smart_library.initialize()
        await agent_bus.initialize_from_library()
        
        logger.info("=== Agent Bus Dual Architecture Demo ===")
        logger.info("This demo shows how the SmartAgentBus uses System Bus and Data Bus operations")
        
        #########################################
        # PHASE 1: SYSTEM BUS - AGENT REGISTRATION
        #########################################
        logger.info("\n--- Phase 1: System Bus - Agent Registration ---")
        
        # 1. Register several specialized agents using System Bus operations
        logger.info("Registering agents on the System Bus...")
        
        # Create mock agent instances
        weather_agent = create_mock_agent(
            llm_service.chat_model,
            "WeatherAgent",
            "Agent that provides weather information for locations"
        )
        
        translation_agent = create_mock_agent(
            llm_service.chat_model,
            "TranslationAgent",
            "Agent that translates text between languages"
        )
        
        travel_agent = create_mock_agent(
            llm_service.chat_model,
            "TravelAssistant",
            "Agent that helps with travel planning"
        )
        
        # Weather capability agent
        weather_agent_id = await agent_bus.register_agent(
            name="WeatherAgent",
            description="Agent that provides weather information for locations",
            capabilities=[
                {
                    "id": "weather_forecast",
                    "name": "Weather Forecast",
                    "description": "Get weather forecasts for locations",
                    "confidence": 0.9
                },
                {
                    "id": "temperature_conversion",
                    "name": "Temperature Conversion",
                    "description": "Convert between temperature units",
                    "confidence": 0.95
                }
            ],
            agent_type="SPECIALIZED",
            metadata={"domain": "weather"},
            agent_instance=weather_agent
        )
        logger.info(f"Registered WeatherAgent with ID {weather_agent_id}")
        
        # Translation capability agent
        translation_agent_id = await agent_bus.register_agent(
            name="TranslationAgent",
            description="Agent that translates text between languages",
            capabilities=[
                {
                    "id": "text_translation",
                    "name": "Text Translation",
                    "description": "Translate text between languages",
                    "confidence": 0.85
                }
            ],
            agent_type="SPECIALIZED",
            metadata={"domain": "language"},
            agent_instance=translation_agent
        )
        logger.info(f"Registered TranslationAgent with ID {translation_agent_id}")
        
        # Multi-capable agent
        travel_agent_id = await agent_bus.register_agent(
            name="TravelAssistant",
            description="Agent that helps with travel planning",
            capabilities=[
                {
                    "id": "travel_planning",
                    "name": "Travel Planning",
                    "description": "Plan travel itineraries",
                    "confidence": 0.8
                },
                {
                    "id": "weather_forecast",
                    "name": "Weather Forecast",
                    "description": "Get weather forecasts for travel destinations",
                    "confidence": 0.7
                }
            ],
            agent_type="SPECIALIZED",
            metadata={"domain": "travel"},
            agent_instance=travel_agent
        )
        logger.info(f"Registered TravelAssistant with ID {travel_agent_id}")
        
        #########################################
        # PHASE 2: SYSTEM BUS - AGENT DISCOVERY
        #########################################
        logger.info("\n--- Phase 2: System Bus - Agent Discovery ---")
        
        # 2a. Discover agents by capability (System Bus operation)
        logger.info("Discovering agents with weather capabilities...")
        weather_capable_agents = await agent_bus.discover_agents(
            capability_id="weather_forecast",
            min_confidence=0.6
        )
        
        logger.info(f"Found {len(weather_capable_agents)} agents with weather capabilities:")
        for agent in weather_capable_agents:
            logger.info(f"  - {agent['name']} (Confidence: {agent.get('similarity_score', 0):.2f})")
        
        # 2b. Discover agents by task description (System Bus operation)
        logger.info("\nDiscovering agents for language translation tasks...")
        translation_agents = await agent_bus.discover_agents(
            task_description="Translate text from English to Spanish",
            min_confidence=0.6,
            limit=3
        )
        
        logger.info(f"Found {len(translation_agents)} agents for translation tasks:")
        for agent in translation_agents:
            logger.info(f"  - {agent['name']} (Similarity: {agent.get('similarity_score', 0):.2f})")
        
        #########################################
        # PHASE 3: DATA BUS - CAPABILITY REQUESTS
        #########################################
        logger.info("\n--- Phase 3: Data Bus - Capability Requests ---")
        
        # Mock internal bus behavior for weather prediction
        logger.info("Requesting weather forecast capability...")
        weather_request = {
            "location": "New York",
            "days": 3,
            "text": "What's the weather like in New York for the next 3 days?"
        }
        
        # Use the real agent bus if available, otherwise simulate
        try:
            # Monkey patch the _execute_agent_task method temporarily to use our mock
            original_execute = agent_bus._execute_agent_task
            
            async def mock_execute_weather(agent_record, task):
                if "weather" in agent_record["name"].lower():
                    return await mock_weather_response(task)
                elif "translation" in agent_record["name"].lower():
                    return await mock_translation_response(task)
                else:
                    return {"error": "Unknown agent type for mocking"}
            
            # Apply the monkey patch
            agent_bus._execute_agent_task = mock_execute_weather
            
            # Now make the request through the bus
            weather_response = await agent_bus.request_capability(
                capability="weather_forecast",
                content=weather_request,
                min_confidence=0.7
            )
            
            logger.info(f"Weather capability response from: {weather_response.get('agent_name', 'Unknown')}")
            logger.info(f"Response content: {json.dumps(weather_response.get('content', {}), indent=2)}")
            
            # Request translation capability via Data Bus
            logger.info("\nRequesting translation capability...")
            translation_request = {
                "source_text": "Hello, how are you today?",
                "source_language": "English",
                "target_language": "Spanish",
                "text": "Translate 'Hello, how are you today?' from English to Spanish."
            }
            
            translation_response = await agent_bus.request_capability(
                capability="text_translation",
                content=translation_request,
                min_confidence=0.7
            )
            
            logger.info(f"Translation capability response from: {translation_response.get('agent_name', 'Unknown')}")
            logger.info(f"Response content: {json.dumps(translation_response.get('content', {}), indent=2)}")
            
            # Restore the original method
            agent_bus._execute_agent_task = original_execute
            
        except Exception as e:
            logger.info(f"Exception during capability request: {str(e)}")
            logger.info("Using simulated capability responses instead...")
            
            # Simulate the weather response
            weather_result = await mock_weather_response(weather_request)
            logger.info("Simulated weather capability response:")
            logger.info(json.dumps(weather_result, indent=2))
            
            # Simulate the translation response
            translation_request = {
                "source_text": "Hello, how are you today?",
                "source_language": "English",
                "target_language": "Spanish"
            }
            translation_result = await mock_translation_response(translation_request)
            logger.info("\nSimulated translation capability response:")
            logger.info(json.dumps(translation_result, indent=2))
        
        #########################################
        # PHASE 4: SYSTEM BUS - SYSTEM MANAGEMENT
        #########################################
        logger.info("\n--- Phase 4: System Bus - System Management ---")
        
        # 4a. List all agents (System Bus operation)
        all_agents = await agent_bus.list_all_agents()
        logger.info(f"Total registered agents: {len(all_agents)}")
        for agent in all_agents:
            logger.info(f"  - {agent['name']} ({agent['type']}): {agent['description_snippet']}")
        
        # 4b. Get status of specific agent (System Bus operation)
        try:
            if weather_capable_agents:
                weather_agent_id = weather_capable_agents[0]["id"]
                logger.info(f"\nChecking status of WeatherAgent (ID: {weather_agent_id})")
                agent_status = await agent_bus.get_agent_status(weather_agent_id)
                logger.info(f"Agent Status: {agent_status.get('status', 'unknown')}")
                logger.info(f"Health Status: {agent_status.get('health_status', 'unknown')}")
                logger.info(f"Is Instance Loaded: {agent_status.get('is_instance_loaded', False)}")
        except Exception as e:
            logger.info(f"Error checking agent status: {str(e)}")
        
        #########################################
        # DISPLAY BUS LOGS AND ARCHITECTURE
        #########################################
        logger.info("\n--- Smart Agent Bus Activity Summary ---")
        
        # Display the bus logs categorized by bus type
        # Call the updated async function
        formatted_logs = await format_bus_logs(agent_bus)
        
        logger.info("\nSystem Bus Operations:")
        for log in formatted_logs["system_bus"]:
            logger.info(f"  - {log['task_description']} on {log['agent_name']} ({log['timestamp']})")
        
        logger.info("\nData Bus Operations:")
        for log in formatted_logs["data_bus"]:
            logger.info(f"  - {log['task_description']} on {log['agent_name']} ({log['timestamp']})")
        
        logger.info("\n=== Bus Architecture Summary ===")
        logger.info("SYSTEM BUS: Used for management operations")
        logger.info("  - register_agent: Register agents and their capabilities")
        logger.info("  - discover_agents: Find agents based on capabilities or tasks")
        logger.info("  - get_agent_status: Check agent health and status")
        logger.info("  - list_all_agents: Get information about all registered agents")
        
        logger.info("\nDATA BUS: Used for agent communication and task execution")
        logger.info("  - request_capability: Route requests to agents based on capability")
        
        logger.info("\nDual Bus Architecture enables:")
        logger.info("  - Decoupled communication based on capabilities, not direct references")
        logger.info("  - Dynamic discovery and routing of requests")
        logger.info("  - Centralized monitoring and management")
        logger.info("  - Circuit breaking for unhealthy agents")
        logger.info("=================================================")
        
        return { # This block remains indented
            "registered_agents": {
                "weather": weather_agent_id,
                "translation": translation_agent_id,
                "travel": travel_agent_id
            },
            "bus_activity": formatted_logs
        }
    finally:
        if mongodb_client:
            logger.info("Closing MongoDB client connection...")
            mongodb_client.close() # Reverted to synchronous close method

if __name__ == "__main__":
    # Ensure environment variables are set for OpenAI API Key for LLMService
    # and optionally EAT_MONGODB_URI, EAT_MONGODB_DB_NAME
    # Example:
    # export OPENAI_API_KEY="your_key_here"
    # export EAT_MONGODB_URI="mongodb://user:pass@host:port" 
    # export EAT_MONGODB_DB_NAME="your_db_name"
    asyncio.run(agent_bus_demo())