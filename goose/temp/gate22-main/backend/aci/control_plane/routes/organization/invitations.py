"""Routes for managing organization invitations."""

import datetime
from typing import Annotated
from urllib.parse import quote
from uuid import UUID

from fastapi import APIRouter, Depends, HTTPException, Query, status

from aci.common import entitlement_utils, utils
from aci.common.db import crud
from aci.common.db.sql_models import OrganizationInvitation, User
from aci.common.enums import OrganizationInvitationStatus, OrganizationRole
from aci.common.logging_setup import get_logger
from aci.common.schemas.organization_invitation import (
    OrganizationInvitationDetail,
    OrganizationInvitationUpdate,
    SendOrganizationInvitationRequest,
)
from aci.control_plane import access_control, config, token_utils
from aci.control_plane import dependencies as deps
from aci.control_plane.exceptions import (
    InvalidInvitationTokenError,
    InvitationExpiredError,
    InvitationNotPendingError,
    UsageExceeded,
)
from aci.control_plane.services.email_service import EmailService

logger = get_logger(__name__)
router = APIRouter()


def _serialize_invitation(
    invitation: OrganizationInvitation, *, include_metadata: bool = True
) -> OrganizationInvitationDetail:
    return OrganizationInvitationDetail(
        invitation_id=invitation.id,
        organization_id=invitation.organization_id,
        email=invitation.email,
        inviter_user_id=invitation.inviter_user_id,
        inviter_name=invitation.inviter.name if invitation.inviter else None,
        role=invitation.role,
        status=invitation.status,
        expires_at=invitation.expires_at,
        used_at=invitation.used_at,
        created_at=invitation.created_at,
        updated_at=invitation.updated_at,
        email_metadata=invitation.email_metadata if include_metadata else None,
    )


def _validate_invitation_token(invitation: OrganizationInvitation, token: str) -> None:
    now = datetime.datetime.now(datetime.UTC)
    if invitation.status != OrganizationInvitationStatus.PENDING:
        raise InvitationNotPendingError()
    if invitation.expires_at < now:
        raise InvitationExpiredError()
    if token_utils.hash_token(token) != invitation.token_hash:
        raise InvalidInvitationTokenError()


def _ensure_user_can_access_invitation(
    context: deps.RequestContextWithoutActAs,
    invitation: OrganizationInvitation,
    *,
    expected_organization_id: UUID | None = None,
) -> User:
    if expected_organization_id and invitation.organization_id != expected_organization_id:
        raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail="Invitation not found")

    user = crud.users.get_user_by_id(context.db_session, context.user_id)
    if not user:
        raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail="User not found")

    membership = crud.organizations.get_organization_membership(
        context.db_session, invitation.organization_id, context.user_id
    )
    is_admin = membership is not None and membership.role == OrganizationRole.ADMIN

    if not is_admin and user.email.lower() != invitation.email.lower():
        raise HTTPException(status_code=status.HTTP_403_FORBIDDEN, detail="Not permitted")

    return user


def _require_invitation_access_by_token(
    context: deps.RequestContextWithoutActAs,
    token: str,
    *,
    expected_organization_id: UUID | None = None,
) -> tuple[OrganizationInvitation, User]:
    token_hash = token_utils.hash_token(token)
    invitation = crud.organization_invitations.get_invitation_by_token_hash(
        context.db_session, token_hash
    )
    if invitation is None:
        raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail="Invitation not found")

    user = _ensure_user_can_access_invitation(
        context, invitation, expected_organization_id=expected_organization_id
    )

    return invitation, user


