from typing import Annotated

from fastapi import APIRouter, Depends, HTTPException

from aci.common.db import crud
from aci.common.logging_setup import get_logger
from aci.common.schemas.mcp_tool import MCPToolPublic
from aci.control_plane import dependencies as deps

logger = get_logger(__name__)
router = APIRouter()


@router.get("/{mcp_tool_name}", response_model=MCPToolPublic, response_model_exclude_none=True)
async def get_mcp_tool(
    context: Annotated[deps.RequestContext, Depends(deps.get_request_context)],
    mcp_tool_name: str,
) -> MCPToolPublic:
    mcp_tool = crud.mcp_tools.get_mcp_tool_by_name(
        context.db_session, mcp_tool_name, throw_error_if_not_found=False
    )
    if mcp_tool is None:
        raise HTTPException(status_code=404, detail="MCP tool not found")

    return MCPToolPublic.model_validate(mcp_tool, from_attributes=True)
