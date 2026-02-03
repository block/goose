# evolving_agents/tools/smart_library/task_context_tool.py

from typing import Dict, Any, Optional, List
from pydantic import BaseModel, Field
import json

from beeai_framework.tools.tool import Tool, StringToolOutput
from beeai_framework.context import RunContext
from beeai_framework.emitter.emitter import Emitter

from evolving_agents.core.llm_service import LLMService

class TaskContextInput(BaseModel):
    """Input schema for the TaskContextTool."""
    task_description: str = Field(description="Brief description of the current task")
    content_query: str = Field(description="The content query that will be used for search")
    enhance_context: bool = Field(True, description="Whether to enhance the task context using LLM")
    save_to_context: bool = Field(True, description="Whether to save this task context for future use")

class TaskContextTool(Tool[TaskContextInput, None, StringToolOutput]):
    """
    Tool for generating and managing task-specific context descriptions.
    This tool helps formulate detailed task contexts to improve the relevance 
    of component search results when using the dual embedding strategy.
    """
    name = "TaskContextTool"
    description = "Generate and manage task-specific context descriptions for improved component search relevance"
    input_schema = TaskContextInput
    
    def __init__(self, llm_service: LLMService, options: Optional[Dict[str, Any]] = None):
        super().__init__(options=options or {})
        self.llm = llm_service
        self._current_task_context = ""
    
    def _create_emitter(self) -> Emitter:
        return Emitter.root().child(
            namespace=["tool", "library", "task_context"],
            creator=self,
        )
    
    async def _run(self, input: TaskContextInput, options: Optional[Dict[str, Any]] = None, context: Optional[RunContext] = None) -> StringToolOutput:
        """
        Generate a detailed task context description to improve search relevance.
        
        Args:
            input: The task context parameters
            
        Returns:
            Detailed task context description
        """
        try:
            task_context = input.task_description
            
            # Enhance the task context if requested
            if input.enhance_context:
                task_prompt = f"""
                Based on this task description: "{task_context}"
                And this content query: "{input.content_query}"
                
                Generate a detailed task context that describes:
                1. The specific use case and implementation scenario
                2. Technical requirements and constraints relevant to the task
                3. Integration points with other systems or components
                4. Expected behavior or functionality needed
                5. Specific domain knowledge that might be relevant
                
                Focus on aspects that would help distinguish between similar components based on their suitability for this specific task.
                Make your description substantive (100-200 words) and technically precise.
                
                Task context:
                """
                
                enhanced_task_context = await self.llm.generate(task_prompt)
                task_context = enhanced_task_context.strip()
            
            # Save the task context if requested
            if input.save_to_context:
                self._current_task_context = task_context
            
            # Return the resulting task context
            response = {
                "status": "success",
                "original_description": input.task_description,
                "enhanced_context": task_context,
                "saved_to_context": input.save_to_context,
                "usage_guide": "Use this task context with SearchComponentTool for more relevant results."
            }
            
            return StringToolOutput(json.dumps(response, indent=2))
            
        except Exception as e:
            import traceback
            return StringToolOutput(json.dumps({
                "status": "error",
                "message": f"Error generating task context: {str(e)}",
                "details": traceback.format_exc()
            }, indent=2))
    
    def get_current_task_context(self) -> str:
        """Get the current task context."""
        return self._current_task_context

class ContextualSearchInput(BaseModel):
    """Input schema for the ContextualSearchTool."""
    query: str = Field(description="Content query to search for")
    task_description: Optional[str] = Field(None, description="Description of the current task (or will use saved task context)")
    record_type: Optional[str] = Field(None, description="Type of record to search for (AGENT or TOOL)")
    domain: Optional[str] = Field(None, description="Domain to search within")
    limit: int = Field(5, description="Maximum number of results to return")
    threshold: float = Field(0.0, description="Minimum similarity threshold (0.0 to 1.0)")
    task_weight: float = Field(0.6, description="Weight for task relevance in the final score (0.0-1.0)")

class ContextualSearchTool(Tool[ContextualSearchInput, None, StringToolOutput]):
    """
    Tool that combines task context generation and semantic search with task relevance.
    This tool integrates TaskContextTool and SearchComponentTool to provide a unified
    interface for task-aware component search.
    """
    name = "ContextualSearchTool"
    description = "Search for components with automatic task context generation for improved relevance"
    input_schema = ContextualSearchInput
    
    def __init__(
        self, 
        task_context_tool: TaskContextTool,
        search_component_tool: Tool,
        options: Optional[Dict[str, Any]] = None
    ):
        super().__init__(options=options or {})
        self.task_context_tool = task_context_tool
        self.search_tool = search_component_tool
    
    def _create_emitter(self) -> Emitter:
        return Emitter.root().child(
            namespace=["tool", "library", "contextual_search"],
            creator=self,
        )
    
    async def _run(self, input: ContextualSearchInput, options: Optional[Dict[str, Any]] = None, context: Optional[RunContext] = None) -> StringToolOutput:
        """
        Perform a contextual search with task-aware relevance.
        
        Args:
            input: The search parameters with task description
            
        Returns:
            Search results enhanced with task relevance
        """
        try:
            # Step 1: Get or generate task context
            task_context = input.task_description
            
            if not task_context:
                # Check if we have a saved task context
                saved_context = self.task_context_tool.get_current_task_context()
                if saved_context:
                    task_context = saved_context
                else:
                    # Create a generic task context since none was provided
                    task_context = f"Finding components related to {input.query} for general use"
            
            # Step 2: Enhance the task context if needed
            if len(task_context) < 50:  # If task context is too brief
                task_input = TaskContextInput(
                    task_description=task_context,
                    content_query=input.query,
                    enhance_context=True,
                    save_to_context=True
                )
                
                task_result = await self.task_context_tool._run(task_input)
                try:
                    task_data = json.loads(task_result.get_text_content())
                    task_context = task_data.get("enhanced_context", task_context)
                except json.JSONDecodeError:
                    pass  # Keep original task_context if parsing fails
            
            # Step 3: Perform the search with task context
            search_input = {
                "query": input.query,
                "task_context": task_context,
                "record_type": input.record_type,
                "domain": input.domain,
                "limit": input.limit,
                "threshold": input.threshold,
                "with_recommendation": True
            }
            
            # Add search tool's expected input format
            if hasattr(self.search_tool, 'input_schema'):
                search_input_model = self.search_tool.input_schema(**search_input)
                search_result = await self.search_tool._run(search_input_model)
            else:
                # Fall back to direct input if no schema available
                search_result = await self.search_tool._run(search_input)
            
            # Step 4: Parse and enhance the search results with task context info
            result_text = search_result.get_text_content()
            try:
                result_data = json.loads(result_text)
                result_data["task_context_used"] = task_context
                result_data["task_weight"] = input.task_weight
                return StringToolOutput(json.dumps(result_data, indent=2))
            except json.JSONDecodeError:
                # If parsing fails, return the original result
                return search_result
            
        except Exception as e:
            import traceback
            return StringToolOutput(json.dumps({
                "status": "error",
                "message": f"Error in contextual search: {str(e)}",
                "details": traceback.format_exc()
            }, indent=2))