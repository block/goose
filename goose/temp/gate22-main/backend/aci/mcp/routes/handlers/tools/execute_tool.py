import json
from datetime import UTC, datetime
from functools import wraps

from mcp import types as mcp_types
from mcp.client.session import ClientSession
from mcp.client.sse import sse_client
from mcp.shared import exceptions as mcp_exceptions
from pydantic import BaseModel, ConfigDict, Field
from sqlalchemy.orm import Session

from aci.common import auth_credentials_manager as acm
from aci.common.db import crud
from aci.common.db.sql_models import (
    MCPServer,
    MCPServerBundle,
    MCPServerConfiguration,
    MCPSession,
)
from aci.common.enums import MCPServerTransportType
from aci.common.logging_setup import get_logger
from aci.common.mcp_auth_manager import MCPAuthManager
from aci.common.schemas.mcp_auth import (
    AuthConfig,
    AuthCredentials,
)
from aci.common.schemas.mcp_tool import MCPToolMetadata
from aci.common.schemas.mcp_tool_call_log import MCPToolCallLogCreate, MCPToolCallStatus
from aci.mcp.context import request_id_ctx_var
from aci.mcp.protocol.streamable_http import streamablehttp_client_fork
from aci.mcp.routes.jsonrpc import (
    JSONRPCErrorCode,
    JSONRPCErrorResponse,
    JSONRPCSuccessResponse,
    JSONRPCToolsCallRequest,
)

logger = get_logger(__name__)


# TODO: add type hint
def track_duration(func):  # type: ignore
    """
    Decorator that tracks execution duration and adds it to MCPToolCallLogData.
    Expects the function to return a tuple where the second element is MCPToolCallLogData.
    """

    @wraps(func)
    async def wrapper(*args, **kwargs):  # type: ignore
        started_at = datetime.now(UTC)
        result = None
        try:
            result = await func(*args, **kwargs)
            return result
        finally:
            # Extract tool_call_log_data from the result tuple
            if result is not None and isinstance(result, tuple) and len(result) == 2:
                tool_call_log_data = result[1]
                if isinstance(tool_call_log_data, MCPToolCallLogCreate):
                    ended_at = datetime.now(UTC)
                    tool_call_log_data.duration_ms = int(
                        (ended_at - started_at).total_seconds() * 1000
                    )
                    tool_call_log_data.ended_at = ended_at
                    tool_call_log_data.started_at = started_at

    return wrapper


class ExecuteToolInputSchema(BaseModel):
    tool_name: str = Field(..., description="The name of the tool to execute")
    tool_arguments: dict = Field(
        ...,
        description="A dictionary containing key-value pairs of input parameters required by the "
        "specified tool. The parameter names and types must match those defined in "
        "the tool definition previously retrieved. If the tool requires no "
        "parameters, provide an empty object.",
    )

    model_config = ConfigDict(extra="forbid")


EXECUTE_TOOL = mcp_types.Tool(
    name="EXECUTE_TOOL",
    description="Execute a specific retrieved tool. Provide the executable tool name, and the "
    "required tool parameters for that tool based on tool definition retrieved.",
    inputSchema=ExecuteToolInputSchema.model_json_schema(),
)


