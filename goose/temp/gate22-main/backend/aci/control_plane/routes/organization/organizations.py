from typing import Annotated
from uuid import UUID

from fastapi import APIRouter, Depends, HTTPException, status

from aci.common.db import crud
from aci.common.enums import OrganizationRole
from aci.common.logging_setup import get_logger
from aci.common.schemas.organization import (
    CreateOrganizationRequest,
    CreateOrganizationTeamRequest,
    OrganizationInfo,
    OrganizationMembershipInfo,
    TeamInfo,
    TeamMembershipInfo,
    UpdateOrganizationMemberRoleRequest,
)
from aci.control_plane import access_control
from aci.control_plane import dependencies as deps
from aci.control_plane.services.orphan_records_remover import OrphanRecordsRemover

logger = get_logger(__name__)
router = APIRouter()


@router.post("", response_model=OrganizationInfo, status_code=status.HTTP_201_CREATED)
async def create_organization(
    context: Annotated[deps.RequestContextWithoutActAs, Depends(deps.get_request_context_no_orgs)],
    request: CreateOrganizationRequest,
) -> OrganizationInfo:
    # Every logged in user can create an organization. No permission check.

    # Check if organization name already been used
    if crud.organizations.get_organization_by_name(context.db_session, request.name):
        logger.error(f"Organization name {request.name} already been used")
        raise HTTPException(
            status_code=status.HTTP_400_BAD_REQUEST,
            detail="Organization name already been used",
        )

    # Create organization
    organization = crud.organizations.create_organization(
        db_session=context.db_session,
        name=request.name,
        description=request.description,
    )

    # Add user into organization. First user must be Admin
    crud.organizations.add_user_to_organization(
        db_session=context.db_session,
        organization_id=organization.id,
        user_id=context.user_id,
        role=OrganizationRole.ADMIN,
    )

    context.db_session.commit()

    return OrganizationInfo(
        organization_id=organization.id,
        name=organization.name,
        description=organization.description,
    )


# ------------------------------------------------------------
#
# Organization Memberships Management
#
# ------------------------------------------------------------
@router.get(
    "/{organization_id}/members",
    response_model=list[OrganizationMembershipInfo],
    status_code=status.HTTP_200_OK,
)
async def list_organization_members(
    context: Annotated[deps.RequestContext, Depends(deps.get_request_context)],
    organization_id: UUID,
) -> list[OrganizationMembershipInfo]:
    # Check user's role permission
    access_control.check_act_as_organization_role(
        context.act_as, requested_organization_id=organization_id
    )

    # Get organization members
    organization_members = crud.organizations.get_organization_members(
        db_session=context.db_session,
        organization_id=organization_id,
    )
    return [
        OrganizationMembershipInfo(
            user_id=member.user_id,
            name=member.user.name,
            email=member.user.email,
            role=member.role,
            created_at=member.created_at,
        )
        for member in organization_members
    ]


@router.delete("/{organization_id}/members/{user_id}", status_code=status.HTTP_200_OK)
async def remove_organization_member(
    context: Annotated[deps.RequestContext, Depends(deps.get_request_context)],
    organization_id: UUID,
    user_id: UUID,
) -> None:
    # Check user's role permission
    access_control.check_act_as_organization_role(
        context.act_as, requested_organization_id=organization_id
    )

    # Admin can remove anyone in the organization
    if context.act_as.role == OrganizationRole.ADMIN:
        # Check if user is the last admin in the organization, if so, raise an error
        organization_members = crud.organizations.get_organization_members(
            db_session=context.db_session,
            organization_id=organization_id,
        )
        admins = list(
            filter(lambda member: member.role == OrganizationRole.ADMIN, organization_members)
        )
        if len(admins) == 1 and admins[0].user_id == user_id:
            logger.error(f"Cannot remove the last admin in the organization {organization_id}")
            raise HTTPException(
                status_code=status.HTTP_403_FORBIDDEN,
                detail="Cannot remove the last admin in the organization",
            )
    # Member can only remove themselves
    elif context.act_as.role == OrganizationRole.MEMBER:
        if context.user_id != user_id:
            logger.error("Non-admin cannot remove other members from the organization")
            raise HTTPException(
                status_code=status.HTTP_403_FORBIDDEN,
                detail="Non-admin cannot remove other members from the organization",
            )

    # All checks pass. Now remove member
    crud.organizations.remove_organization_member(
        db_session=context.db_session,
        organization_id=organization_id,
        user_id=user_id,
    )

    context.db_session.commit()


