"""Pydantic models for organization invitation APIs."""

import datetime
from typing import Any
from uuid import UUID

from pydantic import BaseModel, ConfigDict, EmailStr, Field, model_validator

from aci.common.enums import OrganizationInvitationStatus, OrganizationRole


class SendOrganizationInvitationRequest(BaseModel, extra="forbid"):
    email: EmailStr = Field(description="Email address to invite")
    role: OrganizationRole = Field(description="Role to grant after accepting the invitation")


class RespondOrganizationInvitationRequest(BaseModel, extra="forbid"):
    token: str = Field(min_length=1, description="Raw invitation token supplied via email")


class CancelOrganizationInvitationRequest(BaseModel, extra="forbid"):
    invitation_id: UUID = Field(description="Identifier of the invitation to cancel")


class OrganizationInvitationUpdate(BaseModel):
    """Fields that can be updated on an organization invitation."""

    model_config = ConfigDict(extra="forbid")

    role: OrganizationRole | None = Field(default=None, description="Updated role")
    status: OrganizationInvitationStatus | None = Field(default=None, description="Updated status")
    token_hash: str | None = Field(
        default=None,
        min_length=1,
        description="New invitation token hash",
    )
    expires_at: datetime.datetime | None = Field(
        default=None,
        description="New expiration timestamp",
    )
    email_metadata: dict[str, Any] | None = Field(
        default=None, description="Metadata returned by the email provider"
    )
    used_at: datetime.datetime | None = Field(default=None, description="Timestamp when consumed")
    inviter_user_id: UUID | None = Field(default=None, description="Updated inviter user ID")

    @model_validator(mode="after")
    def _check_non_nullable_fields(self) -> "OrganizationInvitationUpdate":
        non_nullable_fields = ["role", "status", "token_hash", "expires_at", "inviter_user_id"]
        for field in self.model_fields_set:
            if field in non_nullable_fields and getattr(self, field) is None:
                raise ValueError(f"{field} cannot be None")
        return self


class OrganizationInvitationDetail(BaseModel):
    invitation_id: UUID
    organization_id: UUID
    email: EmailStr
    inviter_user_id: UUID
    inviter_name: str | None
    role: OrganizationRole
    status: OrganizationInvitationStatus
    expires_at: datetime.datetime
    used_at: datetime.datetime | None
    created_at: datetime.datetime
    updated_at: datetime.datetime
    email_metadata: dict[str, Any] | None = None
