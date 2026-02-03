from typing import Annotated
from uuid import UUID

from authlib.jose import jwt
from fastapi import APIRouter, Depends, HTTPException, Query, Request, status
from fastapi.responses import RedirectResponse
from pydantic import ValidationError
from sqlalchemy.orm import Session

from aci.common import auth_credentials_manager as acm
from aci.common.db import crud
from aci.common.db.sql_models import ConnectedAccount, MCPServerConfiguration
from aci.common.enums import AuthType, ConnectedAccountOwnership, OrganizationRole
from aci.common.logging_setup import get_logger
from aci.common.oauth2_manager import OAuth2Manager
from aci.common.schemas.connected_account import (
    ConnectedAccountAPIKeyCreate,
    ConnectedAccountCreate,
    ConnectedAccountNoAuthCreate,
    ConnectedAccountOAuth2Create,
    ConnectedAccountOAuth2CreateState,
    ConnectedAccountPublic,
    OAuth2ConnectedAccountCreateResponse,
)
from aci.common.schemas.mcp_auth import APIKeyCredentials, NoAuthCredentials
from aci.common.schemas.pagination import PaginationParams, PaginationResponse
from aci.control_plane import access_control, config, schema_utils
from aci.control_plane import dependencies as deps
from aci.control_plane.exceptions import (
    MCPServerConfigurationNotFound,
    NotPermittedError,
    OAuth2Error,
)

logger = get_logger(__name__)
router = APIRouter()
CONNECTED_ACCOUNTS_OAUTH2_CALLBACK_ROUTE_NAME = "connected_accounts_oauth2_callback"


@router.post("")
async def create_connected_account(
    request: Request,
    context: Annotated[deps.RequestContext, Depends(deps.get_request_context)],
    body: ConnectedAccountCreate,
) -> OAuth2ConnectedAccountCreateResponse | ConnectedAccountPublic:
    mcp_server_config = crud.mcp_server_configurations.get_mcp_server_configuration_by_id(
        context.db_session, body.mcp_server_configuration_id, throw_error_if_not_found=False
    )

    if not mcp_server_config:
        raise HTTPException(status_code=404, detail="MCP server configuration not found")

    # Check if the user is acted as the organization of the MCP server configuration
    access_control.check_act_as_organization_role(
        context.act_as,
        requested_organization_id=mcp_server_config.organization_id,
        throw_error_if_not_permitted=True,
    )

    if (
        mcp_server_config.connected_account_ownership == ConnectedAccountOwnership.SHARED
        or mcp_server_config.connected_account_ownership == ConnectedAccountOwnership.OPERATIONAL
    ):
        if context.act_as.role != OrganizationRole.ADMIN:
            logger.error("Only admin can create shared or operational accounts")
            raise NotPermittedError("Only admin can create shared or operational accounts")

    else:
        if context.act_as.role != OrganizationRole.MEMBER:
            logger.error("Only members can create individual accounts")
            raise NotPermittedError("Only members can create individual accounts")

        # check if the MCP server configuration's allowed teams contains team the user belongs to
        access_control.check_mcp_server_config_accessibility(
            db_session=context.db_session,
            user_id=context.user_id,
            mcp_server_configuration_id=mcp_server_config.id,
            throw_error_if_not_permitted=True,
        )

    try:
        match mcp_server_config.auth_type:
            case AuthType.OAUTH2:
                redirect_url_after_account_creation = ConnectedAccountOAuth2Create.model_validate(
                    body.model_dump(exclude_none=True)
                ).redirect_url_after_account_creation
                return await _create_oauth2_connected_account(
                    request,
                    context,
                    mcp_server_config,
                    redirect_url_after_account_creation,
                )
            case AuthType.API_KEY:
                api_key = ConnectedAccountAPIKeyCreate.model_validate(
                    body.model_dump(exclude_none=True)
                ).api_key
                connected_account = await _create_api_key_connected_account(
                    context,
                    mcp_server_config,
                    api_key,
                )
                context.db_session.commit()
                return schema_utils.construct_connected_account_public(
                    context.db_session, connected_account
                )
            case AuthType.NO_AUTH:
                ConnectedAccountNoAuthCreate.model_validate(body.model_dump(exclude_none=True))
                connected_account = await _create_no_auth_connected_account(
                    context, mcp_server_config
                )
                context.db_session.commit()
                return schema_utils.construct_connected_account_public(
                    context.db_session, connected_account
                )
    except ValidationError as e:
        logger.error(f"Invalid auth type, auth_type={mcp_server_config.auth_type}, error={e}")
        raise HTTPException(
            status_code=400, detail="Invalid input for mcp server configuration auth type"
        ) from e


