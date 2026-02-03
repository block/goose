from datetime import datetime
from uuid import UUID

from pydantic import BaseModel, ConfigDict, Field

from aci.common.schemas.mcp_server_configuration import MCPServerConfigurationPublic
from aci.common.schemas.user import UserPublic


class MCPServerBundleCreate(BaseModel):
    name: str
    description: str | None = None
    mcp_server_configuration_ids: list[UUID] = Field(..., min_length=1)


class MCPServerBundlePublic(BaseModel):
    model_config = ConfigDict(extra="forbid")

    id: UUID
    name: str
    description: str | None = None
    user_id: UUID
    organization_id: UUID
    mcp_server_configurations: list[MCPServerConfigurationPublic]
    user: UserPublic

    created_at: datetime
    updated_at: datetime


class MCPServerBundlePublicWithBundleKey(MCPServerBundlePublic):
    bundle_key: str
