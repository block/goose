import re
from dataclasses import dataclass
from urllib.parse import urljoin, urlparse

import httpx

from aci.common.logging_setup import get_logger
from aci.control_plane.exceptions import OAuth2MetadataDiscoveryError
from aci.control_plane.services.oauth2_client.schema import (
    OAuthMetadata,
    ProtectedResourceMetadata,
)

logger = get_logger(__name__)


@dataclass
class DiscoveryContext:
    server_url: str
    protected_resource_metadata: ProtectedResourceMetadata | None = None
    auth_server_url: str | None = None
    oauth_metadata: OAuthMetadata | None = None

    def get_authorization_base_url(self, server_url: str) -> str:
        """Extract base URL by removing path component."""
        parsed = urlparse(server_url)
        return f"{parsed.scheme}://{parsed.netloc}"


class MetadataFetcher:
    def __init__(self, server_url: str):
        self.context = DiscoveryContext(server_url=server_url)

    def _extract_resource_metadata_from_www_auth(self, init_response: httpx.Response) -> str | None:
        """
        Extract protected resource metadata URL from WWW-Authenticate header as per RFC9728.

        Returns:
            Resource metadata URL if found in WWW-Authenticate header, None otherwise

        This function is adapted from the Official MCP Python SDK
        https://github.com/modelcontextprotocol/python-sdk/blob/ca3466666310dbcb5c45690ac2571c574759984f/src/mcp/client/auth.py#L207-L229
        """
        if not init_response or init_response.status_code != 401:
            return None

        www_auth_header = init_response.headers.get("WWW-Authenticate")
        if not www_auth_header:
            return None

        # Pattern matches: resource_metadata="url" or resource_metadata=url (unquoted)
        pattern = r'resource_metadata=(?:"([^"]+)"|([^\s,]+))'
        match = re.search(pattern, www_auth_header)

        if match:
            # Return quoted value if present, otherwise unquoted value
            return match.group(1) or match.group(2)

        return None

    def _discover_protected_resource(self, init_response: httpx.Response) -> None:
        """
        Discover protected resource metadata (RFC9728 with WWW-Authenticate support)
        Updates the context with:
        1. Protected resource metadata
        2. The authorization server URL

        This function is adapted and modified from the Official MCP Python SDK
        https://github.com/modelcontextprotocol/python-sdk/blob/ca3466666310dbcb5c45690ac2571c574759984f/src/mcp/client/auth.py#L231-L238
        """
        # RFC9728: Try to extract resource_metadata URL from WWW-Authenticate header of the initial response # noqa: E501
        url = self._extract_resource_metadata_from_www_auth(init_response)

        if not url:
            # Fallback to well-known discovery
            auth_base_url = self.context.get_authorization_base_url(self.context.server_url)
            url = urljoin(auth_base_url, "/.well-known/oauth-protected-resource")

        # Question: Do we need to provide header regarding MCP protocol version?
        # Original code snippet:
        # return httpx.Request("GET", url, headers={MCP_PROTOCOL_VERSION: LATEST_PROTOCOL_VERSION})
        try:
            response = httpx.get(url, timeout=httpx.Timeout(5.0))
        except httpx.RequestError as e:
            raise OAuth2MetadataDiscoveryError(
                f"Protected resource metadata discovery failed: {url}"
            ) from e

        if response.status_code != 200:
            raise OAuth2MetadataDiscoveryError(
                f"Protected resource metadata discovery failed: {url}"
            )

        content = response.read()
        metadata = ProtectedResourceMetadata.model_validate_json(content)
        self.context.protected_resource_metadata = metadata
        if metadata.authorization_servers:
            self.context.auth_server_url = str(metadata.authorization_servers[0])

    def _get_discovery_urls(self) -> list[str]:
        """
        Generate ordered list of (url, type) tuples for discovery attempts.

        This function is adapted from the Official MCP Python SDK
        https://github.com/modelcontextprotocol/python-sdk/blob/ca3466666310dbcb5c45690ac2571c574759984f/src/mcp/client/auth.py#L254-L279
        """
        urls: list[str] = []
        auth_server_url = self.context.auth_server_url or self.context.server_url
        parsed = urlparse(auth_server_url)
        base_url = f"{parsed.scheme}://{parsed.netloc}"

        # RFC 8414: Path-aware OAuth discovery
        if parsed.path and parsed.path != "/":
            oauth_path = f"/.well-known/oauth-authorization-server{parsed.path.rstrip('/')}"
            urls.append(urljoin(base_url, oauth_path))

        # OAuth root fallback
        urls.append(urljoin(base_url, "/.well-known/oauth-authorization-server"))

        # RFC 8414 section 5: Path-aware OIDC discovery
        # See https://www.rfc-editor.org/rfc/rfc8414.html#section-5
        if parsed.path and parsed.path != "/":
            oidc_path = f"/.well-known/openid-configuration{parsed.path.rstrip('/')}"
            urls.append(urljoin(base_url, oidc_path))

        # OIDC 1.0 fallback (appends to full URL per OIDC spec)
        oidc_fallback = f"{auth_server_url.rstrip('/')}/.well-known/openid-configuration"
        urls.append(oidc_fallback)

        return urls

    def _get_oauth_metadata(self, discovery_urls: list[str]) -> None:
        """
        Discover OAuth metadata (RFC8414 with fallback for legacy servers)
        Updates the context with:
        1. OAuth metadata

        This function is adapted and modified from the Official MCP Python SDK
        https://github.com/modelcontextprotocol/python-sdk/blob/ca3466666310dbcb5c45690ac2571c574759984f/src/mcp/client/auth.py#L474-L483
        """
        for url in discovery_urls:
            # Question: Do we need to provide header regarding MCP protocol version?
            # Original code snippet:
            # return httpx.Request("GET", url, headers={MCP_PROTOCOL_VERSION: LATEST_PROTOCOL_VERSION}) # noqa: E501

            try:
                response = httpx.get(url, timeout=httpx.Timeout(5.0))
            except httpx.RequestError:
                # Network error; try next URL
                continue

            if response.status_code != 200:
                # Not 200 OK; try next URL
                continue

            content = response.read()
            metadata = OAuthMetadata.model_validate_json(content)
            self.context.oauth_metadata = metadata

            # Apply default scope if needed
            # if self.context.client_metadata.scope is None and metadata.scopes_supported is not None: # noqa: E501
            #     self.context.client_metadata.scope = " ".join(metadata.scopes_supported)
            break

    def metadata_discovery(self) -> OAuthMetadata:
        try:
            init_response = httpx.get(self.context.server_url, timeout=httpx.Timeout(5.0))

            # Step 1: Discover protected resource metadata (RFC9728 with WWW-Authenticate support)
            try:
                self._discover_protected_resource(init_response)
            except Exception:
                logger.error(
                    f"Protected resource metadata discovery failed: {self.context.server_url}"
                )
                # Do not raise an error, as this is not a critical error, we can still proceed with
                # the discovery process

            # Step 2: Discover OAuth metadata (with fallback for legacy servers)
            discovery_urls = self._get_discovery_urls()
            self._get_oauth_metadata(discovery_urls)

            if self.context.oauth_metadata is None:
                raise OAuth2MetadataDiscoveryError(
                    f"OAuth metadata not found for server URL: {self.context.server_url}"
                )

            return self.context.oauth_metadata

        except Exception as e:
            raise OAuth2MetadataDiscoveryError(
                f"Metadata discovery failed: {self.context.server_url}"
            ) from e