@router.patch("/{organization_id}/members/{user_id}", status_code=status.HTTP_200_OK)
async def update_organization_member_role(
    context: Annotated[deps.RequestContext, Depends(deps.get_request_context)],
    organization_id: UUID,
    user_id: UUID,
    request: UpdateOrganizationMemberRoleRequest,
) -> None:
    # Check user's role permission
    access_control.check_act_as_organization_role(
        context.act_as,
        requested_organization_id=organization_id,
        required_role=OrganizationRole.ADMIN,
    )

    # Check if user is the last admin in the organization, if so, raise an error
    organization_members = crud.organizations.get_organization_members(
        db_session=context.db_session,
        organization_id=organization_id,
    )
    admins = list(
        filter(lambda member: member.role == OrganizationRole.ADMIN, organization_members)
    )
    # If the targeted user is last admin, and the request is to remove the admin role, raise error
    if len(admins) == 1 and admins[0].user_id == user_id and request.role != OrganizationRole.ADMIN:
        logger.error(f"Cannot downgrade the last admin in the organization {organization_id}")
        raise HTTPException(
            status_code=status.HTTP_403_FORBIDDEN,
            detail="Cannot downgrade the last admin in the organization",
        )

    # Update member role
    crud.organizations.update_organization_member_role(
        db_session=context.db_session,
        organization_id=organization_id,
        user_id=user_id,
        role=request.role,
    )

    context.db_session.commit()


# ------------------------------------------------------------
#
# Team Management
#
# ------------------------------------------------------------
@router.post(
    "/{organization_id}/teams", response_model=TeamInfo, status_code=status.HTTP_201_CREATED
)
async def create_team(
    context: Annotated[deps.RequestContext, Depends(deps.get_request_context)],
    organization_id: UUID,
    request: CreateOrganizationTeamRequest,
) -> TeamInfo:
    # Check user's role permission
    access_control.check_act_as_organization_role(
        context.act_as,
        requested_organization_id=organization_id,
        required_role=OrganizationRole.ADMIN,
    )

    # Check if team name already exists
    if crud.teams.get_team_by_organization_id_and_name(
        context.db_session, organization_id, request.name
    ):
        logger.error(f"Team name {request.name} already exists in organization {organization_id}")
        raise HTTPException(
            status_code=status.HTTP_400_BAD_REQUEST,
            detail="Team name already exists",
        )

    # Validate member IDs if provided
    if request.member_user_ids:
        # De-duplicate while preserving order
        member_ids = list(dict.fromkeys(request.member_user_ids))
        # Check that all users exist and are members of the organization
        org_members = crud.organizations.get_organization_members(
            db_session=context.db_session,
            organization_id=organization_id,
        )
        org_member_ids = {member.user_id for member in org_members}

        invalid_user_ids = [user_id for user_id in member_ids if user_id not in org_member_ids]

        if invalid_user_ids:
            logger.error(f"Invalid user IDs: {invalid_user_ids}")
            raise HTTPException(
                status_code=status.HTTP_400_BAD_REQUEST,
                detail=f"Some users are not members of the organization: {invalid_user_ids}",
            )

    # Create team
    team = crud.teams.create_team(
        db_session=context.db_session,
        organization_id=organization_id,
        name=request.name,
        description=request.description,
    )

    # Add initial members if provided
    if request.member_user_ids:
        # Reuse the validated, de-duplicated list
        for user_id in member_ids:
            crud.teams.add_team_member(
                db_session=context.db_session,
                organization_id=organization_id,
                team_id=team.id,
                user_id=user_id,
            )

    context.db_session.commit()

    return TeamInfo(
        team_id=team.id,
        name=team.name,
        description=team.description,
        created_at=team.created_at,
    )


