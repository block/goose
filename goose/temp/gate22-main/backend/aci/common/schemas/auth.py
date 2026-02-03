import re
from uuid import UUID

from pydantic import BaseModel, EmailStr, Field, field_validator

from aci.common.enums import OrganizationRole
from aci.common.url_utils import sanitize_redirect_path


class ActAsInfo(BaseModel):
    organization_id: UUID
    role: OrganizationRole


class JWTPayload(BaseModel):
    sub: str
    exp: int
    iat: int
    user_id: UUID
    name: str
    email: str
    act_as: ActAsInfo | None


class OAuth2State(BaseModel):
    code_verifier: str
    redirect_uri: str
    client_id: str
    post_oauth_redirect_uri: str


class EmailRegistrationRequest(BaseModel):
    name: str = Field(min_length=1, max_length=100, description="User name")
    email: EmailStr = Field(min_length=1, max_length=255, description="User email")
    password: str = Field(min_length=1, max_length=255, description="User password")
    redirect_path: str | None = Field(
        default=None,
        max_length=2048,
        description=("Optional relative path used to redirect the user after email verification."),
    )

    # TODO: Define password strength requirements
    @field_validator("password")
    @classmethod
    def strong_password(cls, v: str) -> str:
        if len(v) < 8:
            raise ValueError("Password must be at least 8 characters long")
        if not re.search(r"[A-Z]", v):
            raise ValueError("Password must contain an uppercase letter")
        if not re.search(r"[a-z]", v):
            raise ValueError("Password must contain a lowercase letter")
        if not re.search(r"\d", v):
            raise ValueError("Password must contain a digit")
        if not re.search(r"[@$!%*?&]", v):
            raise ValueError("Password must contain a special character (@$!%*?&)")
        return v

    @field_validator("redirect_path")
    @classmethod
    def validate_redirect_path(cls, v: str | None) -> str | None:
        if v is None:
            return None

        sanitized = sanitize_redirect_path(v)
        if sanitized is None:
            raise ValueError("redirect_path must be a relative path starting with '/'")

        return sanitized


class EmailLoginRequest(BaseModel):
    email: str = Field(min_length=1, max_length=255, description="User email")
    password: str = Field(min_length=1, max_length=255, description="User password")


class TokenResponse(BaseModel):
    token: str


class IssueTokenRequest(BaseModel):
    act_as: ActAsInfo | None = Field(
        default=None,
        description="""
        Act as organization and role. If not provided, it will use the last used organization
        and role.
        """,
    )


class UserOrganizationInfo(BaseModel):
    organization_id: UUID
    organization_name: str
    role: OrganizationRole
