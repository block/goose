# evolving_agents/providers/openai_agents_provider.py

import logging
import asyncio
from typing import Dict, Any, List, Optional, Union

from agents import Agent as OpenAIAgent, Runner, RunConfig
from agents.model_settings import ModelSettings
from agents.run_context import RunContextWrapper

from evolving_agents.providers.base import FrameworkProvider
from evolving_agents.core.llm_service import LLMService
from evolving_agents.firmware.firmware import Firmware
from evolving_agents.adapters.openai_tool_adapter import OpenAIToolAdapter
from evolving_agents.adapters.openai_guardrails_adapter import OpenAIGuardrailsAdapter

logger = logging.getLogger(__name__)

class OpenAIAgentsProvider(FrameworkProvider):
    """
    Provider for OpenAI Agents SDK integration.
    Handles creation and execution of OpenAI Agents.
    """
    
    def __init__(self, llm_service: Optional[LLMService] = None):
        """
        Initialize the OpenAI Agents provider.
        
        Args:
            llm_service: Optional LLM service to use for agents
        """
        self.llm_service = llm_service
        self.openai_agents = {}
        logger.info("OpenAI Agents Provider initialized")
    
    async def create_agent(
        self, 
        record: Dict[str, Any],
        tools: Optional[List[Any]] = None,
        firmware_content: Optional[str] = None,
        config: Optional[Dict[str, Any]] = None
    ) -> OpenAIAgent:
        """
        Create an OpenAI Agent with the specified configuration.
        
        Args:
            record: Agent record from the Smart Library
            tools: Optional list of tools to provide to the agent
            firmware_content: Optional firmware content to inject
            config: Optional configuration parameters
            
        Returns:
            Instantiated OpenAI Agent
        """
        logger.info(f"Creating OpenAI Agent '{record['name']}'")
        
        # Apply default config if none provided
        config = config or {}
        
        # Prepare instructions - combine firmware with description
        instructions = record["description"]
        if firmware_content:
            instructions = f"{firmware_content}\n\n{instructions}"
        
        # Convert tools to OpenAI's format if provided
        openai_tools = []
        if tools:
            openai_tools = [OpenAIToolAdapter.convert_evolving_tool_to_openai(tool) for tool in tools]
        
        # Extract any additional model settings from metadata
        metadata = record.get("metadata", {})
        model_settings_dict = metadata.get("model_settings", {})
        
        # Get model name from metadata or use default
        model = metadata.get("model", "gpt-4o")
        
        # Create model settings
        model_settings = ModelSettings(
            temperature=model_settings_dict.get("temperature", 0.7),
            top_p=model_settings_dict.get("top_p", 1.0),
            frequency_penalty=model_settings_dict.get("frequency_penalty", 0),
            presence_penalty=model_settings_dict.get("presence_penalty", 0),
        )
        
        # Create guardrails if needed
        input_guardrails = []
        output_guardrails = []
        
        if config.get("apply_firmware", True) and metadata.get("guardrails_enabled", True):
            # Create a firmware instance
            firmware = Firmware()
            domain = record.get("domain", "general")
            
            # Create guardrails from firmware
            guardrails_dict = OpenAIGuardrailsAdapter.convert_firmware_to_guardrails(firmware, domain)
            input_guardrails.append(guardrails_dict["input_guardrail"])
            output_guardrails.append(guardrails_dict["output_guardrail"])
        
        # Create the OpenAI Agent
        agent = OpenAIAgent(
            name=record["name"],
            instructions=instructions,
            tools=openai_tools,
            model=model,
            model_settings=model_settings,
            input_guardrails=input_guardrails,
            output_guardrails=output_guardrails
        )
        
        # Store for future reference
        self.openai_agents[record["name"]] = agent
        
        return agent
    
    async def execute_agent(
        self, 
        agent_instance: Union[OpenAIAgent, str],
        input_text: str,
        execution_config: Optional[Dict[str, Any]] = None
    ) -> Dict[str, Any]:
        """
        Execute an OpenAI Agent with input text.
        
        Args:
            agent_instance: The OpenAI Agent instance or name
            input_text: Input text to process
            execution_config: Optional execution configuration parameters
            
        Returns:
            Execution result dictionary
        """
        # Handle string agent name
        if isinstance(agent_instance, str):
            if agent_instance not in self.openai_agents:
                return {
                    "status": "error",
                    "message": f"Agent '{agent_instance}' not found",
                    "result": f"Error: Agent '{agent_instance}' not found"
                }
            agent_instance = self.openai_agents[agent_instance]
        
        logger.info(f"Executing OpenAI Agent with input: {input_text[:50]}...")
        
        try:
            # Configure execution using config parameters
            max_turns = execution_config.get("max_turns", 10) if execution_config else 10
            
            # Prepare run config if needed
            run_config = None
            if execution_config and execution_config.get("trace_metadata"):
                run_config = RunConfig(
                    trace_metadata=execution_config.get("trace_metadata"),
                    workflow_name=execution_config.get("workflow_name", "Agent workflow")
                )
            
            # Create empty context for execution
            context = {}
            
            # Execute the agent
            result = await Runner.run(
                agent_instance,
                input_text,
                context=context,
                max_turns=max_turns,
                run_config=run_config
            )
            
            return {
                "status": "success",
                "message": "OpenAI Agent executed successfully",
                "result": str(result.final_output),
                "raw_result": result  # Include raw result for advanced usage
            }
            
        except Exception as e:
            logger.error(f"Error executing OpenAI Agent: {str(e)}")
            import traceback
            logger.error(traceback.format_exc())
            
            return {
                "status": "error",
                "message": f"Error executing OpenAI Agent: {str(e)}",
                "result": f"Error: {str(e)}",
                "error": e
            }
    
    def supports_framework(self, framework_name: str) -> bool:
        """
        Check if this provider supports the specified framework.
        
        Args:
            framework_name: Name of the framework to check
            
        Returns:
            True if supported, False otherwise
        """
        return framework_name.lower() in ["openai", "openai-agents", "openai_agents"]
    
    def get_supported_agent_types(self) -> List[str]:
        """
        Get the agent types supported by this provider.
        
        Returns:
            List of supported agent type names
        """
        return ["OpenAIAgent"]
    
    def get_configuration_schema(self) -> Dict[str, Any]:
        """
        Get the schema for provider configuration.
        
        Returns:
            Dictionary with configuration options and their schema
        """
        return {
            "model": {
                "type": "string",
                "enum": ["gpt-4o", "gpt-4-turbo", "gpt-3.5-turbo"],
                "default": "gpt-4o",
                "description": "The model to use for the agent"
            },
            "model_settings": {
                "type": "object",
                "properties": {
                    "temperature": {
                        "type": "number",
                        "default": 0.7,
                        "description": "Controls randomness in the model's output"
                    },
                    "top_p": {
                        "type": "number",
                        "default": 1.0,
                        "description": "Controls diversity of the model's output"
                    }
                }
            },
            "execution": {
                "type": "object",
                "properties": {
                    "max_turns": {
                        "type": "integer",
                        "default": 10,
                        "description": "Maximum number of turns for the agent"
                    }
                }
            },
            "apply_firmware": {
                "type": "boolean",
                "default": True,
                "description": "Whether to apply firmware guardrails to the agent"
            }
        }