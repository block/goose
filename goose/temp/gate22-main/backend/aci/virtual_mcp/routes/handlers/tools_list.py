from sqlalchemy.orm import Session

from aci.common.db import crud
from aci.common.logging_setup import get_logger
from aci.virtual_mcp.routes.jsonrpc import (
    JSONRPCErrorCode,
    JSONRPCErrorResponse,
    JSONRPCSuccessResponse,
    JSONRPCToolsListRequest,
)
from aci.virtual_mcp.utils import format_tool_schema

logger = get_logger(__name__)


async def handle_tools_list(
    request: JSONRPCToolsListRequest,
    db_session: Session,
    server_name: str,
) -> JSONRPCSuccessResponse | JSONRPCErrorResponse:
    """
    Handle the tools/list request for a MCP server.
    """
    vms = crud.virtual_mcp.servers.get_server(
        db_session, server_name, throw_error_if_not_found=False
    )
    if vms is None:
        return JSONRPCErrorResponse(
            id=request.id,
            error=JSONRPCErrorResponse.ErrorData(
                code=JSONRPCErrorCode.INVALID_REQUEST,
                message=f"MCP server '{server_name}' not found",
            ),
        )

    # Format tools for response
    tool_schemas = []
    for tool in vms.tools:
        tool_schemas.append(format_tool_schema(tool))

    logger.info(f"Returning {len(tool_schemas)} tools for MCP server '{server_name}'")

    return JSONRPCSuccessResponse(
        id=request.id,
        result={
            "tools": tool_schemas,
        },
    )
