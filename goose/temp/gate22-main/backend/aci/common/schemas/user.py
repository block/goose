from uuid import UUID

from pydantic import BaseModel

from aci.common.enums import OrganizationRole


class UserOrganizationInfo(BaseModel):
    organization_id: UUID
    organization_name: str
    role: OrganizationRole


# Only used in getting own user profile, which discloses organization information
class UserSelfProfile(BaseModel):
    user_id: UUID
    name: str
    email: str
    organizations: list[UserOrganizationInfo]


class UserPublic(BaseModel):
    id: UUID
    name: str
    email: str
