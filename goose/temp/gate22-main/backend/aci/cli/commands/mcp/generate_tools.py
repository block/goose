import asyncio
import json
from datetime import datetime
from pathlib import Path

import click
from rich.console import Console
from sqlalchemy import select
from sqlalchemy.orm import Session

from aci.cli import config
from aci.common import auth_credentials_manager as acm
from aci.common import mcp_tool_utils, utils
from aci.common.db import crud
from aci.common.db.sql_models import (
    ConnectedAccount,
    MCPServer,
    MCPServerConfiguration,
)
from aci.common.enums import MCPServerTransportType
from aci.common.exceptions import MCPToolSanitizationError
from aci.common.mcp_auth_manager import MCPAuthManager
from aci.common.schemas.mcp_auth import (
    AuthConfig,
    AuthCredentials,
    NoAuthConfig,
    NoAuthCredentials,
)
from aci.common.schemas.mcp_server import MCPServerMetadata
from mcp import types as mcp_types
from mcp.client.session import ClientSession
from mcp.client.sse import sse_client
from mcp.client.streamable_http import streamablehttp_client

console = Console()


async def _generate_tools_async(mcp_server_name: str) -> None:
    """
    Generate the tools.json file for the given mcp server.
    Pre-requisites:
      - The mcp server must be inserted into the database.
      - One mcp server configuration must be created for the mcp server.
      - One connected account must be created for the mcp server configuration.
    """
    with utils.create_db_session(config.DB_FULL_URL) as db_session:
        # check if the mcp server exists
        mcp_server = crud.mcp_servers.get_mcp_server_by_name(db_session, mcp_server_name, False)
        if mcp_server is None:
            raise click.ClickException(f"MCP server {mcp_server_name} not found")

        auth_config, auth_credentials = await _get_auth(mcp_server, db_session)
        db_session.commit()

        # get list of tools from the mcp server
        tools = await _list_tools(
            mcp_server,
            auth_config,
            auth_credentials,
        )

        # create the tools.json file
        _create_tools_json_file(mcp_server_name, tools)


async def _get_auth(
    mcp_server: MCPServer, db_session: Session
) -> tuple[AuthConfig, AuthCredentials]:
    server_metadata = MCPServerMetadata.model_validate(mcp_server.server_metadata)
    # TODO: for now, virtual mcp servers tools/list doesn't require auth
    # so we just return some mock auth config and credentials
    if server_metadata.is_virtual_mcp_server:
        return AuthConfig(root=NoAuthConfig()), AuthCredentials(root=NoAuthCredentials())
    else:
        # find one of the mcp server configurations
        mcp_server_configuration = db_session.execute(
            select(MCPServerConfiguration)
            .where(MCPServerConfiguration.mcp_server_id == mcp_server.id)
            .limit(1)
        ).scalar()
        if mcp_server_configuration is None:
            console.print(
                f"[yellow]Warning: MCP server configuration for {mcp_server.name} not found, skipping tools generation[/yellow]"  # noqa: E501
            )
            raise click.ClickException(f"MCP server configuration for {mcp_server.name} not found")

        # find one connected account for the mcp server configuration
        connected_account = db_session.execute(
            select(ConnectedAccount)
            .where(ConnectedAccount.mcp_server_configuration_id == mcp_server_configuration.id)
            .limit(1)
        ).scalar()
        if connected_account is None:
            console.print(
                f"[yellow]Warning: Connected account for {mcp_server.name} not found, skipping tools generation[/yellow]"  # noqa: E501
            )
            raise click.ClickException(f"Connected account for {mcp_server.name} not found")

        # Get the auth config and credentials
        auth_config = acm.get_auth_config(mcp_server, mcp_server_configuration)
        # TODO: handle token refresh for oauth2 credentials
        auth_credentials = await acm.get_auth_credentials(
            db_session,
            mcp_server_configuration.id,
            connected_account.ownership,  # Use the ownership type of whatever account we retrieved
            user_id=connected_account.user_id,
        )

        return auth_config, auth_credentials


