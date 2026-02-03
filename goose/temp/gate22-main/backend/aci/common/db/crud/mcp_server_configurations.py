from typing import Literal, overload
from uuid import UUID

from sqlalchemy import select
from sqlalchemy.orm import Session

from aci.common.db.sql_models import MCPServerConfiguration, Team
from aci.common.enums import ConnectedAccountOwnership
from aci.common.schemas.mcp_server_configuration import (
    MCPServerConfigurationCreate,
    MCPServerConfigurationUpdate,
)


def create_mcp_server_configuration(
    db_session: Session,
    organization_id: UUID,
    mcp_server_configuration: MCPServerConfigurationCreate,
) -> MCPServerConfiguration:
    db_mcp_server_configuration = MCPServerConfiguration(
        name=mcp_server_configuration.name,
        description=mcp_server_configuration.description,
        mcp_server_id=mcp_server_configuration.mcp_server_id,
        organization_id=organization_id,
        auth_type=mcp_server_configuration.auth_type,
        all_tools_enabled=mcp_server_configuration.all_tools_enabled,
        enabled_tools=mcp_server_configuration.enabled_tools,
        allowed_teams=mcp_server_configuration.allowed_teams,
        connected_account_ownership=mcp_server_configuration.connected_account_ownership,
    )
    db_session.add(db_mcp_server_configuration)
    db_session.flush()
    db_session.refresh(db_mcp_server_configuration)

    return db_mcp_server_configuration


@overload
def get_mcp_server_configuration_by_id(
    db_session: Session,
    mcp_server_configuration_id: UUID,
    throw_error_if_not_found: Literal[True],
) -> MCPServerConfiguration: ...


@overload
def get_mcp_server_configuration_by_id(
    db_session: Session,
    mcp_server_configuration_id: UUID,
    throw_error_if_not_found: Literal[False],
) -> MCPServerConfiguration | None: ...


def get_mcp_server_configuration_by_id(
    db_session: Session,
    mcp_server_configuration_id: UUID,
    throw_error_if_not_found: bool,
) -> MCPServerConfiguration | None:
    statement = select(MCPServerConfiguration).where(
        MCPServerConfiguration.id == mcp_server_configuration_id
    )

    mcp_server_configuration: MCPServerConfiguration | None = None
    if throw_error_if_not_found:
        mcp_server_configuration = db_session.execute(statement).scalar_one()
        return mcp_server_configuration
    else:
        mcp_server_configuration = db_session.execute(statement).scalar_one_or_none()
        return mcp_server_configuration


def get_mcp_server_configurations_by_ids(
    db_session: Session,
    mcp_server_configuration_ids: list[UUID],
) -> list[MCPServerConfiguration]:
    if not mcp_server_configuration_ids:
        return []

    statement = select(MCPServerConfiguration).where(
        MCPServerConfiguration.id.in_(mcp_server_configuration_ids)
    )

    # make sure the results are in the same order as the mcp_server_configuration_ids
    results = list(db_session.execute(statement).scalars().all())
    # map the rows by id, and use the order of requested ids to map the final results
    results_by_id = {result.id: result for result in results}
    return [
        results_by_id[mcp_server_configuration_id]
        for mcp_server_configuration_id in mcp_server_configuration_ids
        if mcp_server_configuration_id in results_by_id
    ]


def get_operational_mcp_server_configuration_mcp_server_id(
    db_session: Session,
    mcp_server_id: UUID,
) -> MCPServerConfiguration | None:
    statement = select(MCPServerConfiguration).where(
        MCPServerConfiguration.mcp_server_id == mcp_server_id,
        MCPServerConfiguration.connected_account_ownership == ConnectedAccountOwnership.OPERATIONAL,
    )
    return db_session.execute(statement).scalar_one_or_none()


def get_mcp_server_configurations(
    db_session: Session,
    organization_id: UUID,
    team_ids: list[UUID] | None = None,  # None means no filter
    mcp_server_id: UUID | None = None,  # None means no filter
    connected_account_ownerships: list[ConnectedAccountOwnership]
    | None = None,  # None means no filter
    offset: int | None = None,
    limit: int | None = None,
) -> list[MCPServerConfiguration]:
    statement = select(MCPServerConfiguration).where(
        MCPServerConfiguration.organization_id == organization_id,
    )

    if team_ids is not None:
        # Verify the team ids are in the org
        select_team_statement = select(Team).where(
            Team.id.in_(team_ids), Team.organization_id == organization_id
        )
        teams = list(db_session.execute(select_team_statement).scalars().all())

        # overlap() is not type hinted but is available.
        # Make sure the field is Column of PostgreSQL array (import sqlalchemy.dialects.postgresql)
        statement = statement.where(
            MCPServerConfiguration.allowed_teams.overlap([team.id for team in teams]),
        )

    if mcp_server_id is not None:
        statement = statement.where(MCPServerConfiguration.mcp_server_id == mcp_server_id)

    if connected_account_ownerships is not None:
        statement = statement.where(
            MCPServerConfiguration.connected_account_ownership.in_(connected_account_ownerships),
        )

    statement = statement.order_by(MCPServerConfiguration.created_at.desc())
    if offset is not None:
        statement = statement.offset(offset)
    if limit is not None:
        statement = statement.limit(limit)

    return list(db_session.execute(statement).scalars().all())


def delete_mcp_server_configuration(
    db_session: Session,
    mcp_server_configuration_id: UUID,
) -> None:
    db_session.query(MCPServerConfiguration).filter(
        MCPServerConfiguration.id == mcp_server_configuration_id
    ).delete()


def update_mcp_server_configuration(
    db_session: Session,
    mcp_server_configuration_id: UUID,
    mcp_server_configuration_update: MCPServerConfigurationUpdate,
) -> MCPServerConfiguration:
    statement = select(MCPServerConfiguration).where(
        MCPServerConfiguration.id == mcp_server_configuration_id
    )
    mcp_server_configuration = db_session.execute(statement).scalar_one()

    for field, value in mcp_server_configuration_update.model_dump(exclude_unset=True).items():
        setattr(mcp_server_configuration, field, value)

    return mcp_server_configuration