@router.post(
    "/{organization_id}/invitations",
    response_model=OrganizationInvitationDetail,
    status_code=status.HTTP_201_CREATED,
)
async def create_invitation(
    context: Annotated[deps.RequestContext, Depends(deps.get_request_context)],
    email_service: Annotated[EmailService, Depends(deps.get_email_service)],
    organization_id: UUID,
    request: SendOrganizationInvitationRequest,
) -> OrganizationInvitationDetail:
    access_control.check_act_as_organization_role(
        context.act_as,
        requested_organization_id=organization_id,
        required_role=OrganizationRole.ADMIN,
    )

    target_email = request.email.strip()

    organization = crud.organizations.get_organization_by_id(context.db_session, organization_id)
    if not organization:
        raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail="Organization not found")

    # Prevent inviting existing members
    existing_user = crud.users.get_user_by_email(context.db_session, target_email)
    if existing_user:
        membership = crud.organizations.get_organization_membership(
            context.db_session, organization_id, existing_user.id
        )
        if membership:
            raise HTTPException(
                status_code=status.HTTP_409_CONFLICT,
                detail="User is already a member of the organization",
            )

    invitation = crud.organization_invitations.get_invitation_by_email(
        context.db_session, organization_id, target_email
    )

    token, token_hash, expires_at = token_utils.generate_invitation_token(
        config.ORGANIZATION_INVITATION_EXPIRE_MINUTES
    )

    if invitation is None:
        invitation = crud.organization_invitations.create_invitation(
            context.db_session,
            organization_id=organization_id,
            email=target_email,
            inviter_user_id=context.user_id,
            role=request.role,
            token_hash=token_hash,
            expires_at=expires_at,
            email_metadata=None,
        )
    else:
        if invitation.status == OrganizationInvitationStatus.ACCEPTED:
            raise HTTPException(
                status_code=status.HTTP_409_CONFLICT,
                detail="Invitation already accepted",
            )
        invitation = crud.organization_invitations.update_invitation(
            context.db_session,
            invitation,
            OrganizationInvitationUpdate(
                role=request.role,
                inviter_user_id=context.user_id,
                status=OrganizationInvitationStatus.PENDING,
                token_hash=token_hash,
                expires_at=expires_at,
                email_metadata=None,
                used_at=None,
            ),
        )

    inviter = crud.users.get_user_by_id(context.db_session, context.user_id)
    if not inviter:
        raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail="Inviter not found")
    inviter_name = inviter.name
    expires_label = utils.format_duration_from_minutes(
        config.ORGANIZATION_INVITATION_EXPIRE_MINUTES
    )

    try:
        invitation_query = f"?organization_id={quote(str(organization_id))}"
        invitation_url = (
            f"{config.FRONTEND_URL.rstrip('/')}/invite/{quote(token)}{invitation_query}"
        )

        email_metadata = await email_service.send_organization_invitation_email(
            recipient=target_email,
            organization_name=organization.name,
            inviter_name=inviter_name,
            invitation_url=invitation_url,
            expires_label=expires_label,
        )
    except Exception as exc:
        context.db_session.rollback()
        logger.exception("Failed to send organization invitation email")
        raise HTTPException(
            status_code=status.HTTP_503_SERVICE_UNAVAILABLE,
            detail="Failed to send invitation email. Please try again later.",
        ) from exc

    invitation = crud.organization_invitations.update_invitation(
        context.db_session,
        invitation,
        OrganizationInvitationUpdate(email_metadata=email_metadata),
    )

    context.db_session.commit()
    return _serialize_invitation(invitation)


@router.post(
    "/{organization_id}/invitations/accept/{token}",
    response_model=OrganizationInvitationDetail,
    status_code=status.HTTP_200_OK,
)
async def accept_invitation(
    context: Annotated[deps.RequestContextWithoutActAs, Depends(deps.get_request_context_no_orgs)],
    organization_id: UUID,
    token: str,
) -> OrganizationInvitationDetail:
    invitation, user = _require_invitation_access_by_token(
        context, token, expected_organization_id=organization_id
    )

    _validate_invitation_token(invitation, token)

    now = datetime.datetime.now(datetime.UTC)

    organization_id = invitation.organization_id

    if not user.email_verified:
        raise HTTPException(
            status_code=status.HTTP_403_FORBIDDEN,
            detail="Email not verified",
        )
    if user.email.lower() != invitation.email.lower():
        raise HTTPException(
            status_code=status.HTTP_403_FORBIDDEN,
            detail="Invitation email does not match the authenticated user",
        )

    existing_membership = crud.organizations.get_organization_membership(
        context.db_session, organization_id, context.user_id
    )
    if existing_membership:
        raise HTTPException(
            status_code=status.HTTP_409_CONFLICT,
            detail="User is already a member of the organization",
        )

    # Check if the usage exceeds the entitlement after inviting the new member
    if config.SUBSCRIPTION_ENABLED:
        entitlement = entitlement_utils.get_organization_entitlement(
            context.db_session, organization_id
        )
        usage = entitlement_utils.get_organization_usage(context.db_session, organization_id)
        usage.seat_count += 1
        if not entitlement_utils.is_entitlement_fulfilling_usage(
            entitlement=entitlement, usage=usage
        ):
            raise UsageExceeded(
                message="organization has reached usage limit. Please ask your admin to upgrade the subscription to continue.",  # noqa: E501
            )

    crud.organizations.add_user_to_organization(
        context.db_session,
        organization_id=organization_id,
        user_id=context.user_id,
        role=invitation.role,
    )

    invitation = crud.organization_invitations.update_invitation(
        context.db_session,
        invitation,
        OrganizationInvitationUpdate(
            status=OrganizationInvitationStatus.ACCEPTED,
            used_at=now,
        ),
    )

    context.db_session.commit()
    return _serialize_invitation(invitation)


