import random
import string
import time
from typing import Any, cast

from authlib.integrations.httpx_client import AsyncOAuth2Client

from aci.common.exceptions import OAuth2ManagerError
from aci.common.logging_setup import get_logger
from aci.common.schemas.mcp_auth import OAuth2Credentials

UNICODE_ASCII_CHARACTER_SET = string.ascii_letters + string.digits
logger = get_logger(__name__)


class OAuth2Manager:
    def __init__(
        self,
        app_name: str,
        client_id: str,
        scope: str,
        authorize_url: str,
        access_token_url: str,
        refresh_token_url: str,
        client_secret: str | None = None,
        token_endpoint_auth_method: str | None = None,
    ):
        """
        Initialize the OAuth2Manager

        Args:
            app_name: The name of the ACI.dev App
            client_id: The client ID of the OAuth2 client
            client_secret: The client secret of the OAuth2 client
            scope: The scope of the OAuth2 client
            authorize_url: The URL of the OAuth2 authorization server
            access_token_url: The URL of the OAuth2 access token server
            refresh_token_url: The URL of the OAuth2 refresh token server
            token_endpoint_auth_method:
                client_secret_basic (default) | client_secret_post | none
                Additional options can be achieved by registering a custom auth method
        """
        self.app_name = app_name
        self.client_id = client_id
        self.client_secret = client_secret
        self.scope = scope
        self.authorize_url = authorize_url
        self.access_token_url = access_token_url
        self.refresh_token_url = refresh_token_url
        self.token_endpoint_auth_method = token_endpoint_auth_method

        # TODO: need to close the client after use
        # Add an aclose() helper (or implement __aenter__/__aexit__) and make callers invoke it
        # during shutdown.
        # NOTE: don't pass in scope here, otherwise it will be sent during refresh token request
        # which is not needed
        # NOTE: that token_endpoint_auth_method is being handled smartly by authlib if None:
        # if token_endpoint_auth_method is None:
        #   if client_secret:
        #     token_endpoint_auth_method = "client_secret_basic"
        #   else:
        #     token_endpoint_auth_method = "none"
        self.oauth2_client = AsyncOAuth2Client(
            client_id=client_id,
            client_secret=client_secret,
            token_endpoint_auth_method=token_endpoint_auth_method,
            code_challenge_method="S256",  # only S256 is supported
            # TODO: use update_token callback to save tokens to the database
            update_token=None,
        )

    # TODO: some app may not support "code_verifier"?
    async def create_authorization_url(
        self,
        redirect_uri: str,
        state: str,
        code_verifier: str,
        access_type: str = "offline",
        prompt: str = "consent",
    ) -> str:
        """
        Create authorization URL for user to authorize your application

        Args:
            redirect_uri: The redirect URI of the OAuth2 client
            state: state parameter for CSRF protection, also used to store required data for the
                   callback
            code_verifier: The code verifier used to for the authorization url
            access_type: The access type of the OAuth2 client
            prompt: The prompt of the OAuth2 client

        Returns:
            authorization_url: The authorization URL for the user to authorize the app
        """

        # TODO: some oauth2 apps may have unconventional params, temporarily handle them here
        # app_specific_params = {}
        # if self.app_name == "REDDIT":
        #     app_specific_params = {
        #         "duration": "permanent",
        #     }
        #     logger.info(
        #         f"Adding app specific params, app_name={self.app_name}, "
        #         f"params={app_specific_params}"
        #     )
        # NOTE:
        # - "scope" can be specified here
        # - "response_type" can be specified here (default is "code")
        # - and additional options can be specified here (like access_type, prompt, etc.)
        authorization_url, _ = self.oauth2_client.create_authorization_url(
            url=self.authorize_url,
            redirect_uri=redirect_uri,
            state=state,
            code_verifier=code_verifier,
            access_type=access_type,
            prompt=prompt,
            scope=self.scope,
            # **app_specific_params,
        )

        return str(authorization_url)

    # TODO: some app may not support "code_verifier"?
    async def fetch_token(
        self,
        redirect_uri: str,
        code: str,
        code_verifier: str,
    ) -> dict[str, Any]:
        """
        Exchange authorization code for access token

        Args:
            redirect_uri: The redirect URI of the OAuth2 client
            code: The authorization code returned from OAuth2 provider
            code_verifier: The code verifier used to for the authorization url

        Returns:
            Token response dictionary
        """
        try:
            token = cast(
                dict[str, Any],
                await self.oauth2_client.fetch_token(
                    self.access_token_url,
                    redirect_uri=redirect_uri,
                    code=code,
                    code_verifier=code_verifier,
                    scope=self.scope,
                ),
            )
            return token
        except Exception as e:
            logger.error(f"Failed to fetch access token, app_name={self.app_name}, error={e}")
            raise OAuth2ManagerError("Failed to fetch access token") from e

    async def refresh_token(
        self,
        refresh_token: str,
    ) -> dict[str, Any]:
        try:
            token = cast(
                dict[str, Any],
                await self.oauth2_client.refresh_token(
                    self.refresh_token_url, refresh_token=refresh_token
                ),
            )
            return token
        except Exception as e:
            logger.error(f"Failed to refresh access token, app_name={self.app_name}, error={e}")
            raise OAuth2ManagerError("Failed to refresh access token") from e

    def parse_fetch_token_response(self, token: dict) -> OAuth2Credentials:
        """
        Parse OAuth2Credentials from token response with app-specific handling.

        Args:
            token: OAuth2 token response from provider

        Returns:
            OAuth2SchemeCredentials with appropriate fields set
        """
        data = token

        # # handle Slack's special case
        # TODO: still relevant for mcp server?
        # if self.app_name == "SLACK":
        #     if "authed_user" in data:
        #         data = cast(dict, data["authed_user"])
        #     else:
        #         logger.error(f"Missing authed_user in Slack OAuth response, app={self.app_name}")
        #         raise OAuth2Error("Missing access_token in Slack OAuth response")

        if "access_token" not in data:
            logger.error(f"Missing access_token in OAuth response, app={self.app_name}")
            raise OAuth2ManagerError("Missing access_token in OAuth response")

        # some apps have long live access token so expiration time may not be present
        expires_at: int | None = None
        if "expires_at" in data:
            expires_at = int(data["expires_at"])
        elif "expires_in" in data:
            expires_at = int(time.time()) + int(data["expires_in"])

        # TODO: if scope is present, check if it matches the scope in the MCP Configuration

        return OAuth2Credentials(
            access_token=data["access_token"],
            token_type=data.get("token_type"),
            expires_at=expires_at,
            refresh_token=data.get("refresh_token"),
        )

    @staticmethod
    def generate_code_verifier(length: int = 48) -> str:
        """
        Generate a random code verifier for OAuth2
        """
        rand = random.SystemRandom()
        return "".join(rand.choice(UNICODE_ASCII_CHARACTER_SET) for _ in range(length))

    # # TODO: consider adding this inside create_authorization_url function instead of
    # # calling it separately
    # @staticmethod
    # def rewrite_oauth2_authorization_url(app_name: str, authorization_url: str) -> str:
    #     """
    #     Rewrite OAuth2 authorization URL for specific apps that need special handling.
    #     Currently handles Slack's special case where user scopes and scopes need to be replaced.
    #     TODO: this approach is hacky and need to refactor this in the future

    #     Args:
    #         app_name: Name of the OAuth2 app (e.g., 'slack')
    #         authorization_url: The original authorization URL

    #     Returns:
    #         The rewritten authorization URL if needed, otherwise the original URL
    #     """
    #     if app_name == "SLACK":
    #         # Slack requires user scopes to be prefixed with 'user_'
    #         # Replace 'scope=' with 'user_scope=' and add 'scope=' with the null value
    #         if "scope=" in authorization_url:
    #             # Extract the original scope value
    #             scope_start = authorization_url.find("scope=") + 6
    #             scope_end = authorization_url.find("&", scope_start)
    #             if scope_end == -1:
    #                 scope_end = len(authorization_url)
    #             original_scope = authorization_url[scope_start:scope_end]

    #             # Replace the original scope with user_scope and add scope
    #             new_url = authorization_url.replace(
    #                 f"scope={original_scope}", f"user_scope={original_scope}&scope="
    #             )
    #             return new_url

    #     return authorization_url