async def _create_api_key_connected_account(
    context: deps.RequestContext,
    mcp_server_config: MCPServerConfiguration,
    api_key: str,
) -> ConnectedAccount:
    auth_credentials = APIKeyCredentials(type=AuthType.API_KEY, secret_key=api_key)
    return await _upsert_connected_account(
        context.db_session,
        context.user_id,
        mcp_server_config,
        auth_credentials.model_dump(mode="json"),
    )


async def _create_no_auth_connected_account(
    context: deps.RequestContext,
    mcp_server_config: MCPServerConfiguration,
) -> ConnectedAccount:
    auth_credentials = NoAuthCredentials(type=AuthType.NO_AUTH)
    return await _upsert_connected_account(
        context.db_session,
        context.user_id,
        mcp_server_config,
        auth_credentials.model_dump(mode="json"),
    )


async def _upsert_connected_account(
    db_session: Session,
    user_id: UUID,
    mcp_server_config: MCPServerConfiguration,
    auth_credentials: dict,
) -> ConnectedAccount:
    # Get existing connected account
    match mcp_server_config.connected_account_ownership:
        case ConnectedAccountOwnership.SHARED:
            connected_account = (
                crud.connected_accounts.get_shared_connected_account_by_mcp_server_configuration_id(
                    db_session,
                    mcp_server_config.id,
                )
            )
        case ConnectedAccountOwnership.OPERATIONAL:
            connected_account = crud.connected_accounts.get_operational_connected_account_by_mcp_server_configuration_id(  # noqa: E501
                db_session,
                mcp_server_config.id,
            )
        case ConnectedAccountOwnership.INDIVIDUAL:
            connected_account = crud.connected_accounts.get_connected_account_by_user_id_and_mcp_server_configuration_id(  # noqa: E501
                db_session,
                user_id,
                mcp_server_config.id,
            )

    # if the connected account already exists, update it, otherwise create a new one
    if connected_account:
        crud.connected_accounts.update_connected_account_auth_credentials(
            db_session, connected_account, auth_credentials
        )
    else:
        connected_account = crud.connected_accounts.create_connected_account(
            db_session,
            user_id,
            mcp_server_config.id,
            auth_credentials,
            mcp_server_config.connected_account_ownership,
        )

    return connected_account


async def _create_oauth2_connected_account(
    request: Request,
    context: deps.RequestContext,
    mcp_server_config: MCPServerConfiguration,
    redirect_url_after_account_creation: str | None,
) -> OAuth2ConnectedAccountCreateResponse:
    oauth2_config = acm.get_mcp_server_configuration_oauth2_config(
        mcp_server_config.mcp_server, mcp_server_config
    )

    oauth2_manager = OAuth2Manager(
        app_name=mcp_server_config.mcp_server.name,
        client_id=oauth2_config.client_id,
        scope=oauth2_config.scope,
        authorize_url=oauth2_config.authorize_url,
        access_token_url=oauth2_config.access_token_url,
        refresh_token_url=oauth2_config.refresh_token_url,
        client_secret=oauth2_config.client_secret,
        token_endpoint_auth_method=oauth2_config.token_endpoint_auth_method,
    )

    # create and encode the state payload.
    # NOTE: the state payload is jwt encoded (signed), but it's not encrypted, anyone can decode it
    # TODO: add expiration check to the state payload for extra security
    oauth2_state = ConnectedAccountOAuth2CreateState(
        mcp_server_configuration_id=mcp_server_config.id,
        user_id=context.user_id,
        code_verifier=OAuth2Manager.generate_code_verifier(),
        redirect_url_after_account_creation=redirect_url_after_account_creation,
    )

    # decode() is needed to convert the bytes to a string (not decoding the jwt payload) for this
    # jwt library.
    oauth2_state_jwt = jwt.encode(
        {"alg": config.JWT_ALGORITHM},
        oauth2_state.model_dump(mode="json", exclude_none=True),
        config.JWT_SIGNING_KEY,
    ).decode()

    path = request.url_for(CONNECTED_ACCOUNTS_OAUTH2_CALLBACK_ROUTE_NAME).path
    redirect_uri = f"{config.CONTROL_PLANE_BASE_URL}{path}"
    authorization_url = await oauth2_manager.create_authorization_url(
        redirect_uri=redirect_uri,
        state=oauth2_state_jwt,
        code_verifier=oauth2_state.code_verifier,
    )

    logger.info(f"Connected account oauth2 authorization url={authorization_url}")

    return OAuth2ConnectedAccountCreateResponse(authorization_url=authorization_url)


