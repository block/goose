import time
from uuid import UUID

from sqlalchemy.orm import Session
from tenacity import (
    retry,
    retry_if_exception_type,
    stop_after_attempt,
    wait_exponential,
)

from aci.common.db import crud
from aci.common.db.sql_models import (
    MCPServer,
    MCPServerConfiguration,
)
from aci.common.enums import ConnectedAccountOwnership
from aci.common.exceptions import AuthCredentialsManagerError
from aci.common.logging_setup import get_logger
from aci.common.oauth2_manager import OAuth2Manager
from aci.common.schemas.mcp_auth import AuthConfig, AuthCredentials, OAuth2Config, OAuth2Credentials

logger = get_logger(__name__)


# # TODO: only pass necessary data to the functions
# class SecurityCredentialsResponse(BaseModel):
#     scheme: APIKeyScheme | OAuth2Scheme | NoAuthScheme
#     credentials: APIKeySchemeCredentials | OAuth2SchemeCredentials | NoAuthSchemeCredentials
#     is_app_default_credentials: bool
#     is_updated: bool


# async def get_security_credentials(
#     app: App, app_configuration: AppConfiguration, linked_account: LinkedAccount
# ) -> SecurityCredentialsResponse:
#     if linked_account.security_scheme == SecurityScheme.API_KEY:
#         return _get_api_key_credentials(app, linked_account)
#     elif linked_account.security_scheme == SecurityScheme.OAUTH2:
#         return await _get_oauth2_credentials(app, app_configuration, linked_account)
#     elif linked_account.security_scheme == SecurityScheme.NO_AUTH:
#         return _get_no_auth_credentials(app, linked_account)
#     else:
#         logger.error(
#             f"Unsupported security scheme, linked_account_id={linked_account.id}, "
#             f"security_scheme={linked_account.security_scheme}, app={app.name}"
#         )
#         raise NoImplementationFound(
#             f"unsupported security scheme={linked_account.security_scheme}, app={app.name}"
#         )


# def update_security_credentials(
#     db_session: Session,
#     app: App,
#     linked_account: LinkedAccount,
#     security_credentials_response: SecurityCredentialsResponse,
# ) -> None:
#     """
#     Update security credentials in the database.

#     Args:
#         db_session: Database session
#         app: App object
#         linked_account: Linked account object
#         security_credentials_response: The security credentials response to update
#     """
#     if not security_credentials_response.is_updated:
#         return

#     if security_credentials_response.is_app_default_credentials:
#         crud.apps.update_app_default_security_credentials(
#             db_session,
#             app,
#             linked_account.security_scheme,
#             security_credentials_response.credentials.model_dump(),
#         )
#     else:
#         crud.linked_accounts.update_linked_account_credentials(
#             db_session,
#             linked_account,
#             security_credentials=security_credentials_response.credentials,
#         )

#     db_session.refresh(linked_account)


# async def _get_oauth2_credentials(
#     app: App, app_configuration: AppConfiguration, linked_account: LinkedAccount
# ) -> SecurityCredentialsResponse:
#     """Get OAuth2 credentials from linked account or app's default credentials.
#     If the access token is expired, it will be refreshed.
#     """
#     is_updated = False
#     oauth2_scheme = get_app_configuration_oauth2_scheme(app_configuration.app, app_configuration)
#     oauth2_scheme_credentials = OAuth2SchemeCredentials.model_validate(
#         linked_account.security_credentials
#     )
#     if _access_token_is_expired(oauth2_scheme_credentials):
#         logger.warning(
#             f"Access token expired, trying to refresh linked_account_id={linked_account.id}, "
#             f"security_scheme={linked_account.security_scheme}, app={app.name}"
#         )
#         token_response = await _refresh_oauth2_access_token(
#             app.name, oauth2_scheme, oauth2_scheme_credentials
#         )
#         # TODO: refactor parsing to _refresh_oauth2_access_token
#         expires_at: int | None = None
#         if "expires_at" in token_response:
#             expires_at = int(token_response["expires_at"])
#         elif "expires_in" in token_response:
#             expires_at = int(time.time()) + int(token_response["expires_in"])

