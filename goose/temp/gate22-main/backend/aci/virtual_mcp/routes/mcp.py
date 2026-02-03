from typing import Annotated

from fastapi import APIRouter, Depends, Header, Request, Response, status
from pydantic import ValidationError
from sqlalchemy.orm import Session

from aci.common.enums import HttpLocation
from aci.common.logging_setup import get_logger
from aci.common.schemas.virtual_mcp import VirtualMCPAuthTokenData
from aci.virtual_mcp import dependencies as deps
from aci.virtual_mcp.exceptions import (
    InvalidAuthTokenError,
    InvalidJSONRPCPayloadError,
    UnsupportedJSONRPCMethodError,
)
from aci.virtual_mcp.routes import handlers
from aci.virtual_mcp.routes.jsonrpc import (
    JSONRPCErrorCode,
    JSONRPCErrorResponse,
    JSONRPCInitializeRequest,
    JSONRPCNotificationInitialized,
    JSONRPCPayload,
    JSONRPCPingRequest,
    JSONRPCSuccessResponse,
    JSONRPCToolsCallRequest,
    JSONRPCToolsListRequest,
)

logger = get_logger(__name__)
router = APIRouter()

SUPPORTED_PROTOCOL_VERSION = "2025-06-18"


@router.post("", status_code=status.HTTP_200_OK)
async def mcp_post(
    db_session: Annotated[Session, Depends(deps.yield_db_session)],
    request: Request,
    response: Response,
    server_name: str,
    x_virtual_mcp_auth_token: Annotated[str | None, Header()] = None,
    mcp_protocol_version: Annotated[str | None, Header()] = None,
) -> JSONRPCSuccessResponse | JSONRPCErrorResponse | None:
    # parse auth token (agreed auth format with ACI's mcp service)
    auth_token_data = (
        _parse_auth_token(x_virtual_mcp_auth_token) if x_virtual_mcp_auth_token else None
    )

    # parse payload
    try:
        payload = await _parse_payload(request)
    except UnsupportedJSONRPCMethodError as e:
        logger.error(str(e))
        return JSONRPCErrorResponse(
            id=e.id,
            error=JSONRPCErrorResponse.ErrorData(
                code=JSONRPCErrorCode.METHOD_NOT_FOUND,
                message=str(e),
            ),
        )
    except InvalidJSONRPCPayloadError as e:
        logger.error(str(e))
        return JSONRPCErrorResponse(
            id=e.id,
            error=JSONRPCErrorResponse.ErrorData(
                code=JSONRPCErrorCode.INVALID_REQUEST,
                message=str(e),
            ),
        )
    except Exception as e:
        logger.exception(f"Unexpected error parsing payload: {e}")
        return JSONRPCErrorResponse(
            # In jsonrpc, id is required in response, if unable to to detect, it must be Null
            id=None,
            error=JSONRPCErrorResponse.ErrorData(
                code=JSONRPCErrorCode.PARSE_ERROR,
                message=str(e),
            ),
        )

    # handle different requests
    match payload:
        case JSONRPCInitializeRequest():
            logger.info(f"Received initialize request={payload.model_dump()}")
            return JSONRPCSuccessResponse(
                id=payload.id,
                result={
                    "protocolVersion": SUPPORTED_PROTOCOL_VERSION
                    if mcp_protocol_version is None
                    else mcp_protocol_version,
                    "capabilities": {"tools": {}},
                    "serverInfo": {
                        "name": f"ACI.dev {server_name} MCP",
                        "title": f"ACI.dev {server_name} MCP",
                        "version": "0.0.1",
                    },
                    # TODO: add instructions (such as server's description?)
                    "instructions": "",
                },
            )

        case JSONRPCToolsListRequest():
            logger.info(f"Received tools/list request={payload.model_dump()}")
            return await handlers.handle_tools_list(payload, db_session, server_name)

        case JSONRPCToolsCallRequest():
            logger.info(f"Received tools/call request={payload.model_dump()}")
            return await handlers.handle_tools_call(
                payload, db_session, server_name, auth_token_data
            )

        case JSONRPCNotificationInitialized():
            # NOTE: no-op for initialized notifications
            # NOTE: it's important to return 202 otherwise some clients will not handle the response
            logger.info(f"Received initialized notification={payload.model_dump()}")
            response.status_code = status.HTTP_202_ACCEPTED
            return None

        case JSONRPCPingRequest():
            logger.info(f"Received ping request={payload.model_dump()}")
            return JSONRPCSuccessResponse(
                id=payload.id,
                result={},
            )


