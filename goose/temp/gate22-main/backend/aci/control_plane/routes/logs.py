from datetime import UTC, datetime, timedelta
from typing import Annotated

from fastapi import APIRouter, Depends, HTTPException

from aci.common.db.crud import mcp_tool_call_logs
from aci.common.enums import OrganizationRole
from aci.common.logging_setup import get_logger
from aci.common.schemas.mcp_tool_call_log import (
    MCPToolCallLogCursor,
    MCPToolCallLogFilters,
    MCPToolCallLogResponse,
)
from aci.common.schemas.pagination import CursorPaginationParams, CursorPaginationResponse
from aci.control_plane import dependencies as deps

logger = get_logger(__name__)
router = APIRouter()


@router.get("/tool-calls", response_model=CursorPaginationResponse[MCPToolCallLogResponse])
async def get_tool_call_logs(
    context: Annotated[deps.RequestContext, Depends(deps.get_request_context)],
    pagination: Annotated[CursorPaginationParams, Depends()],
    filters: Annotated[MCPToolCallLogFilters, Depends()],
) -> CursorPaginationResponse[MCPToolCallLogResponse]:
    """
    Get paginated tool call logs with cursor-based pagination.
    Results are ordered by started_at DESC (most recent first).
    """
    logger.info(f"Getting tool call logs with filters: {filters}")
    cursor = None
    if pagination.cursor is not None:
        try:
            cursor = MCPToolCallLogCursor.decode(pagination.cursor)
        except Exception as e:
            raise HTTPException(status_code=400, detail="Invalid cursor") from e

    # set start_time to 7 days ago if not provided or provided is before 7 days ago
    # TODO: depend on pricing plan
    current_time = datetime.now(UTC)
    if filters.start_time is None or filters.start_time < current_time - timedelta(days=7):
        filters.start_time = current_time - timedelta(days=7)
    if filters.end_time is None or filters.end_time > current_time:
        filters.end_time = current_time
    # if time range is invalid, return empty response
    if filters.end_time < filters.start_time:
        return CursorPaginationResponse(
            data=[],
            next_cursor=None,
        )

    if context.act_as.role == OrganizationRole.ADMIN:
        logs, next_log = mcp_tool_call_logs.get_by_org(
            db_session=context.db_session,
            organization_id=context.act_as.organization_id,
            limit=pagination.limit,
            cursor=cursor,
            mcp_tool_name=filters.mcp_tool_name,
            start_time=filters.start_time,
            end_time=filters.end_time,
        )
    else:
        logs, next_log = mcp_tool_call_logs.get_by_user(
            db_session=context.db_session,
            user_id=context.user_id,
            limit=pagination.limit,
            cursor=cursor,
            mcp_tool_name=filters.mcp_tool_name,
            start_time=filters.start_time,
            end_time=filters.end_time,
        )

    return CursorPaginationResponse(
        data=[MCPToolCallLogResponse.model_validate(log, from_attributes=True) for log in logs],
        next_cursor=MCPToolCallLogCursor.encode(next_log.started_at, next_log.id)
        if next_log
        else None,
    )