async def _list_tools(
    mcp_server: MCPServer,
    auth_config: AuthConfig,
    auth_credentials: AuthCredentials,
) -> list[mcp_types.Tool]:
    mcp_auth_credentials_manager = MCPAuthManager(
        mcp_server=mcp_server,
        auth_config=auth_config,
        auth_credentials=auth_credentials,
    )

    async def _gather_tools(session: ClientSession) -> list[mcp_types.Tool]:
        console.rule(f"Gathering tools from {mcp_server.name}...")
        await session.initialize()
        all_tools: list[mcp_types.Tool] = []
        # list tools supports pagination, so we need to keep calling it until all tools are found
        next_cursor: mcp_types.Cursor | None = None
        while True:
            list_tools_result = await session.list_tools(cursor=next_cursor)
            next_cursor = list_tools_result.nextCursor
            console.print(f"Found {len(list_tools_result.tools)} tools, next cursor: {next_cursor}")
            all_tools.extend(list_tools_result.tools)
            if next_cursor is None:
                break

        console.print(f"Found total {len(all_tools)} tools")
        return all_tools

    match mcp_server.transport_type:
        case MCPServerTransportType.STREAMABLE_HTTP:
            async with streamablehttp_client(mcp_server.url, auth=mcp_auth_credentials_manager) as (
                read,
                write,
                _,
            ):
                async with ClientSession(read, write) as session:
                    return await _gather_tools(session)

        case MCPServerTransportType.SSE:
            async with sse_client(mcp_server.url, auth=mcp_auth_credentials_manager) as (
                read,
                write,
            ):
                async with ClientSession(read, write) as session:
                    return await _gather_tools(session)


def _create_tools_json_file(mcp_server_name: str, canonical_tools: list[mcp_types.Tool]) -> None:
    """Generate JSON schemas for function definitions and save to functions.json"""
    tools: list[dict] = []

    for canonical_tool in canonical_tools:
        try:
            sanitized = mcp_tool_utils.sanitize_canonical_name(canonical_tool.name)
        except MCPToolSanitizationError:
            console.print(
                f"[yellow]Warning: Tool name '{canonical_tool.name}' is empty after sanitization. Using 'UNKNOWN_TOOL' as placeholder. Need manual fix after generation.[/yellow]"  # noqa: E501
            )
            sanitized = "UNKNOWN_TOOL"

        tool_name = f"{mcp_server_name}__{sanitized}"

        console.print(f"Creating tool {tool_name} ({canonical_tool.name})")

        # Check description length
        if canonical_tool.description is not None and len(canonical_tool.description) > 1024:
            console.print(
                f"  └─ [yellow]Warning: Description for '{canonical_tool.name}' is {len(canonical_tool.description)} characters (exceeds 1024 limit).[/yellow]"  # noqa: E501
            )

        # Generate hashes for change detection
        description_hash = (
            ""
            if canonical_tool.description is None
            else mcp_tool_utils.normalize_and_hash_content(canonical_tool.description)
        )
        input_schema_hash = mcp_tool_utils.normalize_and_hash_content(canonical_tool.inputSchema)

        # Build function definition according to the specification
        tool = {
            "name": tool_name,
            "description": "" if canonical_tool.description is None else canonical_tool.description,
            "tags": [],
            "tool_metadata": {
                "canonical_tool_name": canonical_tool.name,
                # storing hashes so we can better detect changes from the original MCP server
                "canonical_tool_description_hash": description_hash,
                "canonical_tool_input_schema_hash": input_schema_hash,
            },
            "input_schema": canonical_tool.inputSchema,
        }

        tools.append(tool)

    # generate the tools.json file in current directory
    output_dir = Path("mcp_servers") / mcp_server_name.lower()
    # Create app folder if it doesn't exist
    if not output_dir.exists():
        output_dir.mkdir(parents=True, exist_ok=True)
        console.print(f"[blue]Created app directory: {output_dir}[/blue]")

    # append tools.json with datetime
    tools_file = output_dir / f"{datetime.now().strftime('%Y%m%d_%H%M%S')}_tools.json"
    console.print(f"Creating file {tools_file}")

    with open(tools_file, "w") as f:
        json.dump(tools, f, indent=2)

    console.print(f"[green]Created {tools_file} with {len(tools)} tools[/green]")


@click.command()
@click.option(
    "--mcp-server-name",
    "mcp_server_name",
    required=True,
    type=str,
    help="Name of the mcp server, e.g., 'NOTION'",
)
def generate_tools(mcp_server_name: str) -> None:
    """
    Generate the tools.json file for the given mcp server.
    Pre-requisites:
      - The mcp server must be inserted into the database.
      - One mcp server configuration must be created for the mcp server.
      - One connected account must be created for the mcp server configuration.
    """
    asyncio.run(_generate_tools_async(mcp_server_name))