# TODO: handle direct tool call that is not through the "EXECUTE_TOOL" tool
# (can happen due to LLM misbehavior)
# TODO: handle wrong input where tool arguments are under tool_arguments?
@track_duration
async def handle_execute_tool(
    db_session: Session,
    mcp_session: MCPSession,
    mcp_server_bundle: MCPServerBundle,
    jsonrpc_tools_call_request: JSONRPCToolsCallRequest,
) -> tuple[JSONRPCSuccessResponse | JSONRPCErrorResponse, MCPToolCallLogCreate]:
    # TODO: a better way to populate the tool_call_log_data
    tool_call_log_data = MCPToolCallLogCreate.model_construct()
    tool_call_log_data.organization_id = mcp_server_bundle.organization_id
    tool_call_log_data.user_id = mcp_server_bundle.user_id
    tool_call_log_data.request_id = request_id_ctx_var.get()
    tool_call_log_data.session_id = mcp_session.id
    tool_call_log_data.bundle_name = mcp_server_bundle.name
    tool_call_log_data.bundle_id = mcp_server_bundle.id
    tool_call_log_data.via_execute_tool = True
    tool_call_log_data.jsonrpc_payload = jsonrpc_tools_call_request.model_dump()
    # validate input
    try:
        validated_input = ExecuteToolInputSchema.model_validate(
            jsonrpc_tools_call_request.params.arguments
        )
        tool_name = validated_input.tool_name
        tool_arguments = validated_input.tool_arguments
        tool_call_log_data.mcp_tool_name = tool_name
        tool_call_log_data.arguments = json.dumps(tool_arguments)
    except Exception as e:
        logger.exception(f"Error validating execute tool input: {e}")
        # try to parse the tool name from the arguments even if it's malformed
        tool_name_raw = jsonrpc_tools_call_request.params.arguments.get("tool_name")
        if tool_name_raw is not None:
            tool_call_log_data.mcp_tool_name = (
                tool_name_raw if isinstance(tool_name_raw, str) else json.dumps(tool_name_raw)
            )
        tool_arguments_raw = jsonrpc_tools_call_request.params.arguments.get("tool_arguments")
        if tool_arguments_raw is not None:
            tool_call_log_data.arguments = json.dumps(tool_arguments_raw)

        errorData = JSONRPCErrorResponse.ErrorData(
            code=JSONRPCErrorCode.INVALID_METHOD_PARAMS,
            message=str(e),
        )
        tool_call_log_data.result = errorData.model_dump(exclude_none=True)
        tool_call_log_data.status = MCPToolCallStatus.ERROR
        return (
            JSONRPCErrorResponse(
                id=jsonrpc_tools_call_request.id,
                error=errorData,
            ),
            tool_call_log_data,
        )

    # validate resource existence and access permissions
    try:
        mcp_tool = crud.mcp_tools.get_mcp_tool_by_name(db_session, tool_name, False)
        if mcp_tool is None:
            logger.warning(f"Tool not found, tool_name={tool_name}")
            errorData = JSONRPCErrorResponse.ErrorData(
                code=JSONRPCErrorCode.INVALID_METHOD_PARAMS,
                message=f"Tool not found, tool_name={tool_name}",
            )
            tool_call_log_data.result = errorData.model_dump(exclude_none=True)
            tool_call_log_data.status = MCPToolCallStatus.ERROR
            return (
                JSONRPCErrorResponse(
                    id=jsonrpc_tools_call_request.id,
                    error=errorData,
                ),
                tool_call_log_data,
            )
        mcp_server = mcp_tool.mcp_server
        mcp_server_configuration: MCPServerConfiguration | None = None

        tool_call_log_data.mcp_server_name = mcp_server.name
        tool_call_log_data.mcp_server_id = mcp_server.id
        tool_call_log_data.mcp_tool_id = mcp_tool.id
        # find the mcp server configuration (from the mcp server bundle's configuration list) that
        # is associated with the mcp server
        # TODO: abstract this logic to a service layer function
        for mcp_server_configuration_id in mcp_server_bundle.mcp_server_configuration_ids:
            mcp_server_configuration_candidate = (
                crud.mcp_server_configurations.get_mcp_server_configuration_by_id(
                    db_session,
                    mcp_server_configuration_id,
                    False,
                )
            )
            if mcp_server_configuration_candidate is None:
                logger.error(
                    f"MCP server configuration not found even though the id is part of the bundle, mcp_server_configuration_id={mcp_server_configuration_id}"  # noqa: E501
                )
                continue
            # TODO: this is under the assumption that the mcp server configuration is unique
            # per mcp server
            if mcp_server_configuration_candidate.mcp_server_id == mcp_server.id:
                mcp_server_configuration = mcp_server_configuration_candidate
                break

        if mcp_server_configuration is None:
            logger.error(
                f"MCP server not configured, mcp_server_id={mcp_server.id}, mcp_server_name={mcp_server.name}"  # noqa: E501
            )
            errorData = JSONRPCErrorResponse.ErrorData(
                code=JSONRPCErrorCode.INVALID_REQUEST,
                message=f"MCP server not configured, mcp_server_name={mcp_server.name}",
            )
            tool_call_log_data.result = errorData.model_dump(exclude_none=True)
            tool_call_log_data.status = MCPToolCallStatus.ERROR
            return (
                JSONRPCErrorResponse(
                    id=jsonrpc_tools_call_request.id,
                    error=errorData,
                ),
                tool_call_log_data,
            )

        tool_call_log_data.mcp_server_configuration_name = mcp_server_configuration.name
        tool_call_log_data.mcp_server_configuration_id = mcp_server_configuration.id

        # check if this tool is enabled in the mcp server configuration
        # TODO: test
        if (
            not mcp_server_configuration.all_tools_enabled
            and mcp_tool.id not in mcp_server_configuration.enabled_tools
        ):
            logger.warning(
                f"MCP tool not enabled in mcp server configuration, mcp_tool_id={mcp_tool.id}, mcp_tool_name={mcp_tool.name}"  # noqa: E501
            )
            errorData = JSONRPCErrorResponse.ErrorData(
                code=JSONRPCErrorCode.UNAUTHORIZED,
                message=f"MCP tool not enabled in mcp server configuration, mcp_tool_name={mcp_tool.name}",  # noqa: E501
            )
            tool_call_log_data.result = errorData.model_dump(exclude_none=True)
            tool_call_log_data.status = MCPToolCallStatus.ERROR

            return (
                JSONRPCErrorResponse(
                    id=jsonrpc_tools_call_request.id,
                    error=errorData,
                ),
                tool_call_log_data,
            )
    except Exception as e:
        logger.exception(f"Unexpected error checking tool call resources: {e}")
        errorData = JSONRPCErrorResponse.ErrorData(
            code=JSONRPCErrorCode.INTERNAL_ERROR,
            message="Internal error",
        )
        tool_call_log_data.result = errorData.model_dump(exclude_none=True)
        tool_call_log_data.status = MCPToolCallStatus.ERROR
        return (
            JSONRPCErrorResponse(
                id=jsonrpc_tools_call_request.id,
                error=errorData,
            ),
            tool_call_log_data,
        )

    # get auth config and credentials
    try:
        auth_config = acm.get_auth_config(mcp_server, mcp_server_configuration)
        auth_credentials = await acm.get_auth_credentials(
            db_session,
            mcp_server_configuration.id,
            mcp_server_configuration.connected_account_ownership,
            user_id=mcp_server_bundle.user_id,
        )
    except Exception as e:
        logger.exception(f"Error getting auth config and credentials: {e}")
        errorData = JSONRPCErrorResponse.ErrorData(
            code=JSONRPCErrorCode.INTERNAL_ERROR,
            message="Internal error",
        )
        tool_call_log_data.result = errorData.model_dump(exclude_none=True)
        tool_call_log_data.status = MCPToolCallStatus.ERROR
        return (
            JSONRPCErrorResponse(
                id=jsonrpc_tools_call_request.id,
                error=errorData,
            ),
            tool_call_log_data,
        )

    # forward tool call to the corresponding mcp server
    try:
        result = await _forward_tool_call(
            name=MCPToolMetadata.model_validate(mcp_tool.tool_metadata).canonical_tool_name,
            arguments=tool_arguments,
            db_session=db_session,
            mcp_session=mcp_session,
            mcp_server=mcp_server,
            auth_config=auth_config,
            auth_credentials=auth_credentials,
        )
        # TODO: should we differentiate tool call error from external MCPs v.s the tool call
        # (SEARCH_TOOLS and EXECUTE_TOOL) error from the unified MCP service itself?
        # e.g., still return JSONRPCSuccessResponse for external tool call error?
        if isinstance(result, mcp_exceptions.McpError):
            errorData = JSONRPCErrorResponse.ErrorData(
                code=result.error.code,
                message=result.error.message,
                data=result.error.data,
            )
            tool_call_log_data.result = errorData.model_dump(exclude_none=True)
            tool_call_log_data.status = MCPToolCallStatus.ERROR
            return (
                JSONRPCErrorResponse(
                    # NOTE: JSONRPCErrorResponse.ErrorData is different class from
                    # mcp.types.ErrorData so we assign the error data manually
                    id=jsonrpc_tools_call_request.id,
                    error=errorData,
                ),
                tool_call_log_data,
            )
        else:
            tool_call_log_data.result = result.model_dump(exclude_none=True)
            tool_call_log_data.status = MCPToolCallStatus.SUCCESS
            return (
                JSONRPCSuccessResponse(
                    id=jsonrpc_tools_call_request.id,
                    result=result.model_dump(exclude_none=True),
                ),
                tool_call_log_data,
            )
    except Exception as e:
        logger.exception(f"Error forwarding tool call: {e}")
        errorData = JSONRPCErrorResponse.ErrorData(
            code=JSONRPCErrorCode.INTERNAL_ERROR,
            message="Unknown error forwarding tool call",
        )
        tool_call_log_data.result = errorData.model_dump(exclude_none=True)
        tool_call_log_data.status = MCPToolCallStatus.ERROR
        return (
            JSONRPCErrorResponse(
                id=jsonrpc_tools_call_request.id,
                error=errorData,
            ),
            tool_call_log_data,
        )


