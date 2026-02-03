# evolving_agents/tools/context/context_builder_tool.py
import logging
import json
from datetime import datetime, timezone
from typing import List, Dict, Optional, Any, Type # Added Type for schema

from pydantic import BaseModel, Field

from beeai_framework.tools.tool import Tool, StringToolOutput # Correct import
from beeai_framework.emitter.emitter import Emitter # Required for Tool
from beeai_framework.context import RunContext # Required for Tool._run signature

from evolving_agents.agent_bus.smart_agent_bus import SmartAgentBus
from evolving_agents.smart_library.smart_library import SmartLibrary
from evolving_agents.core.llm_service import LLMService
# SmartContext object itself is usually constructed by SystemAgent *after* this tool returns data
# from evolving_agents.core.smart_context import SmartContext, Message 

# Configure logging
logger = logging.getLogger(__name__)

class ContextBuilderInput(BaseModel):
    """
    Input schema for the ContextBuilderTool.
    """
    target_agent_id: str = Field(description="ID or description of the agent for whom the context is being built (often 'SystemAgent' or a specific sub-agent).")
    sub_task_description: str = Field(..., description="The specific sub-task or goal for which context needs to be built.")
    current_messages: Optional[List[Dict[str, Any]]] = Field(default_factory=list, description="Recent message history relevant to the sub-task. Each message should be a dict with 'sender' and 'content'.")
    
    max_relevant_experiences: int = Field(default=3, gt=0, le=10, description="Maximum number of relevant past experiences to retrieve from Smart Memory.")
    max_messages_to_summarize: int = Field(default=10, gt=0, le=50, description="Maximum number of recent messages from current_messages to consider for summarization.")
    
    # Flags to control what context components are built
    include_message_summary: bool = Field(default=True, description="Whether to generate and include a summary of current_messages.")
    include_relevant_experiences: bool = Field(default=True, description="Whether to retrieve and include relevant past experiences from Smart Memory.")
    include_library_components: bool = Field(default=True, description="Whether to search for and include relevant components from SmartLibrary.")
    
    # Parameters for SmartLibrary search if include_library_components is true
    library_search_limit: int = Field(default=3, gt=0, le=10, description="Max number of library components to retrieve.")
    library_search_threshold: float = Field(default=0.5, ge=0.0, le=1.0, description="Minimum similarity threshold for library component search.")

class BuiltContextData(BaseModel): # Output schema for the tool
    current_task_for_context: str
    retrieved_experiences: Optional[List[Dict[str, Any]]] = None
    message_history_summary: Optional[str] = None
    relevant_library_components: Optional[List[Dict[str, Any]]] = None
    errors: Optional[List[str]] = None
    tool_metadata: Dict[str, Any]

