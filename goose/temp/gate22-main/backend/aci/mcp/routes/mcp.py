from datetime import UTC, datetime
from typing import Annotated
from uuid import UUID

from fastapi import APIRouter, Depends, Request, Response, status
from pydantic import ValidationError
from sqlalchemy.orm import Session

from aci.common import entitlement_utils, utils
from aci.common.db import crud
from aci.common.db.sql_models import MCPServerBundle, MCPSession
from aci.common.logging_setup import get_logger
from aci.mcp import config
from aci.mcp import dependencies as deps
from aci.mcp.exceptions import (
    InvalidJSONRPCPayloadError,
    UnsupportedJSONRPCMethodError,
    UsageExceeded,
)
from aci.mcp.routes import handlers
from aci.mcp.routes.jsonrpc import (
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


# TODO: this is not a pure jsonrpc endpoint
# Unknown tools, Invalid arguments, Server errors should be handled as jsonrpc error,
# instead of http error (e.g., 400 Bad Request, 500 Internal Server Error)
@router.post("", status_code=status.HTTP_200_OK)
async def mcp_post(
    db_session: Annotated[Session, Depends(deps.yield_db_session)],
    request: Request,
    response: Response,
    bundle_key: str,
) -> JSONRPCSuccessResponse | JSONRPCErrorResponse | None:
    # parse payload
    try:
        payload = await _parse_payload(request)
    except UnsupportedJSONRPCMethodError as e:
        return JSONRPCErrorResponse(
            id=e.id,
            error=JSONRPCErrorResponse.ErrorData(
                code=JSONRPCErrorCode.METHOD_NOT_FOUND,
                message=str(e),
            ),
        )
    except InvalidJSONRPCPayloadError as e:
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
            # In jsonrpc, id is required in response, if unable to to detect, it must be Nullable
            id=None,
            error=JSONRPCErrorResponse.ErrorData(
                code=JSONRPCErrorCode.PARSE_ERROR,
                message=str(e),
            ),
        )

    # validate bundle existence
    mcp_server_bundle = crud.mcp_server_bundles.get_mcp_server_bundle_by_bundle_key(
        db_session,
        bundle_key,
    )
    if mcp_server_bundle is None:
        logger.error(f"Bundle not found, bundle_key={bundle_key}")
        return JSONRPCErrorResponse(
            id=payload.id if not isinstance(payload, JSONRPCNotificationInitialized) else None,
            error=JSONRPCErrorResponse.ErrorData(
                code=JSONRPCErrorCode.INVALID_REQUEST,
                message=f"Bundle not found, bundle_key={bundle_key[:10]}...",
            ),
        )

    # validate mcp session: session id is required except for initialize request
    validation_response = _validate_or_create_mcp_session(
        db_session, request, response, mcp_server_bundle, payload
    )
    if isinstance(validation_response, JSONRPCErrorResponse):
        return validation_response
    else:
        mcp_session = validation_response
        crud.mcp_sessions.update_session_last_accessed_at(
            db_session, mcp_session, datetime.now(UTC)
        )
        db_session.commit()

    # handle the request based on the request type
    match payload:
        case JSONRPCInitializeRequest():
            logger.info(f"Received initialize request={payload.model_dump()}")
            return await handlers.handle_initialize(
                db_session, mcp_session, response, mcp_server_bundle, payload
            )

        case JSONRPCToolsListRequest():
            logger.info(f"Received tools/list request={payload.model_dump()}")
            return await handlers.handle_tools_list(payload)

        case JSONRPCToolsCallRequest():
            logger.info(f"Received tools/call request={payload.model_dump()}")

            # Check if the usage exceeds the entitlement. If so, reject the request
            if config.SUBSCRIPTION_ENABLED:
                entitlement = entitlement_utils.get_organization_entitlement(
                    db_session, mcp_server_bundle.organization_id
                )
                usage = entitlement_utils.get_organization_usage(
                    db_session, mcp_server_bundle.organization_id
                )
                if not entitlement_utils.is_entitlement_fulfilling_usage(
                    entitlement=entitlement, usage=usage
                ):
                    raise UsageExceeded(
                        message="organization has reached usage limit. Please upgrade your subscription to continue.",  # noqa: E501
                    )

            return await handlers.handle_tools_call(
                db_session, mcp_session, payload, mcp_server_bundle
            )

        case JSONRPCNotificationInitialized():
            # NOTE: no-op for initialized notifications
            logger.info(f"Received initialized notification={payload.model_dump()}")
            # NOTE: it's important to return 202 otherwise some clients will not handle the response
            response.status_code = status.HTTP_202_ACCEPTED
            return None

        case JSONRPCPingRequest():
            logger.info(f"Received ping request={payload.model_dump()}")
            return JSONRPCSuccessResponse(
                id=payload.id,
                result={},
            )


