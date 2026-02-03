# evolving_agents/agents/intent_review_agent.py

import logging
from typing import Dict, Any, List, Optional, Union
from datetime import datetime

# BeeAI Framework imports
from beeai_framework.agents.react import ReActAgent
from beeai_framework.agents.types import AgentMeta
from beeai_framework.memory import TokenMemory, UnconstrainedMemory
from beeai_framework.context import RunContext
from beeai_framework.emitter.emitter import Emitter

# Import our specialized tools
from evolving_agents.tools.intent_review.workflow_design_review_tool import WorkflowDesignReviewTool
from evolving_agents.tools.intent_review.component_selection_review_tool import ComponentSelectionReviewTool
from evolving_agents.tools.intent_review.approve_plan_tool import ApprovePlanTool

# Import core components
from evolving_agents.core.llm_service import LLMService
from evolving_agents.core.dependency_container import DependencyContainer

logger = logging.getLogger(__name__)

class IntentReviewAgentInitializer:
    """
    Agent specialized in reviewing and approving workflow designs, 
    component selections, and intent plans.
    
    This agent serves as an AI-powered reviewer that:
    1. Reviews workflow designs before they're turned into executable steps
    2. Reviews component selections to ensure appropriate components are chosen
    3. Reviews intent plans to ensure they're safe and effective
    4. Provides detailed feedback on why plans are approved or rejected
    """
    
    @staticmethod
    async def create_agent(
        llm_service: Optional[LLMService] = None,
        container: Optional[DependencyContainer] = None
    ) -> ReActAgent:
        """
        Create and configure the Intent Review Agent.
        
        Args:
            llm_service: LLM service for text generation
            container: Optional dependency container for managing component dependencies
            
        Returns:
            Configured Intent Review Agent
        """
        logger.info("Initializing Intent Review Agent...")
        
        # Resolve dependencies from container if provided
        if container:
            llm_service = llm_service or container.get('llm_service')
        else:
            if not llm_service:
                raise ValueError("LLM service must be provided when not using a dependency container")
        
        # Get the chat model from LLM service
        chat_model = llm_service.chat_model
        
        # Create specialized tools for review operations
        workflow_review_tool = WorkflowDesignReviewTool()
        component_review_tool = ComponentSelectionReviewTool()
        plan_approval_tool = ApprovePlanTool(llm_service=llm_service)
        
        # Create tools list
        tools = [
            workflow_review_tool,
            component_review_tool,
            plan_approval_tool
        ]
        
        # Create agent metadata with specific review capabilities
        meta = AgentMeta(
            name="IntentReviewAgent",
            description=(
                "I am an Intent Review Agent, specialized in reviewing workflow designs, "
                "component selections, and intent plans before they are executed. "
                "I help ensure safety, effectiveness, and alignment with user goals."
            ),
            extra_description=(
                "I can review workflow designs to verify their structure and approach. "
                "I can review component selections to ensure the right tools and agents are chosen. "
                "I can review intent plans to provide a detailed assessment of each step. "
                "I provide clear reasoning for approvals and rejections."
            ),
            tools=tools
        )
        
        # Create the Intent Review Agent
        agent = ReActAgent(
            llm=chat_model,
            tools=tools,
            memory=TokenMemory(chat_model),
            meta=meta
        )
        
        # Add tools dictionary for direct access
        agent.tools_map = {
            "workflow_review": workflow_review_tool,
            "component_review": component_review_tool,
            "plan_approval": plan_approval_tool
        }
        
        # Register with container if provided
        if container and not container.has('intent_review_agent'):
            container.register('intent_review_agent', agent)
            logger.info("Intent Review Agent registered in container")
        
        logger.info("Intent Review Agent initialization complete")
        return agent