@router.delete("", status_code=status.HTTP_204_NO_CONTENT)
async def mcp_delete() -> None:
    """
    NOTE: delete is a no-op for now.
    """
    pass


# NOTE: for now we don't support sse stream feature so for GET return 405
@router.get("", status_code=status.HTTP_405_METHOD_NOT_ALLOWED)
async def mcp_get() -> None:
    """
    NOTE: get is not allowed for now.
    """
    pass


# TODO: move it to dependencies.py?
def _parse_auth_token(x_virtual_mcp_auth_token: str) -> VirtualMCPAuthTokenData:
    """
    parse the auth token from the header
    e.g.,
    "header Authorization Bearer 1234567890" -> VirtualMCPAuthTokenData(
        location="header",
        name="Authorization",
        prefix="Bearer",
        token="1234567890"
    )
    "query api_key 1234567890" -> VirtualMCPAuthTokenData(
        location="query",
        name="api_key",
        token="1234567890"
    )
    """
    # Strip leading/trailing whitespace and split by any amount of whitespace
    token_data = x_virtual_mcp_auth_token.strip().split()
    if len(token_data) != 3 and len(token_data) != 4:
        raise InvalidAuthTokenError()

    try:
        location = HttpLocation(token_data[0])
    except ValueError as e:
        logger.error(f"Invalid auth token location: {token_data[0]}")
        raise InvalidAuthTokenError() from e

    return VirtualMCPAuthTokenData(
        location=location,
        name=token_data[1],
        prefix=token_data[2] if len(token_data) == 4 else None,
        token=token_data[3] if len(token_data) == 4 else token_data[2],
    )


# TODO: duplicate code with mcp.py
async def _parse_payload(
    request: Request,
) -> (
    JSONRPCInitializeRequest
    | JSONRPCToolsListRequest
    | JSONRPCToolsCallRequest
    | JSONRPCNotificationInitialized
    | JSONRPCPingRequest
):
    payload = await request.json()
    if isinstance(payload, dict):
        try:
            jprc_payload = JSONRPCPayload.model_validate(payload)
        except ValidationError as e:
            raise InvalidJSONRPCPayloadError(f"Invalid payload: {e}", payload.get("id")) from e

        match jprc_payload.method:
            case "initialize":
                try:
                    return JSONRPCInitializeRequest.model_validate(payload)
                except ValidationError as e:
                    raise InvalidJSONRPCPayloadError(
                        f"Invalid initialize request: {e}", jprc_payload.id
                    ) from e
            case "tools/list":
                try:
                    return JSONRPCToolsListRequest.model_validate(payload)
                except ValidationError as e:
                    raise InvalidJSONRPCPayloadError(
                        f"Invalid tools/list request: {e}", jprc_payload.id
                    ) from e
            case "tools/call":
                try:
                    return JSONRPCToolsCallRequest.model_validate(payload)
                except ValidationError as e:
                    raise InvalidJSONRPCPayloadError(
                        f"Invalid tools/call request: {e}", jprc_payload.id
                    ) from e
            case "notifications/initialized":
                try:
                    return JSONRPCNotificationInitialized.model_validate(payload)
                except ValidationError as e:
                    raise InvalidJSONRPCPayloadError(
                        f"Invalid notifications/initialized request: {e}", jprc_payload.id
                    ) from e
            case "ping":
                try:
                    return JSONRPCPingRequest.model_validate(payload)
                except ValidationError as e:
                    raise InvalidJSONRPCPayloadError(
                        f"Invalid ping request: {e}", jprc_payload.id
                    ) from e
            case _:
                raise UnsupportedJSONRPCMethodError(jprc_payload.method, jprc_payload.id)
    else:
        raise InvalidJSONRPCPayloadError(f"Invalid payload type: {type(payload)}")
