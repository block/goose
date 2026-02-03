import json
from pathlib import Path

import click
from deepdiff import DeepDiff
from rich.console import Console
from rich.table import Table
from sqlalchemy.orm import Session

from aci.cli import config
from aci.common import utils
from aci.common.db import crud
from aci.common.schemas.virtual_mcp import VirtualMCPToolUpsert

console = Console()


@click.command()
@click.option(
    "--tools-file",
    "tools_file",
    required=True,
    type=click.Path(exists=True, path_type=Path),
    help="Path to the virtual mcp tools JSON file",
)
@click.option(
    "--skip-dry-run",
    is_flag=True,
    help="Provide this flag to run the command and apply changes to the database",
)
def upsert(tools_file: Path, skip_dry_run: bool) -> list[str]:
    """
    Upsert virtual mcp tools in the DB from a JSON file.

    This command groups the mcp tools into three categories:
      - New virtual mcp tools to create,
      - Existing virtual mcp tools that require an update,
      - Virtual mcp tools that are unchanged.

    Batch creation and update operations are performed.
    """
    return upsert_helper(tools_file, skip_dry_run)


def upsert_helper(tools_file: Path, skip_dry_run: bool) -> list[str]:
    with utils.create_db_session(config.DB_FULL_URL) as db_session:
        with open(tools_file) as f:
            tools_data = json.load(f)

        # Validate and parse each mcp tool record
        tool_upserts = [VirtualMCPToolUpsert.model_validate(tool_data) for tool_data in tools_data]
        vms_name = _validate_all_tools_belong_to_the_vms(tool_upserts)
        console.rule(f"Virtual MCP Server={vms_name}")
        _validate_vms_exists(db_session, vms_name)

        new_tools: list[VirtualMCPToolUpsert] = []
        existing_tools: list[VirtualMCPToolUpsert] = []

        for tool_upsert in tool_upserts:
            existing_tool = crud.virtual_mcp.tools.get_tool(
                db_session, tool_upsert.name, throw_error_if_not_found=False
            )

            if existing_tool is None:
                new_tools.append(tool_upsert)
            else:
                existing_tools.append(tool_upsert)

        console.rule("Checking tools to create...")
        tools_created = create_helper(db_session, new_tools)
        console.rule("Checking tools to update...")
        tools_updated = update_helper(db_session, existing_tools)
        # for tools that are in existing_tools but not in tools_updated
        tools_unchanged = [tool.name for tool in existing_tools if tool.name not in tools_updated]

        if not skip_dry_run:
            console.rule("Provide [bold green]--skip-dry-run[/bold green] to upsert tools")
            db_session.rollback()
        else:
            db_session.commit()
            console.rule("[bold green]Upserted tools[/bold green]")

        table = Table("Tool Name", "Operation")
        for tool in tools_created:
            table.add_row(tool, "Create")
        for tool in tools_updated:
            table.add_row(tool, "Update")
        for tool in tools_unchanged:
            table.add_row(tool, "No changes")

        console.print(table)

        return tools_created + tools_updated


def create_helper(db_session: Session, tool_upserts: list[VirtualMCPToolUpsert]) -> list[str]:
    """
    Batch creates virtual mcp tools in the database.
    Returns a list of created tool names.
    """
    created_tools = crud.virtual_mcp.tools.create_tools(db_session, tool_upserts)

    return [tool.name for tool in created_tools]


def update_helper(db_session: Session, tool_upserts: list[VirtualMCPToolUpsert]) -> list[str]:
    """
    Batch updates virtual mcp tools in the database.
    Returns a list of updated tool names.
    """
    tools_to_update: list[VirtualMCPToolUpsert] = []
    for tool_upsert in tool_upserts:
        existing_tool = crud.virtual_mcp.tools.get_tool(
            db_session, tool_upsert.name, throw_error_if_not_found=True
        )
        existing_tool_upsert = VirtualMCPToolUpsert.model_validate(
            existing_tool, from_attributes=True
        )
        if existing_tool_upsert == tool_upsert:
            continue
        else:
            diff = DeepDiff(
                existing_tool_upsert.model_dump(),
                tool_upsert.model_dump(),
                ignore_order=True,
            )
            console.rule(f"Will update tool '{existing_tool.name}' with the following changes:")
            console.print(diff.pretty())
            tools_to_update.append(tool_upsert)

    # Note: the order matters here because the embeddings need to match the mcp tools
    crud.virtual_mcp.tools.update_tools(
        db_session,
        tools_to_update,
    )

    return [tool.name for tool in tools_to_update]


def _validate_vms_exists(db_session: Session, vms_name: str) -> None:
    vms = crud.virtual_mcp.servers.get_server(db_session, vms_name, False)
    if not vms:
        raise click.ClickException(f"Virtual MCP Server={vms_name} does not exist")


def _validate_all_tools_belong_to_the_vms(
    mcp_tool_upserts: list[VirtualMCPToolUpsert],
) -> str:
    vms_names = {
        utils.parse_mcp_server_name_from_mcp_tool_name(mcp_tool_upsert.name)
        for mcp_tool_upsert in mcp_tool_upserts
    }
    if len(vms_names) != 1:
        raise click.ClickException(
            "All tools must belong to the same virtual mcp server, "
            f"instead found multiple virtual mcp servers={vms_names}"
        )

    return vms_names.pop()