@router.post(
    "/{organization_id}/invitations/reject/{token}",
    response_model=OrganizationInvitationDetail,
    status_code=status.HTTP_200_OK,
)
async def reject_invitation(
    context: Annotated[deps.RequestContextWithoutActAs, Depends(deps.get_request_context_no_orgs)],
    organization_id: UUID,
    token: str,
) -> OrganizationInvitationDetail:
    invitation, user = _require_invitation_access_by_token(
        context, token, expected_organization_id=organization_id
    )

    _validate_invitation_token(invitation, token)

    if user.email.lower() != invitation.email.lower():
        raise HTTPException(
            status_code=status.HTTP_403_FORBIDDEN,
            detail="Invitation email does not match the authenticated user",
        )

    invitation = crud.organization_invitations.update_invitation(
        context.db_session,
        invitation,
        OrganizationInvitationUpdate(
            status=OrganizationInvitationStatus.REJECTED,
            used_at=datetime.datetime.now(datetime.UTC),
        ),
    )

    context.db_session.commit()
    return _serialize_invitation(invitation)


@router.delete(
    "/{organization_id}/invitations/{invitation_id}",
    response_model=OrganizationInvitationDetail,
    status_code=status.HTTP_200_OK,
)
async def cancel_invitation(
    context: Annotated[deps.RequestContext, Depends(deps.get_request_context)],
    organization_id: UUID,
    invitation_id: UUID,
) -> OrganizationInvitationDetail:
    access_control.check_act_as_organization_role(
        context.act_as,
        requested_organization_id=organization_id,
        required_role=OrganizationRole.ADMIN,
    )

    invitation = crud.organization_invitations.get_invitation_by_id(
        context.db_session, invitation_id
    )
    if invitation is None or invitation.organization_id != organization_id:
        raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail="Invitation not found")
    if invitation.status != OrganizationInvitationStatus.PENDING:
        raise HTTPException(
            status_code=status.HTTP_409_CONFLICT,
            detail="Only pending invitations can be canceled",
        )

    invitation = crud.organization_invitations.update_invitation(
        context.db_session,
        invitation,
        OrganizationInvitationUpdate(
            status=OrganizationInvitationStatus.CANCELED,
        ),
    )

    context.db_session.commit()
    return _serialize_invitation(invitation)


@router.get(
    "/{organization_id}/invitations",
    response_model=list[OrganizationInvitationDetail],
    status_code=status.HTTP_200_OK,
)
async def list_invitations(
    context: Annotated[deps.RequestContext, Depends(deps.get_request_context)],
    organization_id: UUID,
    status_filter: Annotated[
        OrganizationInvitationStatus | None,
        Query(description="Filter by invitation status"),
    ] = None,
) -> list[OrganizationInvitationDetail]:
    access_control.check_act_as_organization_role(
        context.act_as, requested_organization_id=organization_id
    )

    invitations = crud.organization_invitations.list_invitations(
        context.db_session, organization_id, status=status_filter
    )

    return [_serialize_invitation(invitation, include_metadata=False) for invitation in invitations]


@router.get(
    "/{organization_id}/invitations/get/{token}",
    response_model=OrganizationInvitationDetail,
    status_code=status.HTTP_200_OK,
)
async def get_invitation_by_token(
    context: Annotated[deps.RequestContextWithoutActAs, Depends(deps.get_request_context_no_orgs)],
    organization_id: UUID,
    token: str,
) -> OrganizationInvitationDetail:
    invitation, _ = _require_invitation_access_by_token(
        context, token, expected_organization_id=organization_id
    )

    return _serialize_invitation(invitation)