@router.get(
    "/oauth2/callback",
    name=CONNECTED_ACCOUNTS_OAUTH2_CALLBACK_ROUTE_NAME,
    response_model=ConnectedAccountPublic,
    response_model_exclude_none=True,
)
async def oauth2_callback(
    request: Request,
    db_session: Annotated[Session, Depends(deps.yield_db_session)],
) -> ConnectedAccountPublic | RedirectResponse:
    """
    Callback endpoint for OAuth2 account creation.
    - A connected account (with necessary credentials from the OAuth2 provider) will be created in
    the database.
    """
    # check for errors
    error = request.query_params.get("error")
    error_description = request.query_params.get("error_description")
    if error:
        logger.error(
            f"OAuth2 account creation callback received, error={error}, "
            f"error_description={error_description}"
        )
        raise OAuth2Error(
            f"oauth2 account creation callback error: {error}, "
            f"error_description: {error_description}"
        )

    # check for code
    code = request.query_params.get("code")
    if not code:
        logger.error("OAuth2 account creation callback received, missing code")
        raise OAuth2Error("missing code parameter during account creation")

    # check for state
    state_jwt = request.query_params.get("state")
    if not state_jwt:
        logger.error(
            "OAuth2 account creation callback received, missing state",
        )
        raise OAuth2Error("missing state parameter during account creation")

    # decode the state payload
    try:
        state = ConnectedAccountOAuth2CreateState.model_validate(
            jwt.decode(state_jwt, config.JWT_SIGNING_KEY)
        )
        logger.info(
            f"OAuth2 account creation callback received, decoded "
            f"state={state.model_dump(exclude_none=True)}",
        )
    except Exception as e:
        logger.exception(f"Failed to decode OAuth2 state, error={e}")
        raise OAuth2Error("invalid state parameter during account linking") from e

    mcp_server_configuration = crud.mcp_server_configurations.get_mcp_server_configuration_by_id(
        db_session, state.mcp_server_configuration_id, throw_error_if_not_found=False
    )
    if not mcp_server_configuration:
        logger.error(
            f"Unable to continue with account creation, mcp server configuration not found "
            f"mcp_server_configuration_id={state.mcp_server_configuration_id}"
        )
        raise MCPServerConfigurationNotFound(
            f"mcp server configuration={state.mcp_server_configuration_id} not found"
        )

    # create oauth2 manager
    oauth2_config = acm.get_mcp_server_configuration_oauth2_config(
        mcp_server_configuration.mcp_server, mcp_server_configuration
    )

    oauth2_manager = OAuth2Manager(
        app_name=mcp_server_configuration.mcp_server.name,
        client_id=oauth2_config.client_id,
        scope=oauth2_config.scope,
        authorize_url=oauth2_config.authorize_url,
        access_token_url=oauth2_config.access_token_url,
        refresh_token_url=oauth2_config.refresh_token_url,
        client_secret=oauth2_config.client_secret,
        token_endpoint_auth_method=oauth2_config.token_endpoint_auth_method,
    )

    path = request.url_for(CONNECTED_ACCOUNTS_OAUTH2_CALLBACK_ROUTE_NAME).path
    redirect_uri = f"{config.CONTROL_PLANE_BASE_URL}{path}"
    token_response = await oauth2_manager.fetch_token(
        redirect_uri=redirect_uri,
        code=code,
        code_verifier=state.code_verifier,
    )
    auth_credentials = oauth2_manager.parse_fetch_token_response(token_response)

    # TODO: consider separating the logic for updating and creating a connected account or give
    # warning to clients if the connected account already exists to avoid accidental overwriting the
    # account
    connected_account = await _upsert_connected_account(
        db_session,
        state.user_id,
        mcp_server_configuration,
        auth_credentials.model_dump(mode="json"),
    )
    db_session.commit()

    if state.redirect_url_after_account_creation:
        return RedirectResponse(
            url=state.redirect_url_after_account_creation, status_code=status.HTTP_302_FOUND
        )

    return schema_utils.construct_connected_account_public(db_session, connected_account)


