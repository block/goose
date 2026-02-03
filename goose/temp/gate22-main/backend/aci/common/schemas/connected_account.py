from datetime import datetime
from uuid import UUID

from pydantic import BaseModel, Field

from aci.common.enums import ConnectedAccountOwnership
from aci.common.schemas.mcp_server_configuration import (
    MCPServerConfigurationPublic,
)
from aci.common.schemas.user import UserPublic


class ConnectedAccountAPIKeyCreate(BaseModel, extra="forbid"):
    mcp_server_configuration_id: UUID
    api_key: str = Field(min_length=1)  # for API key auth type. # TODO: use SecretStr


class ConnectedAccountOAuth2Create(BaseModel, extra="forbid"):
    mcp_server_configuration_id: UUID
    redirect_url_after_account_creation: str | None = None  # for OAuth2 auth type


class ConnectedAccountNoAuthCreate(BaseModel, extra="forbid"):
    mcp_server_configuration_id: UUID


class ConnectedAccountCreate(BaseModel):
    mcp_server_configuration_id: UUID
    api_key: str | None = None
    redirect_url_after_account_creation: str | None = None


class OAuth2ConnectedAccountCreateResponse(BaseModel):
    authorization_url: str


class ConnectedAccountOAuth2CreateState(BaseModel):
    mcp_server_configuration_id: UUID
    user_id: UUID
    code_verifier: str
    redirect_url_after_account_creation: str | None = None
    # TODO: add expires at?


class ConnectedAccountPublic(BaseModel):
    id: UUID
    user_id: UUID
    mcp_server_configuration_id: UUID
    ownership: ConnectedAccountOwnership
    # TODO: add auth credentials (with access token, refresh token removed)

    created_at: datetime
    updated_at: datetime

    mcp_server_configuration: MCPServerConfigurationPublic
    user: UserPublic
