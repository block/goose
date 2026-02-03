import json
from typing import Literal
from unittest.mock import MagicMock
from uuid import uuid4

import pytest
from mcp import types as mcp_types
from pytest_mock import MockerFixture
from sqlalchemy.orm import Session

from aci.common.db.sql_models import (
    MCPServer,
    MCPServerBundle,
    MCPServerConfiguration,
    MCPSession,
    MCPTool,
)
from aci.common.enums import MCPToolCallStatus
from aci.common.schemas.mcp_auth import AuthConfig, AuthCredentials
from aci.mcp.routes.handlers.tools.execute_tool import handle_execute_tool
from aci.mcp.routes.jsonrpc import (
    JSONRPCSuccessResponse,
    JSONRPCToolsCallRequest,
)

# Mock UUIDs
MOCK_MCP_TOOL_UUID = uuid4()
MOCK_MCP_SERVER_UUID = uuid4()
MOCK_MCP_SERVER_CONFIGURATION_UUID = uuid4()
MOCK_MCP_SESSION_UUID = uuid4()
MOCK_MCP_SERVER_BUNDLE_UUID = uuid4()
MOCK_USER_UUID = uuid4()
MOCK_ORGANIZATION_UUID = uuid4()


# Fixtures
@pytest.fixture
def mock_mcp_tool() -> MagicMock:
    """Fixture for mock MCP tool."""
    tool = MagicMock(spec=MCPTool)
    tool.id = MOCK_MCP_TOOL_UUID
    tool.mcp_server_id = MOCK_MCP_SERVER_UUID
    tool.name = "MOCK__TEST_TOOL"
    tool.tool_metadata = {
        "canonical_tool_name": "test_tool",
        "canonical_tool_description_hash": "test_tool_description_hash",
        "canonical_tool_input_schema_hash": "test_tool_input_schema_hash",
    }
    tool.mcp_server = MagicMock(spec=MCPServer)
    tool.mcp_server.id = MOCK_MCP_SERVER_UUID
    tool.mcp_server.name = "MOCK"
    return tool


@pytest.fixture
def mock_server_configuration() -> MagicMock:
    """Fixture for mock MCP server configuration."""
    config = MagicMock(spec=MCPServerConfiguration)
    config.id = MOCK_MCP_SERVER_CONFIGURATION_UUID
    config.mcp_server_id = MOCK_MCP_SERVER_UUID
    config.name = "mock_mcp_server_configuration"
    config.all_tools_enabled = True
    config.enabled_tools = []
    config.connected_account_ownership = "INDIVIDUAL"
    return config


@pytest.fixture
def mock_mcp_session() -> MagicMock:
    """Fixture for mock MCP session."""
    session = MagicMock(spec=MCPSession)
    session.id = MOCK_MCP_SESSION_UUID
    return session


@pytest.fixture
def mock_mcp_server_bundle() -> MagicMock:
    """Fixture for mock MCP server bundle."""
    bundle = MagicMock(spec=MCPServerBundle)
    bundle.id = MOCK_MCP_SERVER_BUNDLE_UUID
    bundle.organization_id = MOCK_ORGANIZATION_UUID
    bundle.user_id = MOCK_USER_UUID
    bundle.name = "mock_mcp_server_bundle"
    bundle.mcp_server_configuration_ids = [MOCK_MCP_SERVER_CONFIGURATION_UUID]
    return bundle


@pytest.fixture
def mock_auth_config() -> MagicMock:
    """Fixture for mock auth config."""
    return MagicMock(spec=AuthConfig)


@pytest.fixture
def mock_auth_credentials() -> MagicMock:
    """Fixture for mock auth credentials."""
    return MagicMock(spec=AuthCredentials)


