# examples/autocomplete/run_autocomplete_system.py

import asyncio
import json
from typing import List
from evolving_agents.agents.architect_zero import create_architect_zero
# from evolving_agents.smart_library.smart_library import SmartLibrary
from evolving_agents.core.system_agent import SystemAgentFactory
from evolving_agents.core.llm_service import LLMService
from evolving_agents.agent_bus.smart_agent_bus import SmartAgentBus
from evolving_agents.core.dependency_container import DependencyContainer
from evolving_agents.core.mongodb_client import MongoDBClient
from evolving_agents import config as eat_config

import os
from dotenv import load_dotenv

# Load environment variables from .env file
load_dotenv()

# Check for essential environment variables
if not os.getenv("MONGODB_URI"):
    print("ERROR: MONGODB_URI not set in environment variables or .env file.")
    # Depending on the application's needs, you might exit or raise an error here
if not os.getenv("OPENAI_API_KEY") and not os.getenv("ANTHROPIC_API_KEY"): # Or other LLM provider keys
    print("WARNING: Neither OPENAI_API_KEY nor ANTHROPIC_API_KEY (or other LLM provider key) is set.")
    print("The system may not function correctly without an LLM API key.")
# Add checks for other critical variables like MONGODB_DATABASE_NAME if they are sourced from os.getenv directly
# For this example, we assume MONGODB_DATABASE_NAME is managed via eat_config


async def run_smart_autocomplete(
    prompt: str, 
    inputs: List[dict], 
    output_path: str
):
    """
    Run a context-aware autocomplete system that learns from multiple sources.
    
    Args:
        prompt: Instructions for the Architect agent
        inputs: List of dictionaries containing texts from different sources
        output_path: Where to save the autocomplete suggestions
    """
    # Create dependency container to manage component dependencies
    container = DependencyContainer()

    # Initialize MongoDBClient
    # TODO: Check eat_config for any MingiDB-specific MongoDB configurations
    # (e.g., specific collection names or other parameters) and ensure they are used
    # by the MingiDB component when it's initialized.
    mongo_uri = eat_config.MONGODB_URI
    mongo_db_name = eat_config.MONGODB_DATABASE_NAME
    if not mongo_uri or not mongo_db_name:
        print("ERROR: MONGODB_URI or MONGODB_DATABASE_NAME not set in .env or eat_config. Please configure them.")
        # Consider raising an error or exiting if essential
        return # Or raise ConfigurationError

    mongodb_client = MongoDBClient(uri=mongo_uri, db_name=mongo_db_name)
    if not await mongodb_client.ping_server():
        print(f"ERROR: Failed to connect to MongoDB at {mongo_uri} / {mongo_db_name}. Please check your settings and ensure MongoDB is running.")
        # Consider raising an error or exiting
        return # Or raise ConnectionError
    container.register('mongodb_client', mongodb_client)
    print(f"MongoDBClient initialized and registered for DB: {mongo_db_name}")
    
    # Step 1: Set up core services
    # TODO: Review eat_config for any other MingiDB-specific settings
    # (e.g., cache policies, indexing options) that might need to be passed to
    # the MingiDB component or affect its behavior.
    llm_service = LLMService(
        provider=eat_config.LLM_PROVIDER,
        model=eat_config.LLM_MODEL,
        embedding_model=eat_config.LLM_EMBEDDING_MODEL,
        use_cache=eat_config.LLM_USE_CACHE,
        mongodb_client=mongodb_client,
        container=container
    )
    container.register('llm_service', llm_service)
    
    # smart_library = SmartLibrary(llm_service=llm_service, container=container)
    # container.register('smart_library', smart_library)
    
    # Create firmware for component creation
    from evolving_agents.firmware.firmware import Firmware
    firmware = Firmware()
    container.register('firmware', firmware)
    # TODO: Initialize and register MingiDB memory component here
    # Example (replace with actual MingiDB class and parameters):
    # from evolving_agents.mingidb_memory.mingidb_memory import MingiDBMemory # Hypothetical import
    # mingidb_memory = MingiDBMemory(mongodb_client=mongodb_client, llm_service=llm_service, container=container)
    # container.register('memory_service', mingidb_memory) # Or appropriate name
    # await mingidb_memory.initialize()
    
    # Step 2: Create agent bus with null system agent
    # TODO: Pass the MingiDB memory component to SmartAgentBus if required
    agent_bus = SmartAgentBus(llm_service=llm_service, container=container)
    container.register('agent_bus', agent_bus)
    
    # Step 3: Create the system agent
    system_agent = await SystemAgentFactory.create_agent(container=container)
    container.register('system_agent', system_agent)
    
    # Initialize components
    # await smart_library.initialize()
    await agent_bus.initialize_from_library()

    # Step 4: Create the architect agent using the container
    architect = await create_architect_zero(container=container)

    # Step 5: Format inputs into a single context object
    formatted_inputs = json.dumps(inputs, indent=2)
    
    # Step 6: Run the full task by combining prompt and inputs
    full_prompt = (
        f"{prompt}\n\n"
        f"Here are the input contexts from various sources:\n\n"
        f"{formatted_inputs}"
    )

    result = await architect.run(full_prompt)

    # Step 7: Extract and save the result
    if hasattr(result, 'result') and hasattr(result.result, 'text'):
        result_text = result.result.text
    elif hasattr(result, 'text'):
        result_text = result.text
    else:
        result_text = str(result)

    with open(output_path, "w") as f:
        f.write(result_text)

    print(f"Smart autocomplete system completed. Suggestions saved to {output_path}")


