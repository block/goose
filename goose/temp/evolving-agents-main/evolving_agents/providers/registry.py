# evolving_agents/providers/registry.py

import logging
from typing import Dict, Any, List, Optional, Type

from evolving_agents.providers.base import FrameworkProvider

logger = logging.getLogger(__name__)

class ProviderRegistry:
    """
    Registry for framework providers.
    
    Manages the available providers and handles provider selection based on framework name.
    """
    
    def __init__(self):
        """Initialize an empty provider registry."""
        self.providers = {}
        logger.info("Provider Registry initialized")
    
    def register_provider(self, provider: FrameworkProvider) -> None:
        """
        Register a provider with the registry.
        
        Args:
            provider: The provider instance to register
        """
        # Use the class name as the provider ID
        provider_id = provider.__class__.__name__
        self.providers[provider_id] = provider
        logger.info(f"Registered provider: {provider_id}")
    
    def register_provider_class(
        self, 
        provider_class: Type[FrameworkProvider], 
        *args, 
        **kwargs
    ) -> None:
        """
        Register a provider class with the registry.
        
        Args:
            provider_class: The provider class to register
            *args: Arguments to pass to the provider constructor
            **kwargs: Keyword arguments to pass to the provider constructor
        """
        provider = provider_class(*args, **kwargs)
        self.register_provider(provider)
    
    def get_provider_for_framework(self, framework_name: str) -> Optional[FrameworkProvider]:
        """
        Get a provider that supports the specified framework.
        
        Args:
            framework_name: Name of the framework to get a provider for
            
        Returns:
            The first provider that supports the framework, or None if no provider is found
        """
        for provider in self.providers.values():
            if provider.supports_framework(framework_name):
                return provider
        
        logger.warning(f"No provider found for framework: {framework_name}")
        return None
    
    def get_provider_by_id(self, provider_id: str) -> Optional[FrameworkProvider]:
        """
        Get a provider by its ID.
        
        Args:
            provider_id: ID of the provider to get
            
        Returns:
            The provider with the specified ID, or None if no provider is found
        """
        return self.providers.get(provider_id)
    
    def get_all_providers(self) -> List[FrameworkProvider]:
        """
        Get all registered providers.
        
        Returns:
            List of all registered providers
        """
        return list(self.providers.values())
    
    def list_available_providers(self) -> List[str]:
        """
        Get a list of all registered provider IDs.
        
        Returns:
            List of provider IDs
        """
        return list(self.providers.keys())