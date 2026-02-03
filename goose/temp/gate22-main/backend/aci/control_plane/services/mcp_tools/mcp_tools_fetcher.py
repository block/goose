"""
MCP Tools Fetcher Service

This service provides functionality to fetch tool lists from MCP servers using the official MCP SDK.
It supports both STREAMABLE_HTTP and SSE transport types with proper authentication.
"""

from mcp import types as mcp_types
from mcp.client.session import ClientSession
from mcp.client.sse import sse_client
from mcp.client.streamable_http import streamablehttp_client

from aci.common.db.sql_models import MCPServer
from aci.common.enums import MCPServerTransportType
from aci.common.logging_setup import get_logger
from aci.common.mcp_auth_manager import MCPAuthManager
from aci.common.schemas.mcp_auth import AuthConfig, AuthCredentials

logger = get_logger(__name__)


class MCPToolsFetcher:
    """
    Service for fetching tools from MCP servers using the official MCP SDK.

    This class handles the complexity of connecting to different transport types
    (STREAMABLE_HTTP, SSE) and managing authentication credentials.
    """

    def __init__(self, timeout_seconds: int = 30) -> None:
        """
        Initialize the tools fetcher.

        Args:
            timeout_seconds: Timeout for MCP server connections
        """
        self.timeout_seconds = timeout_seconds

    async def fetch_tools(
        self,
        mcp_server: MCPServer,
        auth_config: AuthConfig,
        auth_credentials: AuthCredentials,
    ) -> list[mcp_types.Tool]:
        """
        Fetch all tools from an MCP server.

        Args:
            mcp_server: The MCP server configuration
            auth_config: Authentication configuration
            auth_credentials: Authentication credentials

        Returns:
            List of tools available from the MCP server

        Raises:
            ValueError: If the transport type is not supported
            ConnectionError: If connection to the MCP server fails
            TimeoutError: If the operation times out
        """
        logger.info(f"Fetching tools from MCP server: {mcp_server.name} ({mcp_server.url})")

        try:
            # Create auth manager
            mcp_auth_manager = MCPAuthManager(
                mcp_server=mcp_server,
                auth_config=auth_config,
                auth_credentials=auth_credentials,
            )

            # Fetch tools with timeout
            tools = await self._fetch_tools_internal(mcp_server, mcp_auth_manager)

            logger.info(f"Successfully fetched {len(tools)} tools from {mcp_server.name}")
            return tools

        except TimeoutError:
            logger.error(
                f"Timeout fetching tools from {mcp_server.name} after {self.timeout_seconds}s"
            )
            raise TimeoutError(f"Timeout connecting to MCP server {mcp_server.name}") from None
        except Exception as e:
            logger.error(f"Error fetching tools from {mcp_server.name}: {e!s}")
            raise ConnectionError(f"Failed to fetch tools from {mcp_server.name}: {e!s}") from e

    async def _fetch_tools_internal(
        self,
        mcp_server: MCPServer,
        mcp_auth_manager: MCPAuthManager,
    ) -> list[mcp_types.Tool]:
        """
        Internal method to fetch tools based on transport type.

        Args:
            mcp_server: The MCP server configuration
            mcp_auth_manager: Authentication manager

        Returns:
            List of tools from the MCP server
        """
        match mcp_server.transport_type:
            case MCPServerTransportType.STREAMABLE_HTTP:
                return await self._fetch_tools_streamable_http(mcp_server.url, mcp_auth_manager)
            case MCPServerTransportType.SSE:
                return await self._fetch_tools_sse(mcp_server.url, mcp_auth_manager)

    async def _fetch_tools_streamable_http(
        self,
        url: str,
        mcp_auth_manager: MCPAuthManager,
    ) -> list[mcp_types.Tool]:
        """
        Fetch tools using STREAMABLE_HTTP transport.

        Args:
            url: MCP server URL
            mcp_auth_manager: Authentication manager

        Returns:
            List of tools from the MCP server
        """
        logger.debug(f"Connecting to MCP server via STREAMABLE_HTTP: {url}")

        async with streamablehttp_client(
            url, auth=mcp_auth_manager, timeout=self.timeout_seconds
        ) as (read, write, _):
            async with ClientSession(read, write) as session:
                return await self._gather_tools(session)

    async def _fetch_tools_sse(
        self,
        url: str,
        mcp_auth_manager: MCPAuthManager,
    ) -> list[mcp_types.Tool]:
        """
        Fetch tools using SSE transport.

        Args:
            url: MCP server URL
            mcp_auth_manager: Authentication manager

        Returns:
            List of tools from the MCP server
        """
        logger.debug(f"Connecting to MCP server via SSE: {url}")

        async with sse_client(url, auth=mcp_auth_manager, timeout=self.timeout_seconds) as (
            read,
            write,
        ):
            async with ClientSession(read, write) as session:
                return await self._gather_tools(session)

    async def _gather_tools(self, session: ClientSession) -> list[mcp_types.Tool]:
        """
        Gather all tools from an MCP session, handling pagination.

        Args:
            session: MCP client session

        Returns:
            List of all tools from the server
        """
        logger.debug("Initializing MCP session")
        await session.initialize()

        all_tools: list[mcp_types.Tool] = []
        next_cursor: mcp_types.Cursor | None = None
        page_count = 0

        while True:
            page_count += 1
            logger.debug(f"Fetching tools page {page_count}, cursor: {next_cursor}")

            # Fetch tools for this page
            list_tools_result = await session.list_tools(cursor=next_cursor)

            # Add tools from this page
            page_tools = list_tools_result.tools
            all_tools.extend(page_tools)

            logger.debug(f"Page {page_count}: found {len(page_tools)} tools")

            # Check if there are more pages
            next_cursor = list_tools_result.nextCursor
            if next_cursor is None:
                break

        logger.debug(f"Completed pagination: {page_count} pages, {len(all_tools)} total tools")
        return all_tools
