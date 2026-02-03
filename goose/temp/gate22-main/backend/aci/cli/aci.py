import click

from aci.cli import config
from aci.cli.commands import adhoc, mcp, mock_data, subscription, virtual_mcp
from aci.common.logging_setup import setup_logging
from aci.common.openai_client import init_openai_client

setup_logging()


@click.group(context_settings={"help_option_names": ["-h", "--help"]})
def cli() -> None:
    pass


@cli.group(name="virtual-mcp")
def virtual_mcp_group() -> None:
    pass


@cli.group(name="mcp")
def mcp_group() -> None:
    pass


@cli.group(name="adhoc")
def adhoc_group() -> None:
    pass


@cli.group(name="subscription")
def subscription_group() -> None:
    pass


# Virtual MCP commands
virtual_mcp_group.add_command(virtual_mcp.upsert_server, name="upsert-server")
virtual_mcp_group.add_command(virtual_mcp.upsert_tools, name="upsert-tools")

# MCP commands
mcp_group.add_command(mcp.upsert_mcp_server, name="upsert-server")
mcp_group.add_command(mcp.upsert_mcp_tools, name="upsert-tools")
mcp_group.add_command(mcp.generate_tools, name="generate-tools")

# Subscription commands
subscription_group.add_command(subscription.insert_subscription_plan, name="insert-plans")

# Adhoc commands: commands that are one-off but keeping them in the CLI for convenience
adhoc_group.add_command(
    adhoc.convert_integrations_to_virtual_mcp, name="convert-integrations-to-virtual-mcp"
)
adhoc_group.add_command(adhoc.convert_integrations_to_mcp, name="convert-integrations-to-mcp")

# Other commands
# TODO: group these commands
cli.add_command(mock_data.create_mock_org_teams_users, name="create-mock-org-teams-users")
cli.add_command(mock_data.create_mock_mcp_configuration, name="create-mock-mcp-configuration")
cli.add_command(mock_data.create_dummy_tool_call_logs, name="create-dummy-tool-call-logs")

# Initialize the OpenAI client at the startup
init_openai_client(config.OPENAI_API_KEY)

if __name__ == "__main__":
    cli()
