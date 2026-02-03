import datetime
from uuid import UUID

from pydantic import BaseModel, Field

from aci.common.enums import OrganizationRole, TeamRole


class CreateOrganizationRequest(BaseModel):
    name: str = Field(min_length=1, max_length=100, description="Organization name")
    description: str | None = Field(
        default=None, max_length=255, description="Organization description"
    )


class OrganizationInfo(BaseModel):
    organization_id: UUID
    name: str
    description: str | None = None


class OrganizationMembershipInfo(BaseModel):
    user_id: UUID
    name: str
    email: str
    role: OrganizationRole
    created_at: datetime.datetime


class UpdateOrganizationMemberRoleRequest(BaseModel):
    role: OrganizationRole


class CreateOrganizationTeamRequest(BaseModel):
    name: str = Field(min_length=1, max_length=100, description="Team name")
    description: str | None = Field(default=None, max_length=255, description="Team description")
    member_user_ids: list[UUID] | None = Field(
        default=None, description="List of user IDs to add as initial team members"
    )


class TeamInfo(BaseModel):
    team_id: UUID
    name: str
    description: str | None = None
    created_at: datetime.datetime


class TeamMembershipInfo(BaseModel):
    user_id: UUID
    name: str
    email: str
    role: TeamRole
    created_at: datetime.datetime
