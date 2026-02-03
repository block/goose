from typing import Any, Literal

from pydantic import AnyHttpUrl, AnyUrl, BaseModel, Field

"""
All models in this file are adapted from the Official MCP Python SDK, in this file:
https://github.com/modelcontextprotocol/python-sdk/blob/ca3466666310dbcb5c45690ac2571c574759984f/src/mcp/shared/auth.py

These are models that corresponds to the RFC Spec of OAuth2 Discovery and Dynamic Client
Registration Flow.
"""


class InvalidScopeError(Exception):
    pass


class InvalidRedirectUriError(Exception):
    pass


class ProtectedResourceMetadata(BaseModel):
    """
    RFC 9728 OAuth 2.0 Protected Resource Metadata.
    See https://datatracker.ietf.org/doc/html/rfc9728#section-2
    """

    resource: AnyHttpUrl
    authorization_servers: list[AnyHttpUrl] = Field(..., min_length=1)
    jwks_uri: AnyHttpUrl | None = None
    scopes_supported: list[str] | None = None
    bearer_methods_supported: list[str] | None = Field(
        default=["header"]
    )  # MCP only supports header method
    resource_signing_alg_values_supported: list[str] | None = None
    resource_name: str | None = None
    resource_documentation: AnyHttpUrl | None = None
    resource_policy_uri: AnyHttpUrl | None = None
    resource_tos_uri: AnyHttpUrl | None = None
    # tls_client_certificate_bound_access_tokens default is False, but ommited here for clarity
    tls_client_certificate_bound_access_tokens: bool | None = None
    authorization_details_types_supported: list[str] | None = None
    dpop_signing_alg_values_supported: list[str] | None = None
    # dpop_bound_access_tokens_required default is False, but ommited here for clarity
    dpop_bound_access_tokens_required: bool | None = None


class OAuthClientMetadata(BaseModel):
    """
    RFC 7591 OAuth 2.0 Dynamic Client Registration metadata.
    See https://datatracker.ietf.org/doc/html/rfc7591#section-2
    for the full specification.
    """

    redirect_uris: list[AnyUrl] = Field(..., min_length=1)
    # token_endpoint_auth_method: this implementation only supports none &
    # client_secret_post;
    # ie: we do not support client_secret_basic
    token_endpoint_auth_method: Literal["none", "client_secret_post"] = "client_secret_post"
    # grant_types: this implementation only supports authorization_code & refresh_token
    grant_types: list[Literal["authorization_code", "refresh_token"]] = [
        "authorization_code",
        "refresh_token",
    ]
    # this implementation only supports code; ie: it does not support implicit grants
    response_types: list[Literal["code"]] = ["code"]
    scope: str | None = None

    # these fields are currently unused, but we support & store them for potential
    # future use
    client_name: str | None = None
    client_uri: AnyHttpUrl | None = None
    logo_uri: AnyHttpUrl | None = None
    contacts: list[str] | None = None
    tos_uri: AnyHttpUrl | None = None
    policy_uri: AnyHttpUrl | None = None
    jwks_uri: AnyHttpUrl | None = None
    jwks: Any | None = None
    software_id: str | None = None
    software_version: str | None = None

    def validate_scope(self, requested_scope: str | None) -> list[str] | None:
        if requested_scope is None:
            return None
        requested_scopes = requested_scope.split(" ")
        allowed_scopes = [] if self.scope is None else self.scope.split(" ")
        for scope in requested_scopes:
            if scope not in allowed_scopes:
                raise InvalidScopeError(f"Client was not registered with scope {scope}")
        return requested_scopes

    def validate_redirect_uri(self, redirect_uri: AnyUrl | None) -> AnyUrl:
        if redirect_uri is not None:
            # Validate redirect_uri against client's registered redirect URIs
            if redirect_uri not in self.redirect_uris:
                raise InvalidRedirectUriError(
                    f"Redirect URI '{redirect_uri}' not registered for client"
                )
            return redirect_uri
        elif len(self.redirect_uris) == 1:
            return self.redirect_uris[0]
        else:
            raise InvalidRedirectUriError(
                "redirect_uri must be specified when client has multiple registered URIs"
            )


class OAuthClientInformationFull(OAuthClientMetadata):
    """
    RFC 7591 OAuth 2.0 Dynamic Client Registration full response
    (client information plus metadata).
    """

    client_id: str
    client_secret: str | None = None
    client_id_issued_at: int | None = None
    client_secret_expires_at: int | None = None


class OAuthMetadata(BaseModel):
    """
    RFC 8414 OAuth 2.0 Authorization Server Metadata.
    See https://datatracker.ietf.org/doc/html/rfc8414#section-2
    """

    issuer: AnyHttpUrl
    authorization_endpoint: AnyHttpUrl
    token_endpoint: AnyHttpUrl
    registration_endpoint: AnyHttpUrl | None = None
    scopes_supported: list[str] | None = None
    response_types_supported: list[str] = ["code"]
    response_modes_supported: list[str] | None = None
    grant_types_supported: list[str] | None = None
    token_endpoint_auth_methods_supported: list[str] | None = None
    token_endpoint_auth_signing_alg_values_supported: list[str] | None = None
    service_documentation: AnyHttpUrl | None = None
    ui_locales_supported: list[str] | None = None
    op_policy_uri: AnyHttpUrl | None = None
    op_tos_uri: AnyHttpUrl | None = None
    revocation_endpoint: AnyHttpUrl | None = None
    revocation_endpoint_auth_methods_supported: list[str] | None = None
    revocation_endpoint_auth_signing_alg_values_supported: list[str] | None = None
    introspection_endpoint: AnyHttpUrl | None = None
    introspection_endpoint_auth_methods_supported: list[str] | None = None
    introspection_endpoint_auth_signing_alg_values_supported: list[str] | None = None
    code_challenge_methods_supported: list[str] | None = None
