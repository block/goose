import json
from pathlib import Path

import click
from deepdiff import DeepDiff
from rich.console import Console
from rich.table import Table
from sqlalchemy.orm import Session

from aci.cli import config
from aci.common import embeddings, utils
from aci.common.db import crud
from aci.common.openai_client import get_openai_client
from aci.common.schemas.mcp_tool import MCPToolEmbeddingFields, MCPToolUpsert

console = Console()


@click.command()
@click.option(
    "--tools-file",
    "tools_file",
    required=True,
    type=click.Path(exists=True, path_type=Path),
    help="Path to the mcp tools JSON file",
)
@click.option(
    "--skip-dry-run",
    is_flag=True,
    help="Provide this flag to run the command and apply changes to the database",
)
def upsert_mcp_tools(tools_file: Path, skip_dry_run: bool) -> list[str]:
    """
    Upsert mcp tools in the DB from a JSON file.

    This command groups the mcp tools into three categories:
      - New mcp tools to create,
      - Existing mcp tools that require an update,
      - Mcp tools that are unchanged.

    Batch creation and update operations are performed.
    """
    return upsert_mcp_tools_helper(tools_file, skip_dry_run)


def upsert_mcp_tools_helper(tools_file: Path, skip_dry_run: bool) -> list[str]:
    with utils.create_db_session(config.DB_FULL_URL) as db_session:
        with open(tools_file) as f:
            mcp_tools_data = json.load(f)

        # Validate and parse each mcp tool record
        mcp_tool_upserts = [
            MCPToolUpsert.model_validate(mcp_tool_data) for mcp_tool_data in mcp_tools_data
        ]
        mcp_server_name = _validate_all_mcp_tools_belong_to_the_mcp_server(mcp_tool_upserts)
        console.rule(f"MCP Server={mcp_server_name}")
        _validate_mcp_server_exists(db_session, mcp_server_name)

        new_mcp_tools: list[MCPToolUpsert] = []
        existing_mcp_tools: list[MCPToolUpsert] = []

        for mcp_tool_upsert in mcp_tool_upserts:
            existing_mcp_tool = crud.mcp_tools.get_mcp_tool_by_name(
                db_session, mcp_tool_upsert.name, throw_error_if_not_found=False
            )

            if existing_mcp_tool is None:
                new_mcp_tools.append(mcp_tool_upsert)
            else:
                existing_mcp_tools.append(mcp_tool_upsert)

        console.rule("Checking mcp tools to create...")
        mcp_tools_created = create_mcp_tools_helper(db_session, new_mcp_tools)
        console.rule("Checking mcp tools to update...")
        mcp_tools_updated = update_mcp_tools_helper(db_session, existing_mcp_tools)
        # for mcp tools that are in existing_mcp_tools but not in mcp_tools_updated
        mcp_tools_unchanged = [
            mcp_tool.name
            for mcp_tool in existing_mcp_tools
            if mcp_tool.name not in mcp_tools_updated
        ]

        if not skip_dry_run:
            console.rule("Provide [bold green]--skip-dry-run[/bold green] to upsert mcp tools")
            db_session.rollback()
        else:
            db_session.commit()
            console.rule("[bold green]Upserted mcp tools[/bold green]")

        table = Table("MCP Tool Name", "Operation")
        for mcp_tool in mcp_tools_created:
            table.add_row(mcp_tool, "Create")
        for mcp_tool in mcp_tools_updated:
            table.add_row(mcp_tool, "Update")
        for mcp_tool in mcp_tools_unchanged:
            table.add_row(mcp_tool, "No changes")

        console.print(table)

        return mcp_tools_created + mcp_tools_updated


def create_mcp_tools_helper(
    db_session: Session, mcp_tool_upserts: list[MCPToolUpsert]
) -> list[str]:
    """
    Batch creates mcp tools in the database.
    Generates embeddings for each new mcp tool and calls the CRUD layer for creation.
    Returns a list of created mcp tool names.
    """
    mcp_tool_embeddings = embeddings.generate_mcp_tool_embeddings(
        get_openai_client(),
        [
            MCPToolEmbeddingFields.model_validate(mcp_tool.model_dump())
            for mcp_tool in mcp_tool_upserts
        ],
    )
    created_mcp_tools = crud.mcp_tools.create_mcp_tools(
        db_session, mcp_tool_upserts, mcp_tool_embeddings
    )

    return [mcp_tool.name for mcp_tool in created_mcp_tools]


