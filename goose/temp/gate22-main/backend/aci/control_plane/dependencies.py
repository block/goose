from collections.abc import Generator
from dataclasses import dataclass
from typing import Annotated
from uuid import UUID

import jwt
from fastapi import Depends, HTTPException, status
from fastapi.security import HTTPAuthorizationCredentials, HTTPBearer
from sqlalchemy.orm import Session

from aci.common import utils
from aci.common.db import crud
from aci.common.enums import OrganizationRole
from aci.common.logging_setup import get_logger
from aci.common.schemas.auth import ActAsInfo, JWTPayload
from aci.control_plane import config
from aci.control_plane.services.email_service import EmailService

logger = get_logger(__name__)
http_bearer = HTTPBearer(auto_error=True, description="login to receive a JWT token")

logger = get_logger(__name__)


@dataclass
class RequestContextWithoutActAs:
    db_session: Session
    user_id: UUID


@dataclass
class RequestContext:
    db_session: Session
    user_id: UUID
    act_as: ActAsInfo


# TODO: should we rollback in caught exceptions?
# except Exception:
#    db_session.rollback()
#    raise
def yield_db_session() -> Generator[Session, None, None]:
    db_session = utils.create_db_session(config.DB_FULL_URL)
    try:
        yield db_session
    finally:
        db_session.close()


def get_jwt_payload(
    credentials: Annotated[HTTPAuthorizationCredentials, Depends(http_bearer)],
) -> JWTPayload:
    try:
        decoded_token = jwt.decode(
            credentials.credentials, config.JWT_SIGNING_KEY, algorithms=[config.JWT_ALGORITHM]
        )
        jwt_payload = JWTPayload(**decoded_token)
        return jwt_payload
    except Exception as e:
        logger.error(f"Error decoding JWT token: {e}")
        raise HTTPException(status_code=401, detail="Token invalid") from e


def get_request_context_no_orgs(
    db_session: Annotated[Session, Depends(yield_db_session)],
    jwt_payload: Annotated[JWTPayload, Depends(get_jwt_payload)],
) -> RequestContextWithoutActAs:
    """
    This dependency is used in endpoints that requires user to be authenticated but expect user to
    not act as any organization and role.
    For example: POST /organizations
    """
    return RequestContextWithoutActAs(
        db_session=db_session,
        user_id=jwt_payload.user_id,
    )


def get_request_context(
    db_session: Annotated[Session, Depends(yield_db_session)],
    jwt_payload: Annotated[JWTPayload, Depends(get_jwt_payload)],
) -> RequestContext:
    """
    This method validates whether the user is permitted to act as the desired organization and role.
    Returns a RequestContext object containing the DB session, user_id and act_as information.
    """
    if jwt_payload.act_as:
        logger.info(f"JWT payload: {jwt_payload}")
        # Check if organization exists
        if jwt_payload.act_as.organization_id:
            organization = crud.organizations.get_organization_by_id(
                db_session, jwt_payload.act_as.organization_id
            )
            if not organization:
                logger.error(
                    f"Failed to act as organization {jwt_payload.act_as.organization_id}"
                    f"(role: {jwt_payload.act_as.role}). "
                    f"Organization does not exist"
                )
                raise HTTPException(status_code=status.HTTP_404_NOT_FOUND)

        # Check if user is a member of the organization
        membership = crud.organizations.get_organization_membership(
            db_session, jwt_payload.act_as.organization_id, jwt_payload.user_id
        )
        if not membership:
            logger.error(
                f"Failed to act as organization {jwt_payload.act_as.organization_id}"
                f"(role: {jwt_payload.act_as.role}). "
                f"User {jwt_payload.user_id} is not a member of the organization"
            )
            raise HTTPException(status_code=status.HTTP_403_FORBIDDEN)

        # Check if user has the required role in the organization
        if not is_role_valid(jwt_payload.act_as.role, membership.role):
            logger.error(
                f"Failed to act as organization {jwt_payload.act_as.organization_id}"
                f"(role: {jwt_payload.act_as.role}). "
                f"Role not allowed"
            )
            raise HTTPException(status_code=status.HTTP_403_FORBIDDEN)

    else:
        logger.error("The JWT payload is missing act_as information")
        raise HTTPException(status_code=status.HTTP_403_FORBIDDEN)

    return RequestContext(
        db_session=db_session,
        user_id=jwt_payload.user_id,
        act_as=jwt_payload.act_as,
    )


# TODO: Improve this function when we have more roles
def is_role_valid(act_as_role: OrganizationRole, membership_role: OrganizationRole) -> bool:
    # Admin can act as any role
    if membership_role == OrganizationRole.ADMIN:
        return True
    # Member can only act as member
    if membership_role == OrganizationRole.MEMBER:
        return act_as_role == OrganizationRole.MEMBER


def get_email_service() -> EmailService:
    """
    Dependency for email service.
    This allows easy mocking in tests via dependency overrides.
    """
    return EmailService()