async def _forward_tool_call(
    name: str,
    arguments: dict,
    db_session: Session,
    mcp_session: MCPSession,
    mcp_server: MCPServer,
    auth_config: AuthConfig,
    auth_credentials: AuthCredentials,
) -> mcp_types.CallToolResult | mcp_exceptions.McpError:
    # TODO: use the correct auth type
    mcp_auth_credentials_manager = MCPAuthManager(
        mcp_server=mcp_server,
        auth_config=auth_config,
        auth_credentials=auth_credentials,
    )
    existing_mcp_session_id = mcp_session.external_mcp_sessions.get(str(mcp_server.id))

    match mcp_server.transport_type:
        case MCPServerTransportType.STREAMABLE_HTTP:
            async with streamablehttp_client_fork(
                mcp_server.url,
                session_id=existing_mcp_session_id,
                auth=mcp_auth_credentials_manager,
                # NOTE: we don't want to terminate the session when the tool call returns
                terminate_on_close=False,
            ) as (
                read,
                write,
                get_session_id,
            ):
                async with ClientSession(read, write) as client_session:
                    tool_call_result = await _call_tool(
                        client_session, existing_mcp_session_id is None, name, arguments
                    )
                    new_mcp_session_id = get_session_id()
                    if (
                        new_mcp_session_id is not None
                        and new_mcp_session_id != existing_mcp_session_id
                    ):
                        logger.debug("New MCP session id")
                        crud.mcp_sessions.update_session_external_mcp_session(
                            db_session, mcp_session, mcp_server.id, new_mcp_session_id
                        )
                        db_session.commit()
                    return tool_call_result

        case MCPServerTransportType.SSE:
            async with sse_client(mcp_server.url, auth=mcp_auth_credentials_manager) as (
                read,
                write,
            ):
                async with ClientSession(read, write) as client_session:
                    return await _call_tool(
                        client_session, existing_mcp_session_id is None, name, arguments
                    )