@router.get(
    "/{organization_id}/teams", response_model=list[TeamInfo], status_code=status.HTTP_200_OK
)
async def list_teams(
    context: Annotated[deps.RequestContext, Depends(deps.get_request_context)],
    organization_id: UUID,
) -> list[TeamInfo]:
    # Check user's role permission
    access_control.check_act_as_organization_role(
        context.act_as, requested_organization_id=organization_id
    )

    # Get organization teams
    teams = crud.teams.get_teams_by_organization_id(
        db_session=context.db_session,
        organization_id=organization_id,
    )
    return [
        TeamInfo(
            team_id=team.id,
            name=team.name,
            description=team.description,
            created_at=team.created_at,
        )
        for team in teams
    ]


@router.get(
    "/{organization_id}/teams/{team_id}/members",
    response_model=list[TeamMembershipInfo],
    status_code=status.HTTP_200_OK,
)
async def list_team_members(
    context: Annotated[deps.RequestContext, Depends(deps.get_request_context)],
    organization_id: UUID,
    team_id: UUID,
) -> list[TeamMembershipInfo]:
    # Check user's role permission
    access_control.check_act_as_organization_role(
        context.act_as, requested_organization_id=organization_id
    )

    # Get team members
    team_members = crud.teams.get_team_members(
        db_session=context.db_session,
        team_id=team_id,
    )
    return [
        TeamMembershipInfo(
            user_id=member.user_id,
            name=member.user.name,
            email=member.user.email,
            role=member.role,
            created_at=member.created_at,
        )
        for member in team_members
    ]


@router.put("/{organization_id}/teams/{team_id}/members/{user_id}", status_code=status.HTTP_200_OK)
async def add_team_member(
    context: Annotated[deps.RequestContext, Depends(deps.get_request_context)],
    organization_id: UUID,
    team_id: UUID,
    user_id: UUID,
) -> None:
    # Check user's role permission
    access_control.check_act_as_organization_role(
        context.act_as,
        requested_organization_id=organization_id,
        required_role=OrganizationRole.ADMIN,
    )

    # Check if user is a member of the organization
    if not crud.organizations.get_organization_membership(
        context.db_session, organization_id, user_id
    ):
        logger.error(f"User {user_id} is not a member of the organization {organization_id}")
        raise HTTPException(
            status_code=status.HTTP_400_BAD_REQUEST,
            detail="User is not a member of the organization",
        )

    # Check if targeted user is already a member of the team
    team_members = crud.teams.get_team_members(context.db_session, team_id)
    if any(member.user_id == user_id for member in team_members):
        logger.error(f"User {user_id} already a member of the team {team_id}")
        raise HTTPException(
            status_code=status.HTTP_400_BAD_REQUEST,
            detail="User already a member of the team",
        )

    # Add team member
    crud.teams.add_team_member(
        db_session=context.db_session,
        organization_id=organization_id,
        team_id=team_id,
        user_id=user_id,
    )

    context.db_session.commit()

    return None


@router.delete(
    "/{organization_id}/teams/{team_id}/members/{user_id}", status_code=status.HTTP_200_OK
)
async def remove_team_member(
    context: Annotated[deps.RequestContext, Depends(deps.get_request_context)],
    organization_id: UUID,
    team_id: UUID,
    user_id: UUID,
) -> None:
    # Check user's role permission
    access_control.check_act_as_organization_role(
        context.act_as, requested_organization_id=organization_id
    )

    # Admin can remove anyone in the team
    if context.act_as.role == OrganizationRole.ADMIN:
        # No blocking.
        pass

    # Member can only remove themselves
    elif context.act_as.role == OrganizationRole.MEMBER:
        if context.user_id != user_id:
            raise HTTPException(
                status_code=status.HTTP_403_FORBIDDEN,
                detail="Non-admin cannot remove other members from team",
            )
    else:
        raise HTTPException(status_code=status.HTTP_403_FORBIDDEN)

    # Check if targeted user is a member of the team
    team_members = crud.teams.get_team_members(context.db_session, team_id)
    if not any(member.user_id == user_id for member in team_members):
        raise HTTPException(
            status_code=status.HTTP_400_BAD_REQUEST, detail="User is not a member of the team"
        )

    # Remove team member
    crud.teams.remove_team_member(
        db_session=context.db_session,
        organization_id=organization_id,
        team_id=team_id,
        user_id=user_id,
    )

    removal_result = OrphanRecordsRemover(context.db_session).on_user_removed_from_team(
        user_id=user_id,
        organization_id=organization_id,
    )
    logger.info(f"Orphan records removal: {removal_result}")

    context.db_session.commit()

    return None