if __name__ == "__main__":
    # Sample contexts from different sources
    contexts = [
        {
            "source": "Twitter/X",
            "timestamp": "2023-10-15T09:30:00",
            "text": "Just read about Project Nightingale at Google. Their approach to quantum computing is revolutionary. #QuantumAI #ProjectNightingale"
        },
        {
            "source": "Email",
            "timestamp": "2023-10-15T10:15:00",
            "text": "Hi team, Regarding our upcoming presentation on machine learning applications in healthcare, please review the slides I've attached. We should emphasize our competitive advantage over Acme Health's recent predictive analytics system."
        },
        {
            "source": "Slack",
            "timestamp": "2023-10-15T11:45:00",
            "text": "The meeting with Dr. Zhang went well! She's interested in our proposal for implementing federated learning across multiple hospitals while maintaining HIPAA compliance. We should follow up next week."
        },
        {
            "source": "Google Doc",
            "timestamp": "2023-10-15T14:20:00",
            "text": "Draft: Healthcare AI Strategy 2024\n\nObjectives:\n1. Deploy predictive analytics for patient readmission\n2. Implement federated learning across partner hospitals\n3. Obtain FDA approval for our diagnostic algorithm\n4. Compete effectively against Acme Health's platform"
        },
        {
            "source": "Text Message",
            "timestamp": "2023-10-15T16:05:00",
            "text": "Don't forget we need to prepare for the Nightingale demo tomorrow! Make sure the quantum simulation is working properly."
        }
    ]

    # Architect prompt
    architect_prompt = """
    Design a context-aware, adaptive autocomplete system that:

    1. Analyzes multiple text sources to build a unified user context profile
    2. Identifies key entities, terms, projects, and relationships important to the user
    3. Generates intelligent autocomplete suggestions that incorporate:
       - Domain-specific terminology and jargon
       - Project codenames and internal references
       - People, places, and organizations from the user's context
       - Recent topics the user has been engaging with
    4. Adapts suggestions based on which application the user is currently using
    5. Maintains privacy by processing all data locally
    
    The system should synthesize information across contexts to provide suggestions 
    that demonstrate true understanding rather than simple pattern matching.
    
    For example, if a user has been discussing "Project Nightingale" on Twitter 
    and later types "We need to prepare for the Night..." in a work email, 
    the system should suggest "Nightingale" and understand the context around it.
    
    Your task is to design this system and demonstrate how it would generate 
    intelligent autocomplete suggestions for a few example scenarios.
    """

    # Run the autocomplete system
    asyncio.run(run_smart_autocomplete(
        prompt=architect_prompt,
        inputs=contexts,
        output_path="autocomplete_suggestions.txt"
    ))