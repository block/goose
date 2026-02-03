import datetime
from typing import Any
from uuid import UUID

from sqlalchemy.orm import Session

from aci.common.db.sql_models import UserVerification
from aci.common.enums import UserVerificationType


def invalidate_unused_verifications(
    db_session: Session,
    user_id: UUID,
    verification_type: UserVerificationType,
    used_at: datetime.datetime,
) -> None:
    """Mark unused verifications of the given type as used."""

    (
        db_session.query(UserVerification)
        .filter(UserVerification.user_id == user_id)
        .filter(UserVerification.type == verification_type)
        .filter(UserVerification.used_at.is_(None))
        .update({UserVerification.used_at: used_at}, synchronize_session=False)
    )


def create_verification(
    db_session: Session,
    user_id: UUID,
    verification_type: UserVerificationType,
    token_hash: str,
    expires_at: datetime.datetime,
    email_metadata: dict[str, Any] | None,
) -> UserVerification:
    """Persist a verification record."""

    verification = UserVerification(
        user_id=user_id,
        type=verification_type,
        token_hash=token_hash,
        expires_at=expires_at,
        email_metadata=email_metadata,
    )
    db_session.add(verification)
    db_session.flush()
    db_session.refresh(verification)
    return verification


def get_unused_verification_by_token_hash(
    db_session: Session,
    token_hash: str,
    verification_type: UserVerificationType,
) -> UserVerification | None:
    """Fetch an unused verification record by hash and type."""

    return (
        db_session.query(UserVerification)
        .filter(UserVerification.token_hash == token_hash)
        .filter(UserVerification.type == verification_type)
        .filter(UserVerification.used_at.is_(None))
        .first()
    )
