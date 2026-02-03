# evolving_agents/core/system_agent.py

import logging
import uuid # For fallback library name
from typing import Dict, Any, List, Optional

# BeeAI Framework imports
from beeai_framework.agents.react import ReActAgent
from beeai_framework.agents.types import AgentMeta
from beeai_framework.memory import TokenMemory, UnconstrainedMemory

# Import our specialized tools
# Standard Tools
from evolving_agents.tools.smart_library.search_component_tool import SearchComponentTool
from evolving_agents.tools.smart_library.create_component_tool import CreateComponentTool
from evolving_agents.tools.smart_library.evolve_component_tool import EvolveComponentTool
from evolving_agents.tools.smart_library.task_context_tool import TaskContextTool, ContextualSearchTool # Combined import
from evolving_agents.tools.agent_bus.register_agent_tool import RegisterAgentTool
from evolving_agents.tools.agent_bus.request_agent_tool import RequestAgentTool
from evolving_agents.tools.agent_bus.discover_agent_tool import DiscoverAgentTool

# Workflow Tools
from evolving_agents.workflow.generate_workflow_tool import GenerateWorkflowTool
from evolving_agents.workflow.process_workflow_tool import ProcessWorkflowTool

# Intent Review Tools
from evolving_agents.tools.intent_review.workflow_design_review_tool import WorkflowDesignReviewTool
from evolving_agents.tools.intent_review.component_selection_review_tool import ComponentSelectionReviewTool
from evolving_agents.tools.intent_review.approve_plan_tool import ApprovePlanTool

# Memory and Context Tools
from evolving_agents.tools.memory.experience_recorder_tool import ExperienceRecorderTool
from evolving_agents.tools.context.context_builder_tool import ContextBuilderTool

# Core Components
from evolving_agents.smart_library.smart_library import SmartLibrary
from evolving_agents.firmware.firmware import Firmware
from evolving_agents.core.llm_service import LLMService
from evolving_agents.agent_bus.smart_agent_bus import SmartAgentBus
from evolving_agents.core.dependency_container import DependencyContainer
from evolving_agents.core.base import IAgent # Ensure IAgent is imported
from evolving_agents.core.mongodb_client import MongoDBClient # For passing to tools

logger = logging.getLogger(__name__)

# Placeholder for the MemoryManagerAgent's ID.
# This will be updated when the MemoryManagerAgent is formally registered with the bus.
MEMORY_MANAGER_AGENT_ID = "memory_manager_agent_default_id"