# TODO: add more test cases: tool not exist, tool not enabled, etc. Might have to use real db
# instead of mock
@pytest.mark.parametrize(
    "tool_name,tool_arguments,case",
    [
        ("MOCK__TEST_TOOL", {"test_argument": "test_value"}, "success"),
    ],
)
@pytest.mark.asyncio
async def test_tool_call_log_data(
    tool_name: str,
    tool_arguments: dict,
    case: Literal["success", "non_existing_tool"],
    mocker: MockerFixture,
    db_session: Session,
    mock_mcp_tool: MagicMock,
    mock_server_configuration: MagicMock,
    mock_mcp_session: MagicMock,
    mock_mcp_server_bundle: MagicMock,
    mock_auth_config: MagicMock,
    mock_auth_credentials: MagicMock,
) -> None:
    # Mock request_id_ctx_var
    mock_request_id_ctx_var = mocker.patch(
        "aci.mcp.routes.handlers.tools.execute_tool.request_id_ctx_var"
    )
    mock_request_id_ctx_var.get.return_value = "test-request-id"

    # Mock get_mcp_tool_by_name
    mock_get_mcp_tool_by_name = mocker.patch("aci.common.db.crud.mcp_tools.get_mcp_tool_by_name")
    mock_get_mcp_tool_by_name.return_value = mock_mcp_tool

    # Mock get_mcp_server_configuration_by_id
    mock_get_mcp_server_configuration_by_id = mocker.patch(
        "aci.common.db.crud.mcp_server_configurations.get_mcp_server_configuration_by_id"
    )
    mock_get_mcp_server_configuration_by_id.return_value = mock_server_configuration

    # Mock get_auth_config
    mock_get_auth_config = mocker.patch("aci.common.auth_credentials_manager.get_auth_config")
    mock_get_auth_config.return_value = mock_auth_config

    # Mock get_auth_credentials
    mock_get_auth_credentials = mocker.patch(
        "aci.common.auth_credentials_manager.get_auth_credentials",
    )
    mock_get_auth_credentials.return_value = mock_auth_credentials

    # Mock _forward_tool_call
    mock_forward_tool_call = mocker.patch(
        "aci.mcp.routes.handlers.tools.execute_tool._forward_tool_call"
    )
    mock_forward_tool_call_result = mcp_types.CallToolResult(
        content=[mcp_types.TextContent(type="text", text="Success")]
    )
    mock_forward_tool_call.return_value = mock_forward_tool_call_result

    # Create actual jsonrpc_request object because we need to use model_dump() later
    mock_jsonrpc_request = JSONRPCToolsCallRequest(
        id="1",
        method="tools/call",
        params=JSONRPCToolsCallRequest.CallToolRequestParams(
            name="EXECUTE_TOOL",
            arguments={
                "tool_name": tool_name,
                "tool_arguments": tool_arguments,
            },
        ),
    )

    response, tool_call_log_create = await handle_execute_tool(
        db_session,
        mock_mcp_session,
        mock_mcp_server_bundle,
        mock_jsonrpc_request,
    )

    # Verify success response
    assert isinstance(response, JSONRPCSuccessResponse)

    try:
        tool_call_log_create.model_validate(tool_call_log_create.model_dump())
    except Exception as e:
        pytest.fail(f"Error validating tool call log create: {e}")

    assert tool_call_log_create.organization_id == MOCK_ORGANIZATION_UUID
    assert tool_call_log_create.user_id == MOCK_USER_UUID
    assert tool_call_log_create.request_id == "test-request-id"
    assert tool_call_log_create.session_id == MOCK_MCP_SESSION_UUID
    assert tool_call_log_create.bundle_name == "mock_mcp_server_bundle"
    assert tool_call_log_create.bundle_id == MOCK_MCP_SERVER_BUNDLE_UUID
    assert tool_call_log_create.via_execute_tool is True
    assert tool_call_log_create.jsonrpc_payload == mock_jsonrpc_request.model_dump()
    assert tool_call_log_create.mcp_server_name == "MOCK"
    assert tool_call_log_create.mcp_tool_name == tool_name
    assert tool_call_log_create.mcp_tool_id == MOCK_MCP_TOOL_UUID
    assert tool_call_log_create.mcp_server_configuration_name == "mock_mcp_server_configuration"
    assert tool_call_log_create.mcp_server_configuration_id == MOCK_MCP_SERVER_CONFIGURATION_UUID
    assert json.loads(tool_call_log_create.arguments) == tool_arguments
    assert tool_call_log_create.result == mock_forward_tool_call_result.model_dump(
        exclude_none=True
    )
    assert tool_call_log_create.status == MCPToolCallStatus.SUCCESS
    assert tool_call_log_create.duration_ms >= 0
