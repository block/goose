from sqlalchemy.orm import Session

from aci.common.db import crud
from aci.common.db.sql_models import MCPServerBundle, MCPSession
from aci.common.logging_setup import get_logger
from aci.common.schemas.mcp_tool_call_log import MCPToolCallLogCreate
from aci.mcp.routes.handlers.tools.execute_tool import EXECUTE_TOOL, handle_execute_tool
from aci.mcp.routes.handlers.tools.search_tools import SEARCH_TOOLS, handle_search_tools
from aci.mcp.routes.jsonrpc import (
    JSONRPCErrorCode,
    JSONRPCErrorResponse,
    JSONRPCSuccessResponse,
    JSONRPCToolsCallRequest,
)

logger = get_logger(__name__)


async def handle_tools_call(
    db_session: Session,
    mcp_session: MCPSession,
    payload: JSONRPCToolsCallRequest,
    mcp_server_bundle: MCPServerBundle,
) -> JSONRPCSuccessResponse | JSONRPCErrorResponse:
    """
    Handle the tools/call request for a MCP server bundle.
    """
    match payload.params.name:
        case SEARCH_TOOLS.name:
            return await handle_search_tools(db_session, mcp_server_bundle, payload)
        case EXECUTE_TOOL.name:
            result: JSONRPCSuccessResponse | JSONRPCErrorResponse
            result, tool_call_log_create = await handle_execute_tool(
                db_session, mcp_session, mcp_server_bundle, payload
            )
            _create_tool_call_log(db_session, tool_call_log_create)
            return result
        case _:
            logger.error(f"Unknown tool: {payload.params.name}")
            return JSONRPCErrorResponse(
                id=payload.id,
                error=JSONRPCErrorResponse.ErrorData(
                    code=JSONRPCErrorCode.INVALID_METHOD_PARAMS,
                    message=f"Unknown tool: {payload.params.name}",
                ),
            )


def _create_tool_call_log(
    db_session: Session,
    tool_call_log_create: MCPToolCallLogCreate,
) -> None:
    try:
        # need addtiional validation because the model_construct is not validating the extra fields
        tool_call_log_create.model_validate(tool_call_log_create.model_dump())
    except Exception as e:
        logger.exception(f"Error validating tool call log data: {e}")

    try:
        crud.mcp_tool_call_logs.create(db_session, tool_call_log_create)
    except Exception as e:
        logger.exception(f"Error creating tool call log: {e}")