def update_mcp_tools_helper(
    db_session: Session, mcp_tool_upserts: list[MCPToolUpsert]
) -> list[str]:
    """
    Batch updates mcp tools in the database.

    For each mcp tool to update, determines if the embedding needs to be regenerated.
    Regenerates embeddings in batch for those that require it and updates the mcp tools accordingly.
    Returns a list of updated mcp tool names.
    """

    # TODO: Consider adapting diff_tools() from mcp_tool_utils.py
    mcp_tools_with_new_embeddings: list[MCPToolUpsert] = []
    mcp_tools_without_new_embeddings: list[MCPToolUpsert] = []

    for mcp_tool_upsert in mcp_tool_upserts:
        existing_mcp_tool = crud.mcp_tools.get_mcp_tool_by_name(
            db_session, mcp_tool_upsert.name, throw_error_if_not_found=True
        )
        existing_mcp_tool_upsert = MCPToolUpsert.model_validate(
            existing_mcp_tool, from_attributes=True
        )
        if existing_mcp_tool_upsert == mcp_tool_upsert:
            continue
        else:
            diff = DeepDiff(
                existing_mcp_tool_upsert.model_dump(),
                mcp_tool_upsert.model_dump(),
                ignore_order=True,
            )
            console.rule(
                f"Will update mcp tool '{existing_mcp_tool.name}' with the following changes:"
            )
            console.print(diff.pretty())

        if _need_mcp_tool_embedding_regeneration(existing_mcp_tool_upsert, mcp_tool_upsert):
            mcp_tools_with_new_embeddings.append(mcp_tool_upsert)
        else:
            mcp_tools_without_new_embeddings.append(mcp_tool_upsert)

    # Generate new embeddings in batch for mcp tools that require regeneration.
    mcp_tool_embeddings = embeddings.generate_mcp_tool_embeddings(
        get_openai_client(),
        [
            MCPToolEmbeddingFields.model_validate(mcp_tool.model_dump())
            for mcp_tool in mcp_tools_with_new_embeddings
        ],
    )

    # Note: the order matters here because the embeddings need to match the mcp tools
    mcp_tools_updated = crud.mcp_tools.update_mcp_tools(
        db_session,
        mcp_tools_with_new_embeddings + mcp_tools_without_new_embeddings,
        mcp_tool_embeddings + [None] * len(mcp_tools_without_new_embeddings),
    )

    # TODO: When there is any update to the mcp tools, storing the version (or the diff) of the
    # mcp tools in the database

    return [mcp_tool.name for mcp_tool in mcp_tools_updated]


def _validate_mcp_server_exists(db_session: Session, mcp_server_name: str) -> None:
    mcp_server = crud.mcp_servers.get_mcp_server_by_name(db_session, mcp_server_name, False)
    if not mcp_server:
        raise click.ClickException(f"MCP Server={mcp_server_name} does not exist")


def _validate_all_mcp_tools_belong_to_the_mcp_server(
    mcp_tool_upserts: list[MCPToolUpsert],
) -> str:
    mcp_server_names = {
        utils.parse_mcp_server_name_from_mcp_tool_name(mcp_tool_upsert.name)
        for mcp_tool_upsert in mcp_tool_upserts
    }
    if len(mcp_server_names) != 1:
        raise click.ClickException(
            "All mcp tools must belong to the same mcp server, "
            f"instead found multiple mcp servers={mcp_server_names}"
        )

    return mcp_server_names.pop()


def _need_mcp_tool_embedding_regeneration(
    old_mcp_tool: MCPToolUpsert, new_mcp_tool: MCPToolUpsert
) -> bool:
    """
    Determines if the mcp tool embedding should be regenerated based on changes in the
    fields used for embedding (name, description, parameters).
    """
    fields = set(MCPToolEmbeddingFields.model_fields.keys())
    return bool(old_mcp_tool.model_dump(include=fields) != new_mcp_tool.model_dump(include=fields))