@router.delete("", status_code=status.HTTP_204_NO_CONTENT)
async def mcp_delete(
    request: Request,
    db_session: Annotated[Session, Depends(deps.yield_db_session)],
) -> None:
    """
    Delete the mcp session if it exists.
    """
    mcp_session_id = request.headers.get(config.MCP_SESSION_ID_HEADER)
    if mcp_session_id is not None and utils.is_uuid(mcp_session_id):
        mcp_session = crud.mcp_sessions.get_session(db_session, UUID(mcp_session_id))
        if mcp_session is not None:
            logger.debug(f"Deleting mcp session, mcp_session_id={mcp_session_id}")
            crud.mcp_sessions.delete_session(db_session, mcp_session)
            db_session.commit()


# NOTE: for now we don't support sse stream feature so for GET return 405
@router.get("", status_code=status.HTTP_405_METHOD_NOT_ALLOWED)
async def mcp_get() -> None:
    """
    NOTE: get is not allowed for now.
    """
    pass


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
                logger.debug(f"Unsupported jsonrpc method: {jprc_payload.method}")
                raise UnsupportedJSONRPCMethodError(jprc_payload.method, jprc_payload.id)
    else:
        logger.error(f"Invalid payload type: {type(payload)}")
        raise InvalidJSONRPCPayloadError(f"Invalid payload type: {type(payload)}")


def _validate_or_create_mcp_session(
    db_session: Session,
    request: Request,
    response: Response,
    mcp_server_bundle: MCPServerBundle,
    payload: JSONRPCInitializeRequest
    | JSONRPCToolsListRequest
    | JSONRPCToolsCallRequest
    | JSONRPCNotificationInitialized
    | JSONRPCPingRequest,
) -> JSONRPCErrorResponse | MCPSession:
    """
    Validate the mcp session id.
    Returns:
        JSONRPCErrorResponse: if the mcp session id is invalid
        MCPSession: if the mcp session id is valid or a new session is created
    """
    # initialize request does not require a mcp session id
    if isinstance(payload, JSONRPCInitializeRequest):
        return crud.mcp_sessions.create_session(db_session, mcp_server_bundle.id, {})

    jrpc_request_id: str | int | None = None
    if (
        isinstance(payload, JSONRPCToolsListRequest)
        or isinstance(payload, JSONRPCToolsCallRequest)
        or isinstance(payload, JSONRPCPingRequest)
    ):
        jrpc_request_id = payload.id

    mcp_session_id = request.headers.get(config.MCP_SESSION_ID_HEADER)

    if mcp_session_id is None:
        logger.warning("MCP session ID is missing")
        response.status_code = status.HTTP_400_BAD_REQUEST
        return JSONRPCErrorResponse(
            id=jrpc_request_id,
            error=JSONRPCErrorResponse.ErrorData(
                code=JSONRPCErrorCode.INVALID_REQUEST,
                message="MCP session ID header is required, please initialize first",
            ),
        )

    # this check is needed otherwise UUID(mcp_session_id) will raise a ValueError
    if not utils.is_uuid(mcp_session_id):
        logger.warning(f"Invalid MCP session ID, mcp_session_id={mcp_session_id}")
        response.status_code = status.HTTP_400_BAD_REQUEST
        return JSONRPCErrorResponse(
            id=jrpc_request_id,
            error=JSONRPCErrorResponse.ErrorData(
                code=JSONRPCErrorCode.INVALID_REQUEST, message="Invalid MCP session ID"
            ),
        )
    mcp_session = crud.mcp_sessions.get_session(db_session, UUID(mcp_session_id))
    if mcp_session is None or mcp_session.bundle_id != mcp_server_bundle.id:
        logger.warning(
            f"MCP session not found, "
            f"mcp_session_id={mcp_session_id}, bundle_id={mcp_server_bundle.id}"
        )
        response.status_code = status.HTTP_404_NOT_FOUND
        return JSONRPCErrorResponse(
            id=jrpc_request_id,
            error=JSONRPCErrorResponse.ErrorData(
                code=JSONRPCErrorCode.INVALID_REQUEST,
                message="MCP session not found, please initialize a new session",
            ),
        )

    return mcp_session
