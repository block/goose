# evolving_agents/providers/base.py

from abc import ABC, abstractmethod
from typing import Dict, Any, List, Optional

class FrameworkProvider(ABC):
    """
    Abstract base class for framework providers.
    
    Framework providers handle the integration with specific agent frameworks
    like OpenAI, BeeAI, etc. They abstract away the details of creating and
    executing agents in those frameworks.
    """
    
    @abstractmethod
    async def create_agent(
        self, 
        record: Dict[str, Any],
        tools: Optional[List[Any]] = None,
        firmware_content: Optional[str] = None,
        config: Optional[Dict[str, Any]] = None
    ) -> Any:
        """
        Create an agent with the specified configuration.
        
        Args:
            record: Agent record from the Smart Library
            tools: Optional list of tools to provide to the agent
            firmware_content: Optional firmware content to inject
            config: Optional configuration parameters
            
        Returns:
            Instantiated agent
        """
        pass
    
    @abstractmethod
    async def execute_agent(
        self, 
        agent_instance: Any,
        input_text: str,
        execution_config: Optional[Dict[str, Any]] = None
    ) -> Dict[str, Any]:
        """
        Execute an agent with input text.
        
        Args:
            agent_instance: The agent instance
            input_text: Input text to process
            execution_config: Optional execution configuration parameters
            
        Returns:
            Execution result dictionary
        """
        pass
    
    @abstractmethod
    def supports_framework(self, framework_name: str) -> bool:
        """
        Check if this provider supports the specified framework.
        
        Args:
            framework_name: Name of the framework to check
            
        Returns:
            True if supported, False otherwise
        """
        pass
    
    @abstractmethod
    def get_supported_agent_types(self) -> List[str]:
        """
        Get the agent types supported by this provider.
        
        Returns:
            List of supported agent type names
        """
        pass
    
    @abstractmethod
    def get_configuration_schema(self) -> Dict[str, Any]:
        """
        Get the schema for provider configuration.
        
        Returns:
            Dictionary with configuration options and their schema
        """
        pass