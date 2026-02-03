from typing import Annotated

from fastapi import APIRouter, Depends, HTTPException, status

from aci.common.db import crud
from aci.common.logging_setup import get_logger
from aci.common.schemas.user import UserOrganizationInfo, UserSelfProfile
from aci.control_plane import dependencies as deps

logger = get_logger(__name__)
router = APIRouter()


@router.get("/me/profile", response_model=UserSelfProfile, status_code=status.HTTP_200_OK)
async def profile(
    context: Annotated[deps.RequestContext, Depends(deps.get_request_context_no_orgs)],
) -> UserSelfProfile:
    user = crud.users.get_user_by_id(context.db_session, context.user_id)

    # Should never happen as the user_id is validated in the JWT payload
    if not user:
        raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail="User not found")

    return UserSelfProfile(
        user_id=user.id,
        name=user.name,
        email=user.email,
        organizations=[
            UserOrganizationInfo(
                organization_id=org_membership.organization_id,
                organization_name=org_membership.organization.name,
                role=org_membership.role,
            )
            for org_membership in user.organization_memberships
        ],
    )
