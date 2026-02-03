from fastapi import Response
from mcp import types as mcp_types
from mcp.client.session import ClientSession
from mcp.client.streamable_http import streamablehttp_client
from sqlalchemy.orm import Session

from aci.common import auth_credentials_manager as acm
from aci.common.db import crud
from aci.common.db.sql_models import MCPServerBundle, MCPServerConfiguration, MCPSession
from aci.common.enums import MCPServerTransportType
from aci.common.logging_setup import get_logger
from aci.common.mcp_auth_manager import MCPAuthManager
from aci.mcp import config
from aci.mcp.routes.handlers.tools.execute_tool import EXECUTE_TOOL
from aci.mcp.routes.handlers.tools.search_tools import SEARCH_TOOLS
from aci.mcp.routes.jsonrpc import (
    JSONRPCInitializeRequest,
    JSONRPCSuccessResponse,
)

logger = get_logger(__name__)


async def handle_initialize(
    db_session: Session,
    mcp_session: MCPSession,
    response: Response,
    mcp_server_bundle: MCPServerBundle,
    payload: JSONRPCInitializeRequest,
) -> JSONRPCSuccessResponse:
    """
    Handle the initialize request from mcp clients.
    """
    external_mcp_sessions = await _initialize_external_mcp_sessions(db_session, mcp_server_bundle)
    crud.mcp_sessions.update_session_external_mcp_sessions(
        db_session, mcp_session, external_mcp_sessions
    )
    db_session.commit()

    # NOTE: need to set the session id in the response headers
    response.headers[config.MCP_SESSION_ID_HEADER] = str(mcp_session.id)

    return JSONRPCSuccessResponse(
        id=payload.id,
        result=mcp_types.InitializeResult(
            protocolVersion=payload.params.protocol_version
            if payload.params.protocol_version
            else mcp_types.LATEST_PROTOCOL_VERSION,
            # NOTE: for now we don't support tools list changed
            capabilities=mcp_types.ServerCapabilities(
                tools=mcp_types.ToolsCapability(listChanged=False)
            ),
            serverInfo=mcp_types.Implementation(
                name="ACI.dev MCP Gateway", title="ACI.dev MCP Gateway", version="0.0.1"
            ),
            # TODO: add instructions (maybe dynamically based on what mcp servers are available for the bundle) # noqa: E501
            instructions=f"use {SEARCH_TOOLS.name} and {EXECUTE_TOOL.name} to discover and execute tools",  # noqa: E501
        ).model_dump(exclude_none=True),
    )


async def _initialize_external_mcp_sessions(
    db_session: Session, mcp_server_bundle: MCPServerBundle
) -> dict[str, str]:
    """
    For every mcp server in the bundle, initialize the session.
    NOTE: not every mcp server requires and/or supports sessions
    """

    external_mcp_sessions: dict[str, str] = {}
    mcp_server_configurations: list[MCPServerConfiguration] = []
    for mcp_server_configuration_id in mcp_server_bundle.mcp_server_configuration_ids:
        mcp_server_configuration = (
            crud.mcp_server_configurations.get_mcp_server_configuration_by_id(
                db_session,
                mcp_server_configuration_id,
                False,
            )
        )
        if mcp_server_configuration is None:
            # NOTE:This can happen if the resource cleanup is not handled properly or in some race
            # conditions. Don't block the entire process if some of the MCP server configurations
            # are not found.
            logger.error(
                f"MCP server configuration not found, "
                f"mcp_server_configuration_id={mcp_server_configuration_id}, "
                f"bundle_id={mcp_server_bundle.id}"
            )
            continue
        mcp_server_configurations.append(mcp_server_configuration)

    for mcp_server_configuration in mcp_server_configurations:
        session_id = await _initialize_external_mcp_session(
            db_session,
            mcp_server_configuration,
            mcp_server_bundle,
        )
        if session_id:
            external_mcp_sessions[str(mcp_server_configuration.mcp_server_id)] = session_id

    return external_mcp_sessions


async def _initialize_external_mcp_session(
    db_session: Session,
    mcp_server_configuration: MCPServerConfiguration,
    mcp_server_bundle: MCPServerBundle,
) -> str | None:
    """
    Initialize the session for an MCP server.
    Return None if:
        - the session cannot be initialized due to an error
        - the session cannot be initialized because the mcp server does not support sessions
    """
    mcp_server = mcp_server_configuration.mcp_server

    try:
        auth_config = acm.get_auth_config(mcp_server, mcp_server_configuration)
        auth_credentials = await acm.get_auth_credentials(
            db_session,
            mcp_server_configuration.id,
            mcp_server_configuration.connected_account_ownership,
            user_id=mcp_server_bundle.user_id,
        )
    except Exception as e:
        logger.warning(
            f"failed to get auth credentials, "
            f"mcp_server_name={mcp_server.name}, "
            f"bundle_id={mcp_server_bundle.id}, "
            f"error={e}"
        )
        return None

    mcp_auth_credentials_manager = MCPAuthManager(
        mcp_server=mcp_server,
        auth_config=auth_config,
        auth_credentials=auth_credentials,
    )
    match mcp_server.transport_type:
        case MCPServerTransportType.STREAMABLE_HTTP:
            # NOTE: it's important to set terminate_on_close=False here to avoid the session being
            # terminated when this call returns.
            async with streamablehttp_client(
                mcp_server.url, auth=mcp_auth_credentials_manager, terminate_on_close=False
            ) as (
                read,
                write,
                get_session_id,
            ):
                async with ClientSession(read, write) as session:
                    try:
                        await session.initialize()
                        session_id = get_session_id()
                        logger.debug(
                            f"Initialized external mcp session, mcp_server_name={mcp_server.name}, bundle_id={mcp_server_bundle.id}, session_id={session_id}"  # noqa: E501
                        )
                        return session_id
                    except Exception as e:
                        logger.warning(
                            f"failed to initialize external mcp session, mcp_server_name={mcp_server.name}, bundle_id={mcp_server_bundle.id}, error={e}"  # noqa: E501
                        )
                        return None
        case MCPServerTransportType.SSE:
            # NOTE: assume all sse based mcp servers do not support sessions
            return None
