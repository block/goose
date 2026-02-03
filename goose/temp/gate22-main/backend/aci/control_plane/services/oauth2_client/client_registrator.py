from dataclasses import dataclass
from urllib.parse import urljoin, urlparse

import httpx
from pydantic import AnyHttpUrl, ValidationError

from aci.control_plane.exceptions import OAuth2ClientRegistrationError
from aci.control_plane.services.oauth2_client.schema import (
    OAuthClientInformationFull,
    OAuthClientMetadata,
)


@dataclass
class RegistrationContext:
    server_url: AnyHttpUrl
    client_metadata: OAuthClientMetadata
    registration_endpoint: AnyHttpUrl
    client_info: OAuthClientInformationFull | None = None

    def get_authorization_base_url(self, server_url: str) -> str:
        """Extract base URL by removing path component."""
        parsed = urlparse(server_url)
        return f"{parsed.scheme}://{parsed.netloc}"


class ClientRegistrator:
    def __init__(
        self,
        server_url: AnyHttpUrl,
        client_metadata: OAuthClientMetadata,
        registration_endpoint: AnyHttpUrl,
        client_info: OAuthClientInformationFull | None = None,
    ):
        self.context = RegistrationContext(
            server_url=server_url,
            client_metadata=client_metadata,
            registration_endpoint=registration_endpoint,
            client_info=client_info,
        )

    def _register_client(self) -> None:
        """
        Build registration request or skip if already registered.

        This function is adapted and modified from the Official MCP Python SDK
        https://github.com/modelcontextprotocol/python-sdk/blob/ca3466666310dbcb5c45690ac2571c574759984f/src/mcp/client/auth.py#L281-L310
        """
        if self.context.client_info:
            return

        if self.context.registration_endpoint:
            registration_url = str(self.context.registration_endpoint)
        else:
            # Fallback to common registration endpoint if not provided.
            auth_base_url = self.context.get_authorization_base_url(str(self.context.server_url))
            registration_url = urljoin(auth_base_url, "/register")

        registration_data = self.context.client_metadata.model_dump(
            by_alias=True, mode="json", exclude_none=True
        )

        try:
            response = httpx.post(
                registration_url,
                json=registration_data,
                headers={"Content-Type": "application/json"},
                timeout=10.0,
            )
        except httpx.RequestError as e:
            raise OAuth2ClientRegistrationError(f"Registration request failed: {e}") from e

        if response.status_code not in (200, 201):
            response.read()
            raise OAuth2ClientRegistrationError(
                f"Registration failed: {response.status_code} {response.text}"
            )

        try:
            content = response.read()
            client_info = OAuthClientInformationFull.model_validate_json(content)
            self.context.client_info = client_info
        except ValidationError as e:
            raise OAuth2ClientRegistrationError(f"Invalid registration response: {e}") from e

    def dynamic_client_registration(self) -> OAuthClientInformationFull:
        try:
            self._register_client()

            if self.context.client_info is None:
                raise OAuth2ClientRegistrationError("Client information not found")

            return self.context.client_info
        except Exception as e:
            raise OAuth2ClientRegistrationError(f"Client registration failed: {e}") from e
