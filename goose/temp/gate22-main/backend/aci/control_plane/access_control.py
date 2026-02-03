from uuid import UUID

from sqlalchemy.orm import Session

from aci.common.db import crud
from aci.common.enums import OrganizationRole
from aci.common.logging_setup import get_logger
from aci.common.schemas.auth import ActAsInfo
from aci.control_plane.exceptions import NotPermittedError

logger = get_logger(__name__)


def check_act_as_organization_role(
    act_as: ActAsInfo,
    requested_organization_id: UUID | None = None,
    required_role: OrganizationRole = OrganizationRole.MEMBER,
    throw_error_if_not_permitted: bool = True,
) -> bool:
    """
    Check based on user's act_as information, verify if the user's act as:
    - Matches the requested organization_id
    - The role has permission to the requested role. (Admin is eligible to act as member role)

    This function throws an NotPermittedError if the user is not permitted to act as the requested
    organization and role.
    """
    try:
        if requested_organization_id and act_as.organization_id != requested_organization_id:
            raise NotPermittedError(
                message=f"ActAs organization_id {act_as.organization_id} does not match the "
                f"requested organization_id {requested_organization_id}"
            )
        if required_role == OrganizationRole.ADMIN and act_as.role != OrganizationRole.ADMIN:
            raise NotPermittedError(
                message=f"ActAs role {act_as.role} is not permitted to perform this action. "
                f"Required role: {required_role}"
            )
    except NotPermittedError as e:
        logger.error(f"NotPermittedError: {e.message}")
        if throw_error_if_not_permitted:
            raise e
        return False

    return True


def check_mcp_server_config_accessibility(
    db_session: Session,
    user_id: UUID,
    mcp_server_configuration_id: UUID,
    throw_error_if_not_permitted: bool = True,
) -> bool:
    """
    Returns:
        Whether the organization member has access to a MCP server configuration.
        Current rule:
        - True if the organization member belongs to any team that is allowed by the MCP server
        configuration
        - False otherwise
    """
    logger.debug(
        f"Checking if User {user_id} has access to the MCPServerConfiguration {mcp_server_configuration_id} as member"  # noqa: E501
    )

    mcp_server_configuration = crud.mcp_server_configurations.get_mcp_server_configuration_by_id(
        db_session, mcp_server_configuration_id, throw_error_if_not_found=True
    )

    user_teams = crud.teams.get_teams_by_user_id(
        db_session, mcp_server_configuration.organization_id, user_id
    )
    user_team_ids: set[UUID] = {team.id for team in user_teams}
    allowed_team_ids: set[UUID] = set(mcp_server_configuration.allowed_teams or [])

    logger.debug(f"User teams: {user_team_ids}")
    logger.debug(f"Config allowed_teams: {allowed_team_ids}")

    # Check if any of the user's team is allowed by the MCP server configuration
    # (if any overlap between user_team_ids and allowed_team_ids)
    if user_team_ids.intersection(allowed_team_ids):
        logger.debug(
            f"User {user_id} has access to MCP Server Configuration {mcp_server_configuration_id}"
        )
        return True
    else:
        if throw_error_if_not_permitted:
            raise NotPermittedError(
                message=f"none of the user's team is allowed in MCP Server "
                f"Configuration {mcp_server_configuration_id}"
            )
        return False


def check_mcp_server_accessibility(
    db_session: Session,
    act_as: ActAsInfo,
    user_id: UUID,
    mcp_server_id: UUID,
    throw_error_if_not_permitted: bool = True,
) -> bool:
    """
    Check if the user has access to a MCP server.
    """
    logger.debug(f"Checking if User {user_id} has access to the MCPServer {mcp_server_id}")
    mcp_server = crud.mcp_servers.get_mcp_server_by_id(
        db_session, mcp_server_id, throw_error_if_not_found=True
    )
    # Public MCP server, anyone can access
    if mcp_server.organization_id is None:
        return True

    # Check if the user belongs to the organization
    if act_as.organization_id != mcp_server.organization_id:
        if throw_error_if_not_permitted:
            raise NotPermittedError(
                message=f"User {user_id} has no access to the MCP Server {mcp_server_id}"
            )
        return False

    return True
