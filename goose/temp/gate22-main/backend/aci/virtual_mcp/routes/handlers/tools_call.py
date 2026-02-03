from mcp import types as mcp_types
from sqlalchemy.orm import Session

from aci.common.db import crud
from aci.common.logging_setup import get_logger
from aci.common.schemas.virtual_mcp import VirtualMCPAuthTokenData
from aci.virtual_mcp.executors import get_tool_executor
from aci.virtual_mcp.routes.jsonrpc import (
    JSONRPCErrorCode,
    JSONRPCErrorResponse,
    JSONRPCSuccessResponse,
    JSONRPCToolsCallRequest,
)

logger = get_logger(__name__)


async def handle_tools_call(
    request: JSONRPCToolsCallRequest,
    db_session: Session,
    server_name: str,
    auth_token_data: VirtualMCPAuthTokenData | None,
) -> JSONRPCSuccessResponse | JSONRPCErrorResponse:
    """
    Handle the tools/call request for a Virtual MCP server.
    """
    vms = crud.virtual_mcp.servers.get_server(
        db_session, server_name, throw_error_if_not_found=False
    )
    if vms is None:
        logger.error(f"MCP server '{server_name}' not found")
        return JSONRPCErrorResponse(
            id=request.id,
            error=JSONRPCErrorResponse.ErrorData(
                code=JSONRPCErrorCode.INVALID_REQUEST,
                message=f"MCP server '{server_name}' not found",
            ),
        )
    tool = crud.virtual_mcp.tools.get_tool(
        db_session, f"{server_name}__{request.params.name}", throw_error_if_not_found=False
    )
    if tool is None:
        logger.error(f"Tool {request.params.name} not found")
        return JSONRPCErrorResponse(
            id=request.id,
            error=JSONRPCErrorResponse.ErrorData(
                code=JSONRPCErrorCode.INVALID_REQUEST,
                message=f"Tool {request.params.name} not found",
            ),
        )

    result = get_tool_executor(tool).execute(tool, request.params.arguments, auth_token_data)
    if isinstance(result, mcp_types.ErrorData):
        return JSONRPCErrorResponse(
            id=request.id,
            error=JSONRPCErrorResponse.ErrorData(
                code=result.code,
                message=result.message,
                data=result.data,
            ),
        )
    else:
        return JSONRPCSuccessResponse(
            id=request.id,
            result=result.model_dump(exclude_none=True),
        )
