"""Core adapter that wraps generated client"""

import logging
from typing import Any, Dict, Optional

from ._generated._generated import ApiClient, Configuration
from ._generated._generated.api import (
    ContextManagementApi,
    RecipeManagementApi,
    ScheduleApi,
    SessionManagementApi,
    SuperRoutesAgentApi,
    SuperRoutesConfigManagementApi,
    SuperRoutesHealthApi,
    SuperRoutesReplyApi,
)

logger = logging.getLogger(__name__)


class BaseClient:
    """Base client that sets up generated client properly"""

    def __init__(self, api_key: str, base_url: str = "http://localhost:3000"):
        """Initialize the base client.

        Args:
            api_key: The API key for authentication
            base_url: The base URL for the Goose server
        """
        self.api_key = api_key
        self.base_url = base_url

        # Setup generated client
        self.config = Configuration()
        self.config.host = base_url
        self.api_client = ApiClient(configuration=self.config)

        # Set authentication header
        self.api_client.default_headers["X-Secret-Key"] = api_key

        # Initialize API endpoints
        self._sessions_api = SessionManagementApi(self.api_client)
        self._agent_api = SuperRoutesAgentApi(self.api_client)
        self._health_api = SuperRoutesHealthApi(self.api_client)
        self._config_api = SuperRoutesConfigManagementApi(self.api_client)
        self._reply_api = SuperRoutesReplyApi(self.api_client)
        self._context_api = ContextManagementApi(self.api_client)
        self._recipe_api = RecipeManagementApi(self.api_client)
        self._schedule_api = ScheduleApi(self.api_client)

        logger.info(f"Initialized BaseClient with base_url={base_url}")

    def health_check(self) -> bool:
        """Check if server is healthy.

        Returns:
            True if server is healthy, False otherwise
        """
        try:
            response = self._health_api.status()
            # The response should be a string "ok"
            is_healthy = response == "ok"
            logger.debug(f"Health check result: {is_healthy}")
            return is_healthy
        except Exception as e:
            logger.error(f"Health check failed: {e}")
            return False

    def get_server_info(self) -> Optional[Dict[str, Any]]:
        """Get server information.

        Returns:
            Server information dict or None if error
        """
        try:
            # Try to get configuration or extensions as a proxy for server info
            response = self._config_api.get_extensions()
            return {"extensions": response.extensions if hasattr(response, "extensions") else []}
        except Exception as e:
            logger.error(f"Failed to get server info: {e}")
            return None

    def close(self) -> None:
        """Close the API client connection."""
        if hasattr(self.api_client, "close"):
            self.api_client.close()
        logger.debug("Closed API client connection")

    def __enter__(self):
        """Context manager entry."""
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        """Context manager exit."""
        self.close()