@router.get("", response_model=PaginationResponse[ConnectedAccountPublic])
async def list_connected_accounts(
    context: Annotated[deps.RequestContext, Depends(deps.get_request_context)],
    pagination_params: Annotated[PaginationParams, Depends()],
    config_id: Annotated[list[UUID] | None, Query()] = None,
    # Now used `config_id` for shorter query string. Can rename it back to
    # `mcp_server_configuration_id` later.
    # Used Singular key form instead of plural as a common practice for array type query parameters.
) -> PaginationResponse[ConnectedAccountPublic]:
    input_mcp_server_configuration_ids = config_id
    if context.act_as.role == OrganizationRole.ADMIN:
        # Admin can see all connected accounts of the organization
        connected_accounts = crud.connected_accounts.get_connected_accounts_by_organization_id(
            context.db_session,
            context.act_as.organization_id,
            mcp_server_configuration_ids=input_mcp_server_configuration_ids,
            offset=pagination_params.offset,
            limit=pagination_params.limit,
        )
    else:
        # Get a list of MCP server configurations that the user has access to
        teams = crud.teams.get_teams_by_user_id(
            context.db_session, context.act_as.organization_id, context.user_id
        )
        team_ids = [team.id for team in teams]
        if len(team_ids) == 0:
            connected_accounts = []
        else:
            accessible_configurations = (
                crud.mcp_server_configurations.get_mcp_server_configurations(
                    context.db_session, context.act_as.organization_id, team_ids=team_ids
                )
            )
            accessible_mcp_server_configuration_ids = [
                mcp_server_configuration.id
                for mcp_server_configuration in accessible_configurations
            ]

            # If user provided `config_ids` to filter for, intersect it with
            # accessible_mcp_server_configuration_ids to get the actual effective
            # mcp_server_configuration_ids
            if input_mcp_server_configuration_ids is not None:
                accessible_mcp_server_configuration_ids = list(
                    set(accessible_mcp_server_configuration_ids)
                    & set(input_mcp_server_configuration_ids)
                )

            # Fetch connected accounts that the user has access to
            connected_accounts = crud.connected_accounts.get_org_member_accessible_connected_accounts_by_mcp_server_configuration_ids(  # noqa: E501
                db_session=context.db_session,
                user_id=context.user_id,
                user_accessible_mcp_server_configuration_ids=accessible_mcp_server_configuration_ids,
                offset=pagination_params.offset,
                limit=pagination_params.limit,
            )

    return PaginationResponse[ConnectedAccountPublic](
        data=[
            schema_utils.construct_connected_account_public(context.db_session, connected_account)
            for connected_account in connected_accounts
        ],
        offset=pagination_params.offset,
    )


@router.delete("/{connected_account_id}")
async def delete_connected_account(
    context: Annotated[deps.RequestContext, Depends(deps.get_request_context)],
    connected_account_id: UUID,
) -> None:
    connected_account = crud.connected_accounts.get_connected_account_by_id(
        context.db_session, connected_account_id, throw_error_if_not_found=False
    )
    if not connected_account:
        raise HTTPException(status_code=404, detail="Connected account not found")

    # Check if the user is acted as the organization of the connected account
    access_control.check_act_as_organization_role(
        context.act_as,
        requested_organization_id=connected_account.mcp_server_configuration.organization_id,
        throw_error_if_not_permitted=True,
    )

    # Member can only delete their own connected accounts.
    # Admin can delete any connected account, so no need to check here.
    if context.act_as.role == OrganizationRole.MEMBER:
        if not (
            connected_account.user_id == context.user_id
            and connected_account.ownership == ConnectedAccountOwnership.INDIVIDUAL
        ):
            logger.error(
                f"Connected account {connected_account_id} is not belongs to the member {context.user_id}"  # noqa: E501
            )
            raise NotPermittedError(message="Cannot delete others' connected accounts")

    # Delete the connected account
    crud.connected_accounts.delete_connected_account(context.db_session, connected_account_id)
    context.db_session.commit()
