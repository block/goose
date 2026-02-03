"""CRUD helpers for organization invitations."""

import datetime
from typing import Any
from uuid import UUID

from sqlalchemy.orm import Session

from aci.common.db.sql_models import OrganizationInvitation
from aci.common.enums import OrganizationInvitationStatus, OrganizationRole
from aci.common.schemas.organization_invitation import OrganizationInvitationUpdate


def create_invitation(
    db_session: Session,
    *,
    organization_id: UUID,
    email: str,
    inviter_user_id: UUID,
    role: OrganizationRole,
    token_hash: str,
    expires_at: datetime.datetime,
    email_metadata: dict[str, Any] | None,
) -> OrganizationInvitation:
    invitation = OrganizationInvitation(
        organization_id=organization_id,
        email=email.lower(),
        inviter_user_id=inviter_user_id,
        role=role,
        token_hash=token_hash,
        expires_at=expires_at,
        email_metadata=email_metadata,
        status=OrganizationInvitationStatus.PENDING,
    )
    db_session.add(invitation)
    db_session.flush()
    db_session.refresh(invitation)
    return invitation


def get_invitation_by_id(
    db_session: Session,
    invitation_id: UUID,
) -> OrganizationInvitation | None:
    return (
        db_session.query(OrganizationInvitation)
        .filter(OrganizationInvitation.id == invitation_id)
        .first()
    )


def get_pending_invitation_by_email(
    db_session: Session,
    organization_id: UUID,
    email: str,
) -> OrganizationInvitation | None:
    return (
        db_session.query(OrganizationInvitation)
        .filter(OrganizationInvitation.organization_id == organization_id)
        .filter(OrganizationInvitation.email == email.lower())
        .filter(OrganizationInvitation.status == OrganizationInvitationStatus.PENDING)
        .first()
    )


def get_invitation_by_email(
    db_session: Session,
    organization_id: UUID,
    email: str,
) -> OrganizationInvitation | None:
    return (
        db_session.query(OrganizationInvitation)
        .filter(OrganizationInvitation.organization_id == organization_id)
        .filter(OrganizationInvitation.email == email.lower())
        .first()
    )


def get_invitation_by_token_hash(
    db_session: Session,
    token_hash: str,
) -> OrganizationInvitation | None:
    return (
        db_session.query(OrganizationInvitation)
        .filter(OrganizationInvitation.token_hash == token_hash)
        .first()
    )


def list_invitations(
    db_session: Session,
    organization_id: UUID,
    status: OrganizationInvitationStatus | None = None,
) -> list[OrganizationInvitation]:
    query = db_session.query(OrganizationInvitation).filter(
        OrganizationInvitation.organization_id == organization_id
    )
    if status is not None:
        query = query.filter(OrganizationInvitation.status == status)
    return query.order_by(OrganizationInvitation.created_at.desc()).all()


def update_invitation(
    db_session: Session,
    invitation: OrganizationInvitation,
    invitation_update: OrganizationInvitationUpdate,
) -> OrganizationInvitation:
    for field, value in invitation_update.model_dump(exclude_unset=True).items():
        setattr(invitation, field, value)

    db_session.refresh(invitation)
    return invitation