async def _call_tool(
    client_session: ClientSession, need_initialize: bool, name: str, arguments: dict
) -> mcp_types.CallToolResult | mcp_exceptions.McpError:
    """
    Initialize the session (if needed) and call a tool on the mcp server.
    NOTE: here we return the mcp error as response because the async taskgroup
    will wrap the exception under the exception group.
    """
    if need_initialize:
        try:
            await client_session.initialize()
        except mcp_exceptions.McpError as e:
            logger.exception(f"Initialize failed, error={e}")
            return e
        # TODO: will it throw other errors that is not McpError? httpx.HTTPStatusError?
        # raised by _handle_post_request of StreamableHTTPTransport?
        try:
            return await client_session.call_tool(name=name, arguments=arguments)
        except mcp_exceptions.McpError as e:
            logger.exception(f"tool call failed, tool={name}, arguments={arguments}, error={e}")
            return e

    else:
        try:
            return await client_session.call_tool(name=name, arguments=arguments)
        except mcp_exceptions.McpError as e:
            # If it's a session terminated error, try to reinitialize the session and
            # call the tool again.
            # TODO: this is based on _send_session_terminated_error method from
            # StreamableHTTPTransport class of mcp python sdk. This approach is not robust as
            # the error code and message might change in the future.
            # We can write test case to make sure this assumption stands. Otherwise we can
            # fork the StreamableHTTPTransport class.
            if e.error.code == 32600 and e.error.message == "Session terminated":
                logger.warning(
                    "Session terminated error, reinitializing session and calling tool again"
                )
                try:
                    await client_session.initialize()
                except mcp_exceptions.McpError as init_error:
                    logger.exception(
                        f"Initialize failed after session terminated error, error={init_error}"
                    )
                    return init_error

                try:
                    return await client_session.call_tool(name=name, arguments=arguments)
                except mcp_exceptions.McpError as retry_error:
                    logger.exception(
                        f"tool call failed, tool={name}, arguments={arguments}, error={retry_error}"
                    )
                    return retry_error
            else:
                logger.exception(f"tool call failed, tool={name}, arguments={arguments}, error={e}")
                return e
