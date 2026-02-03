from datetime import datetime
from uuid import UUID

from pydantic import AnyHttpUrl, BaseModel, ConfigDict, Field, field_validator, model_validator

from aci.common import mcp_tool_utils
from aci.common.enums import AuthType, MCPServerTransportType
from aci.common.schemas.mcp_auth import AuthConfig
from aci.common.schemas.mcp_tool import MCPToolPublicWithoutSchema


# NOTE: using a generic metadata schema for now before the schema is finalized
# TODO: for now this is not used, some potential useful fields: "need_session"
class MCPServerMetadata(BaseModel):
    is_virtual_mcp_server: bool | None = None

    model_config = ConfigDict(extra="forbid")


class MCPServerPartialUpdateRequest(BaseModel):
    """Used for API schema validation"""

    model_config = ConfigDict(extra="forbid")

    description: str | None = Field(default=None)
    logo: str | None = Field(default=None)
    categories: list[str] | None = Field(default=None)

    @model_validator(mode="after")
    def reject_explicit_none(self) -> "MCPServerPartialUpdateRequest":
        non_nullable_fields = ["description", "logo", "categories"]

        for field in non_nullable_fields:
            if field in self.model_fields_set and getattr(self, field) is None:
                raise ValueError(f"{field} cannot be set to null")
        return self


class MCPServerPartialUpdate(BaseModel):
    """
    Used for Partial Updating data to the database.
    """

    model_config = ConfigDict(extra="forbid")

    description: str | None = Field(default=None)
    logo: str | None = Field(default=None)
    categories: list[str] | None = Field(default=None)
    last_synced_at: datetime | None = Field(default=None)


def _validate_mcp_server_name(v: str) -> str:
    msg = (
        "name must be uppercase, contain only letters, numbers and underscores, not "
        "have consecutive underscores, and not start or end with an underscore"
    )

    if v != mcp_tool_utils.sanitize_canonical_name(v):
        raise ValueError(msg)

    return v


class MCPServerUpsert(BaseModel):
    """Used for Upserting data to the database"""

    model_config = ConfigDict(extra="ignore")

    name: str
    url: str
    transport_type: MCPServerTransportType
    description: str
    logo: str
    categories: list[str]
    auth_configs: list[AuthConfig]
    server_metadata: MCPServerMetadata

    @field_validator("name")
    def validate_name(cls, v: str) -> str:
        return _validate_mcp_server_name(v)


class PublicMCPServerUpsertRequest(MCPServerUpsert):
    """Used for API schema validation"""

    model_config = ConfigDict(extra="forbid")

    pass


class CustomMCPServerCreateRequest(BaseModel):
    """Used for API schema validation"""

    model_config = ConfigDict(extra="forbid")

    operational_account_auth_type: AuthType

    name: str
    url: str

    # Transport Type will be inferred from the url if not provided
    transport_type: MCPServerTransportType | None = None

    description: str
    # Logo will be attempted to be discovered in API logic
    logo: str | None = None
    categories: list[str]
    auth_configs: list[AuthConfig]
    server_metadata: MCPServerMetadata

    @field_validator("name")
    def validate_name(cls, v: str) -> str:
        return _validate_mcp_server_name(v)

    @model_validator(mode="after")
    def validate_operational_account_auth_type(self) -> "CustomMCPServerCreateRequest":
        for auth_config in self.auth_configs:
            if auth_config.root.type == self.operational_account_auth_type:
                return self

        raise ValueError("operational_account_auth_type must be in one of the auth_configs")


class MCPServerEmbeddingFields(BaseModel):
    """
    Fields used for generating the embeddings.
    """

    name: str
    url: str
    description: str
    categories: list[str]


class MCPServerPublic(BaseModel):
    id: UUID
    name: str
    url: str
    organization_id: UUID | None
    last_synced_at: datetime | None
    # TODO: is it necessary to expose transport_type?
    description: str
    logo: str
    categories: list[str]
    supported_auth_types: list[AuthType]

    tools: list[MCPToolPublicWithoutSchema]

    created_at: datetime
    updated_at: datetime


class MCPServerOAuth2DiscoveryRequest(BaseModel):
    mcp_server_url: AnyHttpUrl


class MCPServerOAuth2DiscoveryResponse(BaseModel):
    authorize_url: AnyHttpUrl | None = None
    access_token_url: AnyHttpUrl | None = None
    refresh_token_url: AnyHttpUrl | None = None

    # This could be none if DCR is not supported by the server
    registration_url: AnyHttpUrl | None = None
    token_endpoint_auth_method_supported: list[str]


class MCPServerOAuth2DCRRequest(BaseModel):
    mcp_server_url: AnyHttpUrl
    registration_url: AnyHttpUrl
    token_endpoint_auth_method_supported: list[str]


class MCPServerOAuth2DCRResponse(BaseModel):
    token_endpoint_auth_method: str  # The method we finally used to register the client
    client_id: str | None = None
    client_secret: str | None = None