#         if not token_response.get("access_token") or not expires_at:
#             logger.error(
#                 f"Failed to refresh access token, token_response={token_response}, "
#                 f"app={app.name}, linked_account_id={linked_account.id}, "
#                 f"security_scheme={linked_account.security_scheme}"
#             )
#             raise OAuth2Error("failed to refresh access token")

#         fields_to_update = {
#             "access_token": token_response["access_token"],
#             "expires_at": expires_at,
#         }
#         # NOTE: some app's refresh token can only be used once, so we need to update the refresh
#         # token (if returned)
#         if token_response.get("refresh_token"):
#             fields_to_update["refresh_token"] = token_response["refresh_token"]

#         oauth2_scheme_credentials = oauth2_scheme_credentials.model_copy(
#             update=fields_to_update,
#         )
#         is_updated = True

#     return SecurityCredentialsResponse(
#         scheme=oauth2_scheme,
#         credentials=oauth2_scheme_credentials,
#         is_app_default_credentials=False,  # Should never support default credentials for oauth2
#         is_updated=is_updated,
#     )


# async def _refresh_oauth2_access_token(
#     app_name: str, oauth2_scheme: OAuth2Scheme, oauth2_scheme_credentials: OAuth2SchemeCredentials
# ) -> dict:
#     refresh_token = oauth2_scheme_credentials.refresh_token
#     if not refresh_token:
#         raise OAuth2Error("no refresh token found")

#     # NOTE: it's important to use oauth2_scheme_credentials's client_id, client_secret, scope
#     # because these fields might have changed for the MCP configuration after the linked account
#     # was created
#     oauth2_manager = OAuth2Manager(
#         app_name=app_name,
#         client_id=oauth2_scheme_credentials.client_id,
#         scope=oauth2_scheme_credentials.scope,
#         authorize_url=oauth2_scheme.authorize_url,
#         access_token_url=oauth2_scheme.access_token_url,
#         refresh_token_url=oauth2_scheme.refresh_token_url,
#         client_secret=oauth2_scheme_credentials.client_secret,
#         token_endpoint_auth_method=oauth2_scheme.token_endpoint_auth_method,
#     )

#     return await oauth2_manager.refresh_token(refresh_token)


# def _get_no_auth_credentials(
#     app: App, linked_account: LinkedAccount
# ) -> SecurityCredentialsResponse:
#     """
#     a somewhat no-op function, but we keep it for consistency.
#     """
#     return SecurityCredentialsResponse(
#         scheme=NoAuthScheme.model_validate(app.security_schemes[SecurityScheme.NO_AUTH]),
#         credentials=NoAuthSchemeCredentials.model_validate(linked_account.security_credentials),
#         is_app_default_credentials=False,
#         is_updated=False,
#     )


def get_mcp_server_configuration_oauth2_config(
    mcp_server: MCPServer, mcp_server_configuration: MCPServerConfiguration
) -> OAuth2Config:
    """
    Get the OAuth2 scheme for an MCP configuration, taking into account potential overrides.
    """
    # TODO: optimize?
    for auth_config_dict in mcp_server.auth_configs:
        auth_config = AuthConfig.model_validate(auth_config_dict)
        if isinstance(auth_config.root, OAuth2Config):
            return auth_config.root

    raise AuthCredentialsManagerError(
        f"No OAuth2 config found for mcp_server_id={mcp_server.id}, "
        f"mcp_server_configuration_id={mcp_server_configuration.id}"
    )


def get_auth_config(
    mcp_server: MCPServer, mcp_server_configuration: MCPServerConfiguration
) -> AuthConfig:
    """
    Get the auth config for a mcp server configuration.
    """
    for auth_config_dict in mcp_server.auth_configs:
        auth_config = AuthConfig.model_validate(auth_config_dict)
        if auth_config.root.type == mcp_server_configuration.auth_type:
            return auth_config

    logger.error(
        f"No auth config found for mcp_server_id={mcp_server.id}, mcp_server_name={mcp_server.name}, "  # noqa: E501
        f"mcp_server_configuration_id={mcp_server_configuration.id}, mcp_server_configuration_auth_type={mcp_server_configuration.auth_type}"  # noqa: E501
    )
    raise AuthCredentialsManagerError(
        f"No auth config found for mcp_server_name={mcp_server.name}, "
        f"mcp_server_configuration_auth_type={mcp_server_configuration.auth_type}"
    )


