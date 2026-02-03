# evolving_agents/agents/memory_manager_agent.py
import logging
from typing import List, Dict, Optional, Any

from beeai_framework.agents.react import ReActAgent
from beeai_framework.agents.types import AgentMeta
from beeai_framework.memory import UnconstrainedMemory, TokenMemory
from beeai_framework.backend.chat import ChatModel

from evolving_agents.core.llm_service import LLMService
from evolving_agents.tools.internal.mongo_experience_store_tool import MongoExperienceStoreTool
from evolving_agents.tools.internal.semantic_experience_search_tool import SemanticExperienceSearchTool
from evolving_agents.tools.internal.message_summarization_tool import MessageSummarizationTool

logger = logging.getLogger(__name__)

class MemoryManagerAgent(ReActAgent):
    def __init__(
        self,
        llm_service: LLMService,
        mongo_experience_store_tool: MongoExperienceStoreTool,
        semantic_search_tool: SemanticExperienceSearchTool,
        message_summarization_tool: MessageSummarizationTool,
        agent_meta_override: Optional[AgentMeta] = None,
        memory_override: Optional[Any] = None,
    ):
        if llm_service is None:
            raise ValueError("LLMService instance is required for MemoryManagerAgent.")
        if llm_service.chat_model is None:
            raise ValueError("LLMService's chat_model is not initialized.")

        internal_tools = [
            mongo_experience_store_tool,
            semantic_search_tool,
            message_summarization_tool,
        ]

        if agent_meta_override:
            final_agent_meta = agent_meta_override
            # Ensure tools are correctly set in the meta object
            # This logic ensures our internal tools are present if an override is given
            # but might be simplified if AgentMeta's tools attribute is a mutable list.
            current_meta_tools = list(getattr(final_agent_meta, 'tools', []) or [])
            tools_to_add = [tool for tool in internal_tools if tool not in current_meta_tools]
            if tools_to_add:
                if hasattr(final_agent_meta, 'tools') and isinstance(final_agent_meta.tools, list):
                    final_agent_meta.tools.extend(tools_to_add)
                else: # If tools attribute doesn't exist or isn't a list, overwrite/create it.
                    final_agent_meta.tools = current_meta_tools + tools_to_add
                logger.debug(f"Extended/Set tools in agent_meta_override for MemoryManagerAgent.")
        else:
            final_agent_meta = AgentMeta(
                name="MemoryManagerAgent", # Explicitly set name here
                description=(
                    "Manages persistent storage and retrieval of agent experiences and "
                    "contextual facts. It uses internal tools for storing, searching, and summarizing."
                ),
                extra_description=(
                    "This agent is called by other agents (like SystemAgent) "
                    "via the SmartAgentBus. Tasks should be natural language requests "
                    "specifying the memory operation."
                ),
                tools=internal_tools,
            )

        final_memory = memory_override
        if final_memory is None:
            final_memory = TokenMemory(llm_service.chat_model)

        # Call ReActAgent's __init__
        # Ensure the parameters match what ReActAgent from beeai-framework==0.1.4 expects.
        # Common parameters are llm, tools, memory, meta.
        super().__init__(
            llm=llm_service.chat_model,
            tools=final_agent_meta.tools, # Pass the tools list
            memory=final_memory,
            meta=final_agent_meta         # Pass the AgentMeta object
        )

        # After super().__init__(), ReActAgent should have populated self.name, self.description, etc.
        # from the 'meta' object. If self.name is still not available, it indicates a deeper issue
        # with ReActAgent's initialization or an incorrect assumption about its behavior.

        # The logger line that caused the error:
        # logger.info(f"MemoryManagerAgent '{self.name}' initialized with LLM: {type(self.llm).__name__}, Memory: {type(self.memory).__name__}, Tools: {[t.name for t in self.tools_map.values()]}")
        # Let's make the logging more robust to potential missing attributes for now
        
        agent_name_for_log = getattr(self, 'name', 'UnknownName(MMA)')
        llm_type_for_log = type(getattr(self, 'llm', None)).__name__
        memory_type_for_log = type(getattr(self, 'memory', None)).__name__
        tools_map_for_log = getattr(self, 'tools_map', {})
        tool_names_for_log = [t.name for t in tools_map_for_log.values() if hasattr(t, 'name')]

        logger.info(f"MemoryManagerAgent '{agent_name_for_log}' initialized with LLM: {llm_type_for_log}, Memory: {memory_type_for_log}, Tools: {tool_names_for_log}")


    async def run(self, task_description: str) -> Any:
        logger.info(f"MemoryManagerAgent '{getattr(self, 'name', 'UnnamedMMA')}' received task: {task_description[:200]}...")
        try:
            result_message = await super().run(prompt=task_description) 
            final_result = result_message.result.text if hasattr(result_message, 'result') and hasattr(result_message.result, 'text') else str(result_message)
            logger.info(f"MemoryManagerAgent '{getattr(self, 'name', 'UnnamedMMA')}' completed task. Final result snippet: {final_result[:200]}...")
            return final_result
        except Exception as e:
            logger.error(f"Error during MemoryManagerAgent '{getattr(self, 'name', 'UnnamedMMA')}' execution for task '{task_description[:100]}...': {e}", exc_info=True)
            raise