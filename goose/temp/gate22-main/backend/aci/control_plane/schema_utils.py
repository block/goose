from sqlalchemy.orm.session import Session

from aci.common.db import crud
from aci.common.db.sql_models import (
    ConnectedAccount,
    MCPServer,
    MCPServerBundle,
    MCPServerConfiguration,
)
from aci.common.enums import ConnectedAccountOwnership
from aci.common.schemas.connected_account import ConnectedAccountPublic
from aci.common.schemas.mcp_auth import AuthConfig
from aci.common.schemas.mcp_server import MCPServerPublic
from aci.common.schemas.mcp_server_bundle import (
    MCPServerBundlePublic,
    MCPServerBundlePublicWithBundleKey,
)
from aci.common.schemas.mcp_server_configuration import MCPServerConfigurationPublic
from aci.common.schemas.mcp_tool import MCPToolPublicWithoutSchema
from aci.common.schemas.organization import TeamInfo
from aci.common.schemas.user import UserPublic


def construct_mcp_server_public(mcp_server: MCPServer) -> MCPServerPublic:
    auth_configs = [
        AuthConfig.model_validate(auth_config_dict) for auth_config_dict in mcp_server.auth_configs
    ]

    return MCPServerPublic(
        id=mcp_server.id,
        name=mcp_server.name,
        url=mcp_server.url,
        organization_id=mcp_server.organization_id,
        last_synced_at=mcp_server.last_synced_at,
        description=mcp_server.description,
        logo=mcp_server.logo,
        categories=mcp_server.categories,
        supported_auth_types=[auth_config.root.type for auth_config in auth_configs],
        tools=[
            MCPToolPublicWithoutSchema.model_validate(tool, from_attributes=True)
            for tool in mcp_server.tools
        ],
        created_at=mcp_server.created_at,
        updated_at=mcp_server.updated_at,
    )


def construct_mcp_server_configuration_public(
    db_session: Session, mcp_server_configuration: MCPServerConfiguration
) -> MCPServerConfigurationPublic:
    """
    Dynamically retrieve and populate the enabled_tools and allowed_teams
    for the MCP server configuration.
    """
    enabled_tools = crud.mcp_tools.get_mcp_tools_by_ids(
        db_session, mcp_server_configuration.enabled_tools
    )
    allowed_teams = crud.teams.get_teams_by_ids(db_session, mcp_server_configuration.allowed_teams)

    if (
        mcp_server_configuration.connected_account_ownership
        == ConnectedAccountOwnership.OPERATIONAL
    ):
        if crud.connected_accounts.get_operational_connected_account_by_mcp_server_configuration_id(
            db_session, mcp_server_configuration.id
        ):
            has_operational_connected_account = True
        else:
            has_operational_connected_account = False
    else:
        has_operational_connected_account = None

    return MCPServerConfigurationPublic(
        id=mcp_server_configuration.id,
        name=mcp_server_configuration.name,
        description=mcp_server_configuration.description,
        mcp_server_id=mcp_server_configuration.mcp_server_id,
        organization_id=mcp_server_configuration.organization_id,
        auth_type=mcp_server_configuration.auth_type,
        connected_account_ownership=mcp_server_configuration.connected_account_ownership,
        all_tools_enabled=mcp_server_configuration.all_tools_enabled,
        enabled_tools=[
            MCPToolPublicWithoutSchema.model_validate(tool, from_attributes=True)
            for tool in enabled_tools
        ],
        allowed_teams=[
            TeamInfo(
                team_id=team.id,
                name=team.name,
                description=team.description,
                created_at=team.created_at,
            )
            for team in allowed_teams
        ],
        mcp_server=construct_mcp_server_public(mcp_server_configuration.mcp_server),
        has_operational_connected_account=has_operational_connected_account,
        created_at=mcp_server_configuration.created_at,
        updated_at=mcp_server_configuration.updated_at,
    )


def construct_mcp_server_bundle_public(
    db_session: Session, mcp_server_bundle: MCPServerBundle, include_bundle_key: bool = False
) -> MCPServerBundlePublic | MCPServerBundlePublicWithBundleKey:
    """
    Dynamically retrieve and populate the mcp_server_configurations
    for the MCP server bundle.
    """
    mcp_server_configurations = crud.mcp_server_configurations.get_mcp_server_configurations_by_ids(
        db_session, mcp_server_bundle.mcp_server_configuration_ids
    )

    bundle_public = MCPServerBundlePublic(
        id=mcp_server_bundle.id,
        name=mcp_server_bundle.name,
        description=mcp_server_bundle.description,
        user_id=mcp_server_bundle.user_id,
        organization_id=mcp_server_bundle.organization_id,
        mcp_server_configurations=[
            construct_mcp_server_configuration_public(db_session, mcp_server_configuration)
            for mcp_server_configuration in mcp_server_configurations
        ],
        user=UserPublic.model_validate(mcp_server_bundle.user, from_attributes=True),
        created_at=mcp_server_bundle.created_at,
        updated_at=mcp_server_bundle.updated_at,
    )

    if include_bundle_key:
        bundle_public = MCPServerBundlePublicWithBundleKey(
            **bundle_public.model_dump(),
            bundle_key=mcp_server_bundle.bundle_key,
        )
        return bundle_public
    else:
        return bundle_public


def construct_connected_account_public(
    db_session: Session, connected_account: ConnectedAccount
) -> ConnectedAccountPublic:
    return ConnectedAccountPublic(
        id=connected_account.id,
        user_id=connected_account.user_id,
        mcp_server_configuration_id=connected_account.mcp_server_configuration_id,
        ownership=connected_account.ownership,
        created_at=connected_account.created_at,
        updated_at=connected_account.updated_at,
        mcp_server_configuration=construct_mcp_server_configuration_public(
            db_session, connected_account.mcp_server_configuration
        ),
        user=UserPublic.model_validate(connected_account.user, from_attributes=True),
    )
