from mcp import types as mcp_types

from aci.common.logging_setup import get_logger
from aci.mcp.routes.handlers.tools.execute_tool import EXECUTE_TOOL
from aci.mcp.routes.handlers.tools.search_tools import SEARCH_TOOLS
from aci.mcp.routes.jsonrpc import (
    JSONRPCSuccessResponse,
    JSONRPCToolsListRequest,
)

logger = get_logger(__name__)


async def handle_tools_list(
    payload: JSONRPCToolsListRequest,
) -> JSONRPCSuccessResponse:
    """
    Handle the tools/list request for a MCP server.
    """

    return JSONRPCSuccessResponse(
        id=payload.id,
        result=mcp_types.ListToolsResult(
            tools=[
                SEARCH_TOOLS,
                EXECUTE_TOOL,
            ],
        ).model_dump(exclude_none=True),
    )