async def get_auth_credentials(
    db_session: Session,
    mcp_server_configuration_id: UUID,
    connected_account_ownership: ConnectedAccountOwnership,
    *,
    user_id: UUID | None = None,  # Required for individual ownership
) -> AuthCredentials:
    """
    Get the auth credentials. (part of the connected account)
    For now, connected account is unique per user and mcp server configuration.
    """
    if connected_account_ownership == ConnectedAccountOwnership.SHARED:
        logger.debug(
            f"Getting auth credentials for shared connected account, mcp_server_configuration_id={mcp_server_configuration_id}"  # noqa: E501
        )
        connected_account = (
            crud.connected_accounts.get_shared_connected_account_by_mcp_server_configuration_id(
                db_session,
                mcp_server_configuration_id,
            )
        )
    elif connected_account_ownership == ConnectedAccountOwnership.OPERATIONAL:
        logger.debug(
            f"Getting auth credentials for operational connected account, mcp_server_configuration_id={mcp_server_configuration_id}"  # noqa: E501
        )
        connected_account = crud.connected_accounts.get_operational_connected_account_by_mcp_server_configuration_id(  # noqa: E501
            db_session,
            mcp_server_configuration_id,
        )
    elif connected_account_ownership == ConnectedAccountOwnership.INDIVIDUAL:
        if user_id is None:
            logger.error("User ID is required for individual connected account")
            raise AuthCredentialsManagerError(
                "User ID is required for individual connected account"
            )
        logger.debug(
            f"Getting auth credentials for individual connected account, user_id={user_id}, mcp_server_configuration_id={mcp_server_configuration_id}"  # noqa: E501
        )
        connected_account = crud.connected_accounts.get_connected_account_by_user_id_and_mcp_server_configuration_id(  # noqa: E501
            db_session,
            user_id,
            mcp_server_configuration_id,
        )
    if connected_account is None:
        logger.error(
            f"Connected account not found, user_id={user_id}, mcp_server_configuration_id={mcp_server_configuration_id}, ownership={connected_account_ownership}"  # noqa: E501
        )
        raise AuthCredentialsManagerError("Connected account not found")

    auth_credentials = AuthCredentials.model_validate(connected_account.auth_credentials)

    if _need_refresh(auth_credentials):
        logger.warning(
            f"Auth credentials need to be refreshed, "
            f"user_id={user_id}, "
            f"mcp_server_configuration_id={mcp_server_configuration_id}, "
            f"mcp_server_name={connected_account.mcp_server_configuration.mcp_server.name}"
        )
        # grab connected account for updating:
        # it will NOT block if the account is already locked by another process. in which case we
        # sleep and re-check if it needs to be refreshed again
        logger.info("grabbing connected account with lock for refresh")
        connected_account_id = connected_account.id
        connected_account = crud.connected_accounts.get_connected_account_by_id(
            db_session,
            connected_account_id,
            throw_error_if_not_found=False,
            with_for_update=True,
        )
        # TODO: other account updates can also block the lock acquisition. Might need to move
        # lock acquisition to _wait_for_connected_account_refresh
        if connected_account is None:
            return await _wait_for_connected_account_refresh(db_session, connected_account_id)

        else:
            logger.info("grabbed connected account with lock for refresh")
            # TODO: consider auth_config as parameters?
            auth_credentials = await _refresh_auth_credentials(
                connected_account.mcp_server_configuration.mcp_server.name,
                get_auth_config(
                    connected_account.mcp_server_configuration.mcp_server,
                    connected_account.mcp_server_configuration,
                ),
                auth_credentials,
            )
            # update back to the connected account
            crud.connected_accounts.update_connected_account_auth_credentials(
                db_session,
                connected_account,
                auth_credentials.model_dump(mode="json", exclude_none=True),
            )
            # TODO: better place to unify commit
            db_session.commit()

    return auth_credentials


