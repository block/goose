from typing import Literal, overload

from sqlalchemy import select
from sqlalchemy.orm import Session

from aci.common import utils
from aci.common.db import crud
from aci.common.db.sql_models import VirtualMCPTool
from aci.common.schemas.virtual_mcp import VirtualMCPToolUpsert


@overload
def get_tool(
    db_session: Session, name: str, throw_error_if_not_found: Literal[True]
) -> VirtualMCPTool: ...


@overload
def get_tool(
    db_session: Session, name: str, throw_error_if_not_found: Literal[False]
) -> VirtualMCPTool | None: ...


def get_tool(
    db_session: Session, name: str, throw_error_if_not_found: bool
) -> VirtualMCPTool | None:
    statement = select(VirtualMCPTool).where(VirtualMCPTool.name == name)

    mcp_tool: VirtualMCPTool | None = None
    if throw_error_if_not_found:
        mcp_tool = db_session.execute(statement).scalar_one()
        return mcp_tool
    else:
        mcp_tool = db_session.execute(statement).scalar_one_or_none()
        return mcp_tool


def create_tools(
    db_session: Session,
    tool_upserts: list[VirtualMCPToolUpsert],
) -> list[VirtualMCPTool]:
    """
    Create the mcp tools in the database.
    Each tool might be of a different mcp server.
    """
    tools = []

    for tool_upsert in tool_upserts:
        vms_name = utils.parse_mcp_server_name_from_mcp_tool_name(tool_upsert.name)
        vms = crud.virtual_mcp.servers.get_server(
            db_session, vms_name, throw_error_if_not_found=True
        )

        tool_data = tool_upsert.model_dump(mode="json", exclude_none=True)

        tool = VirtualMCPTool(
            virtual_mcp_server_id=vms.id,
            **tool_data,
        )
        db_session.add(tool)
        tools.append(tool)

    db_session.flush()
    return tools


def update_tools(
    db_session: Session,
    tool_upserts: list[VirtualMCPToolUpsert],
) -> list[VirtualMCPTool]:
    """
    Update the mcp tools in the database.
    Each tool might be of a different mcp server.
    """
    tools = []

    for tool_upsert in tool_upserts:
        tool = crud.virtual_mcp.tools.get_tool(
            db_session, tool_upsert.name, throw_error_if_not_found=True
        )
        tool_data = tool_upsert.model_dump(mode="json", exclude_none=True)
        for field, value in tool_data.items():
            setattr(tool, field, value)

        tools.append(tool)

    return tools
