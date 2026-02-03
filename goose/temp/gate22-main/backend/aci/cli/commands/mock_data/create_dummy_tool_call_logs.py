from datetime import UTC, datetime, timedelta
from uuid import UUID, uuid4

import click
from rich.console import Console

from aci.cli import config
from aci.common import utils
from aci.common.db.sql_models import MCPToolCallLog
from aci.common.enums import MCPToolCallStatus

console = Console()


@click.command()
@click.option(
    "--count",
    "count",
    type=int,
    required=True,
    help="Number of dummy logs to insert",
)
@click.option(
    "--organization-id",
    "organization_id",
    type=click.UUID,
    required=True,
    help="Organization ID for the logs",
)
@click.option(
    "--user-id",
    "user_id",
    type=click.UUID,
    required=True,
    help="User ID for the logs",
)
@click.option(
    "--skip-dry-run",
    is_flag=True,
    help="Provide this flag to run the command and apply changes to the database",
)
def create_dummy_tool_call_logs(
    count: int,
    organization_id: UUID,
    user_id: UUID,
    skip_dry_run: bool,
) -> None:
    """
    Insert dummy MCPToolCallLog records into the database.
    """
    with utils.create_db_session(config.DB_FULL_URL) as db_session:
        current_time = datetime.now(UTC)
        logs = []
        for i in range(count):
            started_at = current_time - timedelta(seconds=i)
            ended_at = started_at + timedelta(seconds=1)
            duration_ms = 1000

            if i % 2 == 0:
                log = MCPToolCallLog(
                    organization_id=organization_id,
                    user_id=user_id,
                    request_id=str(uuid4()),
                    session_id=uuid4(),
                    bundle_name="notion",
                    bundle_id=uuid4(),
                    mcp_server_name=None,
                    mcp_server_id=None,
                    mcp_tool_name="GMAIL__SEND_MAIL",
                    mcp_tool_id=None,
                    mcp_server_configuration_name=None,
                    mcp_server_configuration_id=None,
                    arguments="{}",
                    result={
                        "code": -32602,
                        "message": f"Tool not found, tool_name=GMAIL__SEND_MAIL (dummy log {i + 1})",  # noqa: E501
                    },
                    status=MCPToolCallStatus.ERROR,
                    via_execute_tool=True,
                    jsonrpc_payload={
                        "id": 2,
                        "method": "tools/call",
                        "params": {
                            "meta": {"progressToken": 2},
                            "name": "EXECUTE_TOOL",
                            "arguments": {"tool_name": "GMAIL__SEND_MAIL", "tool_arguments": {}},
                        },
                        "jsonrpc": "2.0",
                    },
                    started_at=started_at,
                    ended_at=ended_at,
                    duration_ms=duration_ms,
                )
            else:  # SUCCESS
                log = MCPToolCallLog(
                    organization_id=organization_id,
                    user_id=user_id,
                    request_id=str(uuid4()),
                    session_id=uuid4(),
                    bundle_name="notion",
                    bundle_id=uuid4(),
                    mcp_server_name="NOTION",
                    mcp_server_id=uuid4(),
                    mcp_tool_name="NOTION__NOTION_SEARCH",
                    mcp_tool_id=uuid4(),
                    mcp_server_configuration_name="notion",
                    mcp_server_configuration_id=uuid4(),
                    arguments='{"query": "mcp"}',
                    result={
                        "content": [
                            {
                                "text": f"Dummy search result {i + 1} for MCP",
                                "type": "text",
                            }
                        ],
                        "isError": False,
                    },
                    status=MCPToolCallStatus.SUCCESS,
                    via_execute_tool=True,
                    jsonrpc_payload={
                        "id": 3,
                        "method": "tools/call",
                        "params": {
                            "meta": {"progressToken": 3},
                            "name": "EXECUTE_TOOL",
                            "arguments": {
                                "tool_name": "NOTION__NOTION_SEARCH",
                                "tool_arguments": {"query": "mcp"},
                            },
                        },
                        "jsonrpc": "2.0",
                    },
                    started_at=started_at,
                    ended_at=ended_at,
                    duration_ms=duration_ms,
                )

            logs.append(log)

        db_session.add_all(logs)

        if not skip_dry_run:
            console.rule(f"[bold yellow]DRY RUN[/bold yellow]: Would insert {count} dummy logs")
            console.print("[yellow]Provide --skip-dry-run to actually insert the logs[/yellow]")
            db_session.rollback()
        else:
            db_session.commit()
            console.rule("[bold green]Success![/bold green]")
            console.print(f"[green]Inserted {count} dummy tool call logs[/green]")
