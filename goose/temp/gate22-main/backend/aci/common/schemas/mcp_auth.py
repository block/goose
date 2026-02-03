from typing import Annotated, Literal

from pydantic import BaseModel, Field, RootModel

from aci.common.enums import AuthType, HttpLocation


class NoAuthConfig(BaseModel, extra="forbid"):
    type: Literal[AuthType.NO_AUTH] = AuthType.NO_AUTH


class APIKeyConfig(BaseModel):
    type: Literal[AuthType.API_KEY] = AuthType.API_KEY
    location: HttpLocation = Field(
        ...,
        description="The location of the API key in the request, e.g., 'header'",
    )
    name: str = Field(
        ...,
        description="The name of the API key in the request, e.g., 'X-Subscription-Token'",
    )
    prefix: str | None = Field(
        default=None,
        description="""The prefix of the API key in the request, e.g., 'Bearer'.
        If None, no prefix will be used.""",
    )


class OAuth2Config(BaseModel):
    type: Literal[AuthType.OAUTH2] = AuthType.OAUTH2
    location: HttpLocation = Field(
        HttpLocation.HEADER,
        description="The location of the OAuth2 access token in the request, e.g., 'header'",
    )
    name: str = Field(
        "Authorization",
        description="The name of the OAuth2 access token in the request, e.g., 'Authorization'",
    )
    prefix: str = Field(
        "Bearer",
        description="The prefix of the OAuth2 access token in the request, e.g., 'Bearer'",
    )
    client_id: str = Field(
        ...,
        min_length=1,
        max_length=2048,
        description="The client ID of the OAuth2 client (provided by ACI) used for the app",
    )
    # NOTE:remote oauth2 based mcp servers use dynamic client registration
    # and most of them don't provide client secret.
    client_secret: str | None = Field(
        default=None,
        min_length=1,
        max_length=2048,
        description="The client secret of the OAuth2 client (provided by ACI) used for the app",
    )
    scope: str = Field(
        ...,
        description="""Space separated scopes of the OAuth2 client (provided by ACI) used for the
        app, e.g., 'openid email profile https://www.googleapis.com/auth/calendar'""",
    )
    authorize_url: str = Field(
        ...,
        description="The URL of the OAuth2 authorization server, e.g., 'https://accounts.google.com/o/oauth2/v2/auth'",
    )
    access_token_url: str = Field(
        ...,
        description="The URL of the OAuth2 access token server, e.g., 'https://oauth2.googleapis.com/token'",
    )
    refresh_token_url: str = Field(
        ...,
        description="The URL of the OAuth2 refresh token server, e.g., 'https://oauth2.googleapis.com/token'",
    )
    token_endpoint_auth_method: Literal["client_secret_basic", "client_secret_post"] | None = Field(
        default=None,
        description="""The authentication method for the OAuth2 token endpoint, e.g.,
        'client_secret_post' for some providers that require client_id/client_secret to be sent
        in the body of the token request, like Hubspot""",
    )


class AuthConfig(
    RootModel[Annotated[NoAuthConfig | APIKeyConfig | OAuth2Config, Field(discriminator="type")],]
):
    pass


class NoAuthCredentials(BaseModel, extra="forbid"):
    type: Literal[AuthType.NO_AUTH] = AuthType.NO_AUTH


class APIKeyCredentials(BaseModel):
    type: Literal[AuthType.API_KEY] = AuthType.API_KEY
    secret_key: str = Field(
        ...,
        description="The secret key of the API key",
    )


# TODO: reconsider client id, client secret, scope, etc should be stored together
# with the credentials
class OAuth2Credentials(BaseModel):
    type: Literal[AuthType.OAUTH2] = AuthType.OAUTH2
    access_token: str
    token_type: str | None = None
    expires_at: int | None = None
    refresh_token: str | None = None


class AuthCredentials(
    RootModel[
        Annotated[
            NoAuthCredentials | APIKeyCredentials | OAuth2Credentials,
            Field(discriminator="type"),
        ]
    ]
):
    pass
