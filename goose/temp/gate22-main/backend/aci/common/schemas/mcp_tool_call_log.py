import base64
import json
from datetime import UTC, datetime
from uuid import UUID

from pydantic import BaseModel, ConfigDict, Field, field_validator

from aci.common.enums import MCPToolCallStatus


class MCPToolCallLogCreate(BaseModel):
    organization_id: UUID
    user_id: UUID
    request_id: str
    session_id: UUID
    bundle_name: str
    bundle_id: UUID
    mcp_server_name: str | None = None
    mcp_server_id: UUID | None = None
    mcp_tool_name: str | None = None
    mcp_tool_id: UUID | None = None
    mcp_server_configuration_name: str | None = None
    mcp_server_configuration_id: UUID | None = None
    arguments: str | None = None
    result: dict
    status: MCPToolCallStatus
    via_execute_tool: bool
    jsonrpc_payload: dict

    started_at: datetime
    ended_at: datetime
    duration_ms: int

    model_config = ConfigDict(extra="forbid")


class MCPToolCallLogResponse(BaseModel):
    id: UUID
    organization_id: UUID
    user_id: UUID
    request_id: str
    session_id: UUID
    bundle_name: str
    bundle_id: UUID
    mcp_server_name: str | None = None
    mcp_server_id: UUID | None = None
    mcp_tool_name: str | None = None
    mcp_tool_id: UUID | None = None
    mcp_server_configuration_name: str | None = None
    mcp_server_configuration_id: UUID | None = None
    arguments: str | None = None
    result: dict
    status: MCPToolCallStatus
    via_execute_tool: bool
    jsonrpc_payload: dict

    started_at: datetime
    ended_at: datetime
    duration_ms: int

    created_at: datetime
    updated_at: datetime


class MCPToolCallLogCursor(BaseModel):
    """
    Internal cursor representation for time-series pagination.
    """

    started_at: datetime
    id: UUID

    @staticmethod
    def encode(started_at: datetime, id: UUID) -> str:
        """Encode cursor to base64 string."""
        payload = {
            "started_at": started_at.isoformat(),
            "id": str(id),
        }
        return base64.urlsafe_b64encode(json.dumps(payload).encode()).decode()

    @staticmethod
    def decode(cursor: str) -> "MCPToolCallLogCursor":
        """Decode cursor from base64 string."""
        data = json.loads(base64.urlsafe_b64decode(cursor.encode()).decode())
        return MCPToolCallLogCursor(
            started_at=datetime.fromisoformat(data["started_at"]),
            id=UUID(data["id"]),
        )


class MCPToolCallLogFilters(BaseModel):
    start_time: datetime | None = Field(
        default=None, description="Start time to filter by.", examples=["2025-06-14T14:53:40.693Z"]
    )
    end_time: datetime | None = Field(
        default=None, description="End time to filter by.", examples=["2025-06-14T14:53:50.693Z"]
    )
    mcp_tool_name: str | None = Field(
        default=None,
        description="Search for tool names (case-insensitive partial match)",
        examples=["TOOL_A"],
    )

    @field_validator("start_time", "end_time", mode="after")
    @classmethod
    def normalize_timezone(cls, value: datetime | None) -> datetime | None:
        """make sure it's timezone aware if provided value is not"""
        if value is not None and value.tzinfo is None:
            return value.replace(tzinfo=UTC)
        return value
