from typing import Literal, overload

from sqlalchemy import select
from sqlalchemy.orm import Session

from aci.common.db.sql_models import VirtualMCPServer
from aci.common.schemas.virtual_mcp import VirtualMCPServerUpsert


# overloads for type hints
@overload
def get_server(
    db_session: Session,
    name: str,
    throw_error_if_not_found: Literal[True],
) -> VirtualMCPServer: ...


@overload
def get_server(
    db_session: Session,
    name: str,
    throw_error_if_not_found: Literal[False],
) -> VirtualMCPServer | None: ...


def get_server(
    db_session: Session,
    name: str,
    throw_error_if_not_found: bool = False,
) -> VirtualMCPServer | None:
    statement = select(VirtualMCPServer).where(VirtualMCPServer.name == name)
    if throw_error_if_not_found:
        return db_session.execute(statement).scalar_one()
    else:
        return db_session.execute(statement).scalar_one_or_none()


def create_server(db_session: Session, vms_upsert: VirtualMCPServerUpsert) -> VirtualMCPServer:
    vms = VirtualMCPServer(**vms_upsert.model_dump(mode="json", exclude_none=True))
    db_session.add(vms)
    db_session.flush()
    db_session.refresh(vms)

    return vms


def update_server(
    db_session: Session,
    vms: VirtualMCPServer,
    vms_upsert: VirtualMCPServerUpsert,
) -> VirtualMCPServer:
    vms_data = vms_upsert.model_dump(mode="json", exclude_none=True)
    for field, value in vms_data.items():
        setattr(vms, field, value)

    return vms
