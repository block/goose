import datetime
from uuid import UUID

from sqlalchemy.orm import Session

from aci.common.db.sql_models import User, UserRefreshToken
from aci.common.enums import UserIdentityProvider


def create_user(
    db_session: Session,
    name: str,
    email: str,
    password_hash: str | None,
    identity_provider: UserIdentityProvider,
    email_verified: bool,
) -> User:
    user = User(
        name=name,
        email=email,
        email_verified=email_verified,
        password_hash=password_hash,
        identity_provider=identity_provider,
    )
    db_session.add(user)
    db_session.flush()
    db_session.refresh(user)
    return user


def get_user_by_email(db_session: Session, email: str) -> User | None:
    return db_session.query(User).filter(User.email == email).first()


def get_user_by_id(db_session: Session, user_id: UUID) -> User | None:
    return db_session.query(User).filter(User.id == user_id).first()


def create_refresh_token(
    db_session: Session,
    user_id: UUID,
    token_hash: str,
    expires_at: datetime.datetime,
) -> UserRefreshToken:
    refresh_token = UserRefreshToken(
        user_id=user_id,
        token_hash=token_hash,
        expires_at=expires_at,
    )
    db_session.add(refresh_token)
    db_session.flush()
    db_session.refresh(refresh_token)
    return refresh_token


def get_refresh_token(db_session: Session, token_hash: str) -> UserRefreshToken | None:
    return (
        db_session.query(UserRefreshToken)
        .filter(UserRefreshToken.token_hash == token_hash)
        .filter(UserRefreshToken.deleted_at.is_(None))
        .filter(UserRefreshToken.expires_at > datetime.datetime.now(datetime.UTC))
        .first()
    )


def delete_refresh_token(db_session: Session, token_hash: str) -> None:
    db_session.query(UserRefreshToken).filter(UserRefreshToken.token_hash == token_hash).filter(
        UserRefreshToken.deleted_at.is_(None)
    ).update({UserRefreshToken.deleted_at: datetime.datetime.now(datetime.UTC)})