class ContextBuilderTool(Tool[ContextBuilderInput, None, BuiltContextData]): # Input and Output Pydantic models
    """
    Dynamically constructs a rich contextual dataset for a sub-task by
    querying Smart Memory for relevant past experiences, summarizing message history,
    and finding relevant library components. This data is then used by SystemAgent
    to create an optimized SmartContext instance.
    """
    name: str = "ContextBuilderTool"
    description: str = (
        "Builds rich context for a given sub-task by fetching relevant past experiences, "
        "message summaries, and library components. Essential for informed planning by SystemAgent."
    )
    input_schema: Type[BaseModel] = ContextBuilderInput
    output_schema: Type[BaseModel] = BuiltContextData

    def __init__(
        self,
        agent_bus: SmartAgentBus,
        smart_library: SmartLibrary,
        memory_manager_agent_id: str,
        llm_service: LLMService, # LLMService is used here for SmartLibrary search
        options: Optional[Dict[str, Any]] = None # For Tool base class
    ):
        super().__init__(options=options) # Call Tool's __init__
        if agent_bus is None: raise ValueError("agent_bus cannot be None for ContextBuilderTool")
        if smart_library is None: raise ValueError("smart_library cannot be None for ContextBuilderTool")
        if not memory_manager_agent_id: raise ValueError("memory_manager_agent_id must be provided.")
        if llm_service is None: raise ValueError("llm_service cannot be None for ContextBuilderTool (used for library search).")

        self.agent_bus = agent_bus
        self.smart_library = smart_library
        self.memory_manager_agent_id = memory_manager_agent_id
        self.llm_service = llm_service

    def _create_emitter(self) -> Emitter: # Implement required method
        return Emitter.root().child(
            namespace=["tool", "context", "builder"],
            creator=self,
        )

    async def _request_mma_capability(
        self,
        task_description_for_mma: str,
        # timeout: Optional[int] = 60 # Timeout would be handled by SmartAgentBus.request_capability if supported
    ) -> Any:
        """
        Helper method to send a task to the MemoryManagerAgent via the SmartAgentBus.
        """
        bus_payload = {"task_description": task_description_for_mma}
        bus_capability_name = "process_task" # Generic capability for MemoryManagerAgent (ReAct)

        logger.debug(f"ContextBuilderTool: Requesting MMA ({self.memory_manager_agent_id}) for task: {task_description_for_mma[:150]}...")
        try:
            response = await self.agent_bus.request_capability(
                capability=bus_capability_name,
                content=bus_payload, 
                specific_agent_id=self.memory_manager_agent_id,
                timeout=180 # Increased timeout for potentially complex MMA tasks
            )
            logger.debug(f"ContextBuilderTool: MMA response received (type: {type(response)}): {str(response)[:200]}...")
            return response
        except Exception as e:
            logger.error(f"ContextBuilderTool: Error requesting MMA capability for task '{task_description_for_mma[:100]}...': {e}", exc_info=True)
            return {"error": f"Failed to communicate with MemoryManagerAgent: {str(e)}"}


    async def _run(
        self, 
        input: ContextBuilderInput, # Input is an instance of ContextBuilderInput
        options: Optional[Dict[str, Any]] = None, 
        context: Optional[RunContext] = None
    ) -> BuiltContextData:
        """
        Constructs context data by querying memory and library.
        """
        built_data = BuiltContextData(
            current_task_for_context=input.sub_task_description,
            errors=[],
            tool_metadata={
                "tool_name": self.name,
                "timestamp": datetime.now(timezone.utc).isoformat(),
                "target_agent_id": input.target_agent_id,
                "builder_config": input.model_dump(exclude_none=True)
            }
        )

        # 1. Retrieve Relevant Experiences from Smart Memory
        if input.include_relevant_experiences:
            experience_task_desc = (
                f"Find and return up to {input.max_relevant_experiences} experiences that are most semantically "
                f"relevant to the following task: '{input.sub_task_description}'. For each relevant experience, "
                "include its 'experience_id', 'primary_goal_description', 'sub_task_description', 'final_outcome', "
                "and 'output_summary'."
            )
            mma_exp_response = await self._request_mma_capability(experience_task_desc)
            
            if isinstance(mma_exp_response, dict) and mma_exp_response.get("error"):
                built_data.errors.append(f"Experience retrieval error: {mma_exp_response['error']}")
            elif isinstance(mma_exp_response, list) and all(isinstance(item, dict) for item in mma_exp_response):
                built_data.retrieved_experiences = mma_exp_response
                logger.info(f"ContextBuilderTool: Added {len(mma_exp_response)} relevant experiences to context.")
            elif isinstance(mma_exp_response, dict) and "final_answer" in mma_exp_response and isinstance(mma_exp_response["final_answer"], list):
                built_data.retrieved_experiences = mma_exp_response["final_answer"]
                logger.info(f"ContextBuilderTool: Added {len(mma_exp_response['final_answer'])} relevant experiences from MMA's final_answer.")
            else:
                msg = f"Unexpected format for relevant experiences from MMA: {str(mma_exp_response)[:200]}"
                logger.warning(msg)
                built_data.errors.append(msg)

        # 2. Summarize Message History via Smart Memory
        if input.include_message_summary and input.current_messages:
            messages_to_summarize = input.current_messages[-input.max_messages_to_summarize:]
            if messages_to_summarize:
                messages_json_string = json.dumps(messages_to_summarize) # MMA expects string for ReAct processing
                summary_task_desc = (
                    f"Summarize the following message history (provided as a JSON string): {messages_json_string}. "
                    f"The summary should focus on extracting key information, decisions, and context "
                    f"relevant to the sub-task: '{input.sub_task_description}'. "
                    f"Provide a concise textual summary."
                )
                mma_summary_response = await self._request_mma_capability(summary_task_desc)

                summary_text = None
                if isinstance(mma_summary_response, dict) and mma_summary_response.get("error"):
                    built_data.errors.append(f"Message summary error: {mma_summary_response['error']}")
                elif isinstance(mma_summary_response, str):
                    summary_text = mma_summary_response
                elif isinstance(mma_summary_response, dict) and "final_answer" in mma_summary_response and isinstance(mma_summary_response["final_answer"], str):
                    summary_text = mma_summary_response["final_answer"]
                
                if summary_text and not summary_text.startswith("Error summarizing messages:"):
                    built_data.message_history_summary = summary_text
                    logger.info(f"ContextBuilderTool: Added message summary to context: {summary_text[:100]}...")
                elif summary_text: # It was an error message from the summarizer tool itself
                    msg = f"Failed to get a valid summary from MMA. MMA/Summarizer response: {summary_text}"
                    logger.warning(msg)
                    built_data.errors.append(msg)
                else: # MMA response was not a string or expected dict structure
                    msg = f"Unexpected response from MMA for summary: {str(mma_summary_response)[:200]}"
                    logger.warning(msg)
                    built_data.errors.append(msg)
        
        # 3. Query SmartLibrary for relevant components
        if input.include_library_components:
            try:
                logger.debug(f"ContextBuilderTool: Querying SmartLibrary with task context: '{input.sub_task_description}'")
                # SmartLibrary's semantic_search uses the sub_task_description as the task_context argument
                # and also as the query if no more specific query is implied by the task.
                library_results_tuples = await self.smart_library.semantic_search(
                    query=input.sub_task_description, # Use sub-task as the primary query
                    task_context=input.sub_task_description, # Also use as task context for T_raz search
                    limit=input.library_search_limit,
                    threshold=input.library_search_threshold
                )
                if library_results_tuples:
                    built_data.relevant_library_components = [
                        {
                            "id": comp_dict["id"], 
                            "name": comp_dict["name"], 
                            "record_type": comp_dict["record_type"],
                            "description": comp_dict["description"], 
                            "final_score": final_score,
                            "content_score": content_score,
                            "task_score": task_score
                        } 
                        for comp_dict, final_score, content_score, task_score in library_results_tuples
                    ]
                    logger.info(f"ContextBuilderTool: Added {len(built_data.relevant_library_components)} library components to context.")
                else:
                    logger.info("ContextBuilderTool: No relevant library components found by SmartLibrary.")
            except Exception as e:
                msg = f"ContextBuilderTool: Error querying SmartLibrary: {e}"
                logger.error(msg, exc_info=True)
                built_data.errors.append(msg)
        
        if not built_data.errors: # If there were errors, errors list won't be None
            built_data.errors = None

        return built_data