class SystemAgentFactory:
    @staticmethod
    async def create_agent(
        # Direct passing still allowed for flexibility/testing, but container is preferred
        llm_service: Optional[LLMService] = None,
        smart_library: Optional[SmartLibrary] = None,
        agent_bus: Optional[SmartAgentBus] = None,
        mongodb_client: Optional[MongoDBClient] = None, # Added for explicit passing
        memory_type: str = "token",
        container: Optional[DependencyContainer] = None
    ) -> ReActAgent:

        # --- Resolve Dependencies ---
        logger.debug(f"SystemAgentFactory: Received container: {container is not None}")

        # Helper to resolve from container or use provided, with logging
        def _resolve_dependency(name: str, provided_instance: Optional[Any], default_factory: Optional[callable] = None):
            instance = provided_instance
            if not instance and container and container.has(name):
                instance = container.get(name)
                logger.debug(f"SystemAgentFactory: Retrieved '{name}' from container.")
            elif instance:
                logger.debug(f"SystemAgentFactory: Using directly passed '{name}'.")
            elif default_factory:
                logger.warning(f"SystemAgentFactory: '{name}' not in container or provided, creating default.")
                instance = default_factory()
                if container and instance: container.register(name, instance) # Register if created
            else:
                logger.error(f"SystemAgentFactory: Critical dependency '{name}' not found or provided, and no default factory.")
                raise ValueError(f"{name} is required but was not found or provided.")
            if instance is None: # Should be caught by the else above, but as a safeguard
                raise ValueError(f"Critical Error: {name} resolved to None unexpectedly.")
            return instance

        resolved_llm_service = _resolve_dependency("llm_service", llm_service, lambda: LLMService())
        chat_model = resolved_llm_service.chat_model
        logger.debug(f"SystemAgentFactory: Using LLMService with chat_model: {chat_model is not None}")

        # For SmartLibrary, the default factory now needs the container or llm_service
        # If container is present, SmartLibrary's __init__ can pull llm_service from it.
        # If only llm_service is passed, it uses that.
        def smart_lib_factory():
            # SmartLibrary expects llm_service if no container, or will try to get from container
            if container:
                return SmartLibrary(container=container) # This will get llm_service and mongodb_client from container
            else: # If no container, llm_service and mongodb_client must be explicitly passed
                # This branch is less ideal as it might create a new MongoDBClient if not passed explicitly
                resolved_mongo_for_lib = mongodb_client or (container.get('mongodb_client') if container and container.has('mongodb_client') else MongoDBClient())
                return SmartLibrary(llm_service=resolved_llm_service, mongodb_client=resolved_mongo_for_lib)


        resolved_smart_library = _resolve_dependency("smart_library", smart_library, smart_lib_factory)

        resolved_mongodb_client = _resolve_dependency("mongodb_client", mongodb_client, lambda: MongoDBClient())


        # For AgentBus, it also needs container or resolved dependencies
        def agent_bus_factory():
            if container:
                return SmartAgentBus(container=container) # Will get smart_library, llm_service, mongodb_client from container
            else: # Less ideal, direct dependency passing
                return SmartAgentBus(smart_library=resolved_smart_library,
                                     llm_service=resolved_llm_service,
                                     mongodb_client=resolved_mongodb_client)

        resolved_agent_bus = _resolve_dependency("agent_bus", agent_bus, agent_bus_factory)

        resolved_firmware = _resolve_dependency("firmware", None, lambda: Firmware()) # Assuming firmware doesn't have complex deps

        # --- Create Tools using RESOLVED dependencies ---
        logger.debug("SystemAgentFactory: Instantiating tools...")
        # Standard Tools
        search_tool = SearchComponentTool(resolved_smart_library)
        create_tool = CreateComponentTool(resolved_smart_library, resolved_llm_service, resolved_firmware)
        evolve_tool = EvolveComponentTool(resolved_smart_library, resolved_llm_service, resolved_firmware)

        # Agent Bus Tools
        register_tool = RegisterAgentTool(resolved_agent_bus)
        request_tool = RequestAgentTool(resolved_agent_bus)
        discover_tool = DiscoverAgentTool(resolved_agent_bus) # This tool now uses MongoDB via AgentBus

        # Workflow Tools
        generate_workflow_tool = GenerateWorkflowTool(resolved_llm_service, resolved_smart_library)
        # **MODIFIED: Pass mongodb_client or container to ProcessWorkflowTool**
        process_workflow_tool = ProcessWorkflowTool(mongodb_client=resolved_mongodb_client, container=container)

        # Task context tools
        task_context_tool = TaskContextTool(resolved_llm_service)
        contextual_search_tool = ContextualSearchTool(task_context_tool, search_tool)

        # Intent review tools
        workflow_design_review_tool = WorkflowDesignReviewTool() # Assumes no complex deps for now
        component_selection_review_tool = ComponentSelectionReviewTool() # Assumes no complex deps
        # **MODIFIED: Pass mongodb_client or container to ApprovePlanTool**
        approve_plan_tool = ApprovePlanTool(llm_service=resolved_llm_service, mongodb_client=resolved_mongodb_client, container=container)

        # Memory and Context Tools
        experience_recorder_tool = ExperienceRecorderTool(
            agent_bus=resolved_agent_bus,
            memory_manager_agent_id=MEMORY_MANAGER_AGENT_ID
        )
        context_builder_tool = ContextBuilderTool(
            agent_bus=resolved_agent_bus,
            smart_library=resolved_smart_library,
            memory_manager_agent_id=MEMORY_MANAGER_AGENT_ID,
            llm_service=resolved_llm_service
        )

        tools = [
            contextual_search_tool, task_context_tool, search_tool, create_tool, evolve_tool,
            register_tool, request_tool, discover_tool,
            generate_workflow_tool, process_workflow_tool,
            workflow_design_review_tool, component_selection_review_tool, approve_plan_tool,
            experience_recorder_tool, context_builder_tool, # Added new tools
        ]
        logger.debug(f"SystemAgentFactory: {len(tools)} tools instantiated.")

        # --- Agent Meta ---
        meta = AgentMeta(
            name="SystemAgent",
            description=(
                "I am the System Agent, the central orchestrator for the agent ecosystem. "
                "My primary purpose is to help you reuse, evolve, and create agents and tools "
                "to solve your problems efficiently. I leverage a Smart Memory system to learn from "
                "past experiences and build rich, task-relevant context. I find the most relevant "
                "components by deeply understanding the specific task context you're working in, "
                "often using the ContextBuilderTool to gather historical data and library components. "
                "After significant tasks, I use the ExperienceRecorderTool to log outcomes for future learning. "
                "I can also operate in a human-in-the-loop workflow where my plans are reviewed "
                "before execution to ensure safety and appropriateness."
            ),
            extra_description=(
                "When faced with a complex task, I first consider if similar tasks have been done before by using the "
                "ContextBuilderTool to retrieve relevant experiences and summarize message histories. This tool also helps me "
                "find suitable components from the SmartLibrary. This enriched context informs my planning, whether it's "
                "designing a workflow, selecting components, or delegating to other agents. "
                "After completing a significant workflow or achieving a key objective, I should remember to use the "
                "ExperienceRecorderTool to save the process and outcome. This helps the entire system learn and improve. "
                "My goal is to deliver the final result effectively and learn from each interaction."
            ),
            tools=tools
        )

        # --- Memory and Agent Creation ---
        memory = UnconstrainedMemory() if memory_type == "unconstrained" else TokenMemory(chat_model)
        system_agent = ReActAgent(
            llm=chat_model,
            tools=tools,
            memory=memory,
            meta=meta
        )

        # --- Tool Mapping ---
        system_agent.tools_map = {tool_instance.name: tool_instance for tool_instance in tools}
        # Log the actual mapped tools for verification
        logger.debug(f"SystemAgent tools_map contains: {list(system_agent.tools_map.keys())}")


        # --- Container Registration & AgentBus Update ---
        if container and not container.has('system_agent'):
            container.register('system_agent', system_agent)
            logger.debug("SystemAgentFactory: Registered SystemAgent instance in container.")

        # Ensure AgentBus knows about the SystemAgent instance for potential internal calls or context
        # This uses a direct assignment to a property as discussed.
        if resolved_agent_bus: # Check if agent_bus was successfully resolved
            if resolved_agent_bus._system_agent_instance is None:
                resolved_agent_bus._system_agent_instance = system_agent
                logger.debug("SystemAgentFactory: Set SystemAgent instance on the resolved AgentBus.")
            elif resolved_agent_bus._system_agent_instance is not system_agent:
                logger.warning("SystemAgentFactory: AgentBus already had a different SystemAgent instance. Overwriting.")
                resolved_agent_bus._system_agent_instance = system_agent
        else:
            logger.error("SystemAgentFactory: resolved_agent_bus is None, cannot set system_agent property on it.")


        logger.info("SystemAgent created successfully with updated tool initializations.")
        return system_agent