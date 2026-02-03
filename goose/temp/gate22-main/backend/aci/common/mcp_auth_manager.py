from collections.abc import AsyncGenerator

import httpx

from aci.common.db.sql_models import MCPServer
from aci.common.enums import HttpLocation
from aci.common.schemas.mcp_auth import (
    APIKeyConfig,
    APIKeyCredentials,
    AuthConfig,
    AuthCredentials,
    NoAuthConfig,
    NoAuthCredentials,
    OAuth2Config,
    OAuth2Credentials,
)
from aci.common.schemas.mcp_server import MCPServerMetadata

VIRTUAL_MCP_AUTH_TOKEN_HEADER = "X-Virtual-MCP-Auth-Token"


# TODO: for now the MCPAuthManager is somewhat "static", meaning it doesn't support
# refreshing token or updating refreshed token back to the database. This is fine for now because
# we create and destroy the streamablehttp_client and ClientSession for each user request.
# In the future, if we want to support long-lived connections, we need to implement a more
# sophisticated auth credentials manager that can refresh tokens and update refreshed tokens back
# to the database, similar to OAuthClientProvider from mcp python sdk.
class MCPAuthManager(httpx.Auth):
    def __init__(
        self, mcp_server: MCPServer, auth_config: AuthConfig, auth_credentials: AuthCredentials
    ):
        self.mcp_server_metadata = MCPServerMetadata.model_validate(mcp_server.server_metadata)
        self.auth_config = auth_config
        self.auth_credentials = auth_credentials

    async def async_auth_flow(
        self, request: httpx.Request
    ) -> AsyncGenerator[httpx.Request, httpx.Response]:
        # TODO: better way to do type narrowing?
        # Only header based auth are supported for real remote mcp servers
        # For virtual mcp servers (hosted by ACI), we unify the auth format.
        if isinstance(self.auth_config.root, OAuth2Config) and isinstance(
            self.auth_credentials.root, OAuth2Credentials
        ):
            if self.mcp_server_metadata.is_virtual_mcp_server:
                request.headers[VIRTUAL_MCP_AUTH_TOKEN_HEADER] = (
                    self._construct_auth_token_for_virtual_mcp(
                        self.auth_config.root.location,
                        self.auth_config.root.name,
                        self.auth_config.root.prefix,
                        self.auth_credentials.root.access_token,
                    )
                )
            else:
                if self.auth_config.root.location != HttpLocation.HEADER:
                    raise ValueError(
                        f"Unsupported auth location for remote MCP servers, location={self.auth_config.root.location}"  # noqa: E501
                    )

                request.headers[self.auth_config.root.name] = (
                    f"{self.auth_config.root.prefix} {self.auth_credentials.root.access_token}"
                    if self.auth_config.root.prefix
                    else self.auth_credentials.root.access_token
                )
        elif isinstance(self.auth_config.root, APIKeyConfig) and isinstance(
            self.auth_credentials.root, APIKeyCredentials
        ):
            if self.mcp_server_metadata.is_virtual_mcp_server:
                request.headers[VIRTUAL_MCP_AUTH_TOKEN_HEADER] = (
                    self._construct_auth_token_for_virtual_mcp(
                        self.auth_config.root.location,
                        self.auth_config.root.name,
                        self.auth_config.root.prefix,
                        self.auth_credentials.root.secret_key,
                    )
                )
            else:
                if self.auth_config.root.location != HttpLocation.HEADER:
                    raise ValueError(
                        f"Unsupported auth location for remote MCP servers, location={self.auth_config.root.location}"  # noqa: E501
                    )

                request.headers[self.auth_config.root.name] = (
                    f"{self.auth_config.root.prefix} {self.auth_credentials.root.secret_key}"
                    if self.auth_config.root.prefix
                    else self.auth_credentials.root.secret_key
                )
        elif isinstance(self.auth_config.root, NoAuthConfig) and isinstance(
            self.auth_credentials.root, NoAuthCredentials
        ):
            pass
        else:
            raise ValueError(
                f"Unsupported auth config and credentials: {self.auth_config}, {self.auth_credentials}"  # noqa: E501
            )
        yield request

    def _construct_auth_token_for_virtual_mcp(
        self, location: HttpLocation, name: str, prefix: str | None, token: str
    ) -> str:
        """
        Construct auth token for virtual mcp server (the ones hosted by ACI)
        auth convention: {"X-Virtual-MCP-Auth-Token": "<location> <name> <optional_prefix> <token>"}
        e.g.,
        - {"X-Virtual-MCP-Auth-Token": "header Authorization Bearer 1234567890"}
        - {"X-Virtual-MCP-Auth-Token": "query api_key 1234567890"}
        """
        if prefix:
            return f"{location} {name} {prefix} {token}"
        else:
            return f"{location} {name} {token}"
