# evolving_agents/core/dependency_container.py

from typing import Dict, Any, Optional, List

class DependencyContainer:
    """
    A simple dependency container to manage and provide access to shared
    components and services within the application, such as database clients,
    LLM services, etc. This helps in decoupling components and managing
    their lifecycle or configuration centrally.

    It acts as a central registry where components can be stored with a unique
    name and retrieved by other components that depend on them. This is
    particularly useful for managing singleton instances of services.

    Example Usage:
    ```python
    # --- In application setup ---
    # from evolving_agents.core.mongodb_client import MongoDBClient # Your client class
    #
    # container = DependencyContainer()
    #
    # # Register MongoDBClient instance
    # mongo_client = MongoDBClient(uri="your_uri", db_name="your_db")
    # container.register("mongodb_client", mongo_client)
    #
    # # Register LLMService instance
    # llm_service = LLMService(mongodb_client=mongo_client) # LLMService might use mongo for cache
    # container.register("llm_service", llm_service)

    # --- In a component that needs MongoDB ---
    # class SmartLibrary:
    #     def __init__(self, container: DependencyContainer):
    #         self.mongodb_client = container.get("mongodb_client")
    #         self.collection = self.mongodb_client.get_collection("my_components")
    #         # ...
    ```
    """

    def __init__(self):
        self._components: Dict[str, Any] = {}
        self._initialized = False # Flag to prevent multiple initializations

    def register(self, name: str, component: Any):
        """
        Register a component instance with a unique name.

        If a component with the same name already exists, it will be overwritten.

        Args:
            name (str): The unique name to identify the component.
            component (Any): The component instance to register.
        """
        if not isinstance(name, str) or not name:
            raise ValueError("Component name must be a non-empty string.")
        self._components[name] = component
        # Optionally, log registration:
        # logger.debug(f"Component '{name}' (type: {type(component).__name__}) registered.")

    def get(self, name: str, default: Optional[Any] = Ellipsis) -> Any:
        """
        Get a registered component by its name.

        Args:
            name (str): The name of the component to retrieve.
            default (Optional[Any]): A default value to return if the component
                                     is not found. If not provided (Ellipsis),
                                     a ValueError is raised for missing components.

        Returns:
            Any: The registered component instance.

        Raises:
            ValueError: If the component is not found and no default is provided.
        """
        if name in self._components:
            return self._components[name]
        elif default is not Ellipsis:
            return default
        else:
            raise ValueError(f"Component '{name}' not registered in the DependencyContainer.")

    def has(self, name: str) -> bool:
        """
        Check if a component with the given name is registered.

        Args:
            name (str): The name of the component to check.

        Returns:
            bool: True if the component is registered, False otherwise.
        """
        return name in self._components

    async def initialize(self):
        """
        Perform any late-stage initialization of registered components.

        This method is a placeholder for scenarios where components might need
        an explicit initialization step after all dependencies are potentially
        available in the container. The container itself does not enforce or
        automatically call any specific initialization methods on registered
        components.

        The application's main setup routine would typically be responsible for
        calling this method after all core components have been registered,
        if such coordinated initialization is required.
        """
        if self._initialized:
            # logger.debug("DependencyContainer already initialized.")
            return

        # Example: If components had an 'async_initialize' method the container could call them:
        # for name, component in self._components.items():
        #     if hasattr(component, "async_initialize") and callable(component.async_initialize):
        #         logger.info(f"Initializing component '{name}' via DependencyContainer...")
        #         await component.async_initialize()

        # For the current EAT setup, components like MongoDBClient or LLMService
        # typically handle their core setup (e.g., DB connection, model loading)
        # within their own __init__ methods or when first used.
        # This initialize() method here is more of a convention or for future expansion.

        self._initialized = True
        # logger.info("DependencyContainer initialization routine complete.")

    def get_all_component_names(self) -> List[str]:
        """
        Returns a list of names of all registered components.
        Useful for debugging or inspection.
        """
        return list(self._components.keys())

    def __len__(self) -> int:
        """Returns the number of registered components."""
        return len(self._components)

    def __contains__(self, name: str) -> bool:
        """Allows checking for component existence using 'in' operator."""
        return self.has(name)