@retry(
    stop=stop_after_attempt(3),
    wait=wait_exponential(multiplier=1, min=1, max=4),
    retry=retry_if_exception_type(AuthCredentialsManagerError),
    reraise=True,
)
async def _wait_for_connected_account_refresh(
    db_session: Session, connected_account_id: UUID
) -> AuthCredentials:
    """
    Wait for another process to complete the token refresh.
    Uses tenacity to retry with exponential backoff if credentials are not yet refreshed.
    NOTE: need to use async for asyncio.sleep
    """
    logger.info(
        f"Connected account locked by another process, checking if refresh completed "
        f"(connected_account_id={connected_account_id})"
    )
    # Try getting the latest connected account without locking
    connected_account = crud.connected_accounts.get_connected_account_by_id(
        db_session,
        connected_account_id,
        throw_error_if_not_found=True,
    )
    auth_credentials = AuthCredentials.model_validate(connected_account.auth_credentials)

    if _need_refresh(auth_credentials):
        logger.warning(
            f"Auth credentials still need refresh, will retry "
            f"(connected_account_id={connected_account_id})"
        )
        raise AuthCredentialsManagerError(
            "Auth credentials not yet refreshed by other process, retrying..."
        )
    else:
        logger.info(
            f"Auth credentials successfully refreshed by another process "
            f"(connected_account_id={connected_account_id})"
        )
        return auth_credentials


def _need_refresh(auth_credentials: AuthCredentials, leeway_seconds: int = 60) -> bool:
    """
    Check if the auth credentials need to be refreshed.
    """

    # TODO: api key based auth credentials can also expire
    if isinstance(auth_credentials.root, OAuth2Credentials):
        return (
            auth_credentials.root.expires_at is not None
            and auth_credentials.root.expires_at < int(time.time()) + leeway_seconds
        )
    return False


# TODO: throw specific error for cases where re-authentication is needed?
# e.g., expired access token but no way to refresh it
async def _refresh_auth_credentials(
    mcp_server_name: str, auth_config: AuthConfig, auth_credentials: AuthCredentials
) -> AuthCredentials:
    """
    Refresh the auth credentials.
    """
    if isinstance(auth_config.root, OAuth2Config) and isinstance(
        auth_credentials.root, OAuth2Credentials
    ):
        refresh_token = auth_credentials.root.refresh_token
        if not refresh_token:
            logger.error(f"No refresh token found for mcp_server_name={mcp_server_name}")
            raise AuthCredentialsManagerError("no refresh token found, please re-authenticate")

        oauth2_manager = OAuth2Manager(
            app_name=mcp_server_name,
            client_id=auth_config.root.client_id,
            scope=auth_config.root.scope,
            authorize_url=auth_config.root.authorize_url,
            access_token_url=auth_config.root.access_token_url,
            refresh_token_url=auth_config.root.refresh_token_url,
            client_secret=auth_config.root.client_secret,
            token_endpoint_auth_method=auth_config.root.token_endpoint_auth_method,
        )

        refresh_token_response = await oauth2_manager.refresh_token(refresh_token)

        expires_at: int | None = None
        if "expires_at" in refresh_token_response:
            expires_at = int(refresh_token_response["expires_at"])
        elif "expires_in" in refresh_token_response:
            expires_at = int(time.time()) + int(refresh_token_response["expires_in"])

        if not refresh_token_response.get("access_token") or not expires_at:
            logger.error(
                f"Failed to refresh access token, refresh_token_response={refresh_token_response}, "
                f"mcp_server_name={mcp_server_name}"
            )
            raise AuthCredentialsManagerError("failed to refresh access token")

        fields_to_update = {
            "access_token": refresh_token_response["access_token"],
            "expires_at": expires_at,
        }
        # NOTE: some providers' refresh token can only be used once, so we need to update the
        # refresh token (if returned)
        if refresh_token_response.get("refresh_token"):
            fields_to_update["refresh_token"] = refresh_token_response["refresh_token"]

        # Update the root OAuth2Credentials object, not the wrapper
        updated_oauth2_credentials = auth_credentials.root.model_copy(
            update=fields_to_update,
        )
        # Create new AuthCredentials with the updated root
        auth_credentials = AuthCredentials(root=updated_oauth2_credentials)

        return auth_credentials
    else:
        logger.error(
            f"Unsupported auth credentials type for refresh: {type(auth_credentials.root)}"
        )
        raise NotImplementedError(
            f"Unsupported auth credentials type for refresh: {type(auth_credentials.root)}"
        )
