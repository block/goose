from typing import Literal, overload
from uuid import UUID

from sqlalchemy import case, or_, select
from sqlalchemy.orm import Session

from aci.common.db.sql_models import MCPServer
from aci.common.logging_setup import get_logger
from aci.common.schemas.mcp_server import MCPServerPartialUpdate, MCPServerUpsert

logger = get_logger(__name__)


@overload
def get_mcp_server_by_name(
    db_session: Session, name: str, throw_error_if_not_found: Literal[True]
) -> MCPServer: ...


@overload
def get_mcp_server_by_name(
    db_session: Session, name: str, throw_error_if_not_found: Literal[False]
) -> MCPServer | None: ...


def get_mcp_server_by_name(
    db_session: Session, name: str, throw_error_if_not_found: bool
) -> MCPServer | None:
    statement = select(MCPServer).where(MCPServer.name == name)

    mcp_server: MCPServer | None = None
    if throw_error_if_not_found:
        mcp_server = db_session.execute(statement).scalar_one()
        return mcp_server
    else:
        mcp_server = db_session.execute(statement).scalar_one_or_none()
        return mcp_server


@overload
def get_mcp_server_by_id(
    db_session: Session, id: UUID, throw_error_if_not_found: Literal[True]
) -> MCPServer: ...


@overload
def get_mcp_server_by_id(
    db_session: Session, id: UUID, throw_error_if_not_found: Literal[False]
) -> MCPServer | None: ...


def get_mcp_server_by_id(
    db_session: Session, id: UUID, throw_error_if_not_found: bool
) -> MCPServer | None:
    statement = select(MCPServer).where(MCPServer.id == id)

    mcp_server: MCPServer | None = None
    if throw_error_if_not_found:
        mcp_server = db_session.execute(statement).scalar_one()
        return mcp_server
    else:
        mcp_server = db_session.execute(statement).scalar_one_or_none()
        return mcp_server


def create_mcp_server(
    db_session: Session,
    organization_id: UUID | None,
    mcp_server_upsert: MCPServerUpsert,
    embedding: list[float],
) -> MCPServer:
    mcp_server_data = mcp_server_upsert.model_dump(mode="json", exclude_none=True)
    mcp_server = MCPServer(
        **mcp_server_data,
        embedding=embedding,
        organization_id=organization_id,
        last_synced_at=None,
    )
    db_session.add(mcp_server)
    db_session.flush()
    db_session.refresh(mcp_server)
    return mcp_server


def update_mcp_server(
    db_session: Session,
    mcp_server: MCPServer,
    mcp_server_upsert: MCPServerUpsert | MCPServerPartialUpdate,
    embedding: list[float] | None = None,
) -> MCPServer:
    new_mcp_server_data = mcp_server_upsert.model_dump(mode="json", exclude_unset=True)

    for field, value in new_mcp_server_data.items():
        setattr(mcp_server, field, value)

    if embedding:
        mcp_server.embedding = embedding

    return mcp_server


def delete_mcp_server(
    db_session: Session,
    mcp_server_id: UUID,
) -> None:
    mcp_server = get_mcp_server_by_id(db_session, mcp_server_id, throw_error_if_not_found=True)
    db_session.delete(mcp_server)


def list_mcp_servers(
    db_session: Session,
    organization_id: UUID | None = None,
    offset: int | None = None,
    limit: int | None = None,
) -> list[MCPServer]:
    """
    Returns:
        List of MCP Servers.
        If organization_id is provided, returns Public MCP Servers + Custom MCP Servers under the
        organization.
        If not provided, returns all Public MCP Servers.
    """
    statement = select(MCPServer)

    if offset is not None:
        statement = statement.offset(offset)
    if limit is not None:
        statement = statement.limit(limit)

    if organization_id is not None:
        statement = statement.where(
            or_(
                MCPServer.organization_id == organization_id,
                MCPServer.organization_id.is_(None),
            )
        )
        # Sort by: custom servers first, then public servers, then alphabetically by name
        statement = statement.order_by(
            case(
                (MCPServer.organization_id == organization_id, 0),
                (MCPServer.organization_id.is_(None), 1),
            ),
            MCPServer.name.asc(),
        )
    else:
        statement = statement.where(MCPServer.organization_id.is_(None))
        statement = statement.order_by(MCPServer.name.asc())

    servers = list(db_session.execute(statement).scalars().all())
    return servers


def count_mcp_servers_by_organization_id(
    db_session: Session,
    organization_id: UUID,
) -> int:
    return db_session.query(MCPServer).filter(MCPServer.organization_id == organization_id).count()