# Example (Conceptual - requires full EAT setup)
# async def main_example():
#     # ... (Setup container with LLMService, SmartLibrary, SmartAgentBus, MemoryManagerAgent) ...
#     # Assume 'container' is fully initialized and MMA is registered on the bus.
#     # context_builder_tool = ContextBuilderTool(
#     #     agent_bus=container.get('agent_bus'),
#     #     smart_library=container.get('smart_library'),
#     #     memory_manager_agent_id=MEMORY_MANAGER_AGENT_ID, # Your MMA's registered ID
#     #     llm_service=container.get('llm_service')
#     # )

#     # input_data_model = ContextBuilderInput(
#     #     target_agent_id="SystemAgent",
#     #     sub_task_description="Resolve a complex customer complaint about a faulty invoice calculation and unresponsive support.",
#     #     current_messages=[
#     #         {"sender": "customer", "content": "My invoice XYZ123 is wrong, the tax is incorrect!"},
#     #         {"sender": "support_bot_v1", "content": "I understand you have an issue with invoice XYZ123. Can you specify the problem?"},
#     #         {"sender": "customer", "content": "The subtotal is $100, tax is $10, but total says $120! And I couldn't reach anyone!"}
#     #     ],
#     #     include_relevant_experiences=True,
#     #     include_message_summary=True,
#     #     include_library_components=True
#     # )

#     # built_context_result = await context_builder_tool._run(input=input_data_model)
#     # print("\n--- Built Context Data ---")
#     # print(built_context_result.model_dump_json(indent=2))
#     pass

# if __name__ == "__main__":
#     import asyncio
#     # asyncio.run(main_example())
#     pass