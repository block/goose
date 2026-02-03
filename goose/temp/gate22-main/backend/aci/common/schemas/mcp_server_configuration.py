from datetime import datetime
from uuid import UUID

from pydantic import BaseModel, ConfigDict, Field, model_validator

from aci.common.enums import AuthType, ConnectedAccountOwnership
from aci.common.logging_setup import get_logger
from aci.common.schemas.mcp_server import MCPServerPublic
from aci.common.schemas.mcp_tool import MCPToolPublicWithoutSchema
from aci.common.schemas.organization import TeamInfo

logger = get_logger(__name__)


class MCPServerConfigurationCreate(BaseModel):
    """Create a new MCP configuration
    "all_tools_enabled=True" → ignore enabled_tools.
    "all_tools_enabled=False" AND non-empty enabled_tools → selectively enable that list.
    "all_tools_enabled=False" AND empty enabled_tools → all tools disabled.
    """

    # TODO: allow white-labeling by providingthe redirect url
    name: str = Field(min_length=1, max_length=100)
    # TODO: put magic number in constants
    description: str | None = Field(default=None, max_length=512)
    mcp_server_id: UUID
    auth_type: AuthType
    connected_account_ownership: ConnectedAccountOwnership
    all_tools_enabled: bool = Field(default=True)
    enabled_tools: list[UUID] = Field(default_factory=list)
    allowed_teams: list[UUID] = Field(default_factory=list)

    # when all_tools_enabled is True, enabled_tools provided by user should be empty
    @model_validator(mode="after")
    def check_all_tools_enabled(self) -> "MCPServerConfigurationCreate":
        if self.all_tools_enabled and self.enabled_tools:
            raise ValueError(
                "all_tools_enabled and enabled_tools cannot be both True and non-empty"
            )
        return self


class MCPServerConfigurationUpdate(BaseModel):
    model_config = ConfigDict(extra="forbid")

    name: str | None = Field(default=None, min_length=1, max_length=100)
    description: str | None = Field(default=None, max_length=512)
    all_tools_enabled: bool | None = None
    enabled_tools: list[UUID] | None = None
    allowed_teams: list[UUID] | None = None

    # when all_tools_enabled is True, enabled_tools provided by user should be empty
    #
    # To ensure integrity, `all_tools_enabled` and `enabled_tools` must either both provided or both
    # not provided. Otherwise the update to database may cause inconsistency.
    #
    @model_validator(mode="after")
    def check_all_tools_enabled(self) -> "MCPServerConfigurationUpdate":
        if (self.all_tools_enabled is None and self.enabled_tools is not None) or (
            self.all_tools_enabled is not None and self.enabled_tools is None
        ):
            raise ValueError(
                "all_tools_enabled and enabled_tools must either both provided or both not provided"
            )

        if self.all_tools_enabled and self.enabled_tools:
            raise ValueError(
                "all_tools_enabled and enabled_tools cannot be both True and non-empty"
            )
        return self

    @model_validator(mode="after")
    def check_non_nullable_fields(self) -> "MCPServerConfigurationUpdate":
        """
        We want to allow users to not provide some fields if they don't want to update them.
        But we want to prevent users from setting some non-nullable fields to None.

        However, seems it's not possible to achieve this constraint correctly with only field
        definitions in pydantic model. (Can't differentiate between None and not provided)
        We must use a custom validator to check for this case.
        """
        # TODO: Further study on how to achieve this constraint correctly with pydantic model.
        non_nullable_fields = ["name", "all_tools_enabled", "allowed_teams", "enabled_tools"]

        for field in self.model_fields_set:
            if field in non_nullable_fields and getattr(self, field) is None:
                logger.error(f"{field} cannot be None")
                raise ValueError(f"{field} cannot be None")
        return self


class MCPServerConfigurationPublic(BaseModel):
    id: UUID
    name: str
    description: str | None = None
    mcp_server_id: UUID
    organization_id: UUID
    auth_type: AuthType
    connected_account_ownership: ConnectedAccountOwnership
    all_tools_enabled: bool
    enabled_tools: list[MCPToolPublicWithoutSchema]
    allowed_teams: list[TeamInfo]

    has_operational_connected_account: bool | None

    created_at: datetime
    updated_at: datetime

    mcp_server: MCPServerPublic

    # TODO: scrub sensitive data from whitelabeling overrides if support in the future
