import datetime
from collections.abc import Generator
from typing import cast
from unittest.mock import AsyncMock, MagicMock
from urllib.parse import parse_qs, urlparse

import httpx
import jwt
import pytest
from fastapi.testclient import TestClient
from sqlalchemy.orm import Session

from aci.common import utils
from aci.common.db import crud
from aci.common.db.sql_models import User, UserVerification
from aci.common.enums import UserIdentityProvider, UserVerificationType
from aci.control_plane import config
from aci.control_plane import token_utils as token_utils


@pytest.fixture
def unverified_user(db_session: Session) -> User:
    """Create an unverified user for testing."""
    password_hash = utils.hash_user_password("TestPassword123!")
    user = crud.users.create_user(
        db_session=db_session,
        name="Unverified User",
        email="unverified@example.com",
        password_hash=password_hash,
        identity_provider=UserIdentityProvider.EMAIL,
        email_verified=False,
    )
    db_session.commit()
    return user


@pytest.fixture
def verified_user(db_session: Session) -> User:
    """Create a verified user for testing."""
    return _create_verified_user(
        db_session,
        name="Verified User",
        email="verified@example.com",
        password="VerifiedPass123!",
    )


@pytest.fixture
def mock_email_service(test_client: TestClient) -> Generator[MagicMock, None, None]:
    """Mock the email service to avoid actual AWS SES calls."""
    # Import here to avoid circular imports
    from aci.control_plane import dependencies as deps
    from aci.control_plane.services.email_service import EmailService

    # Create a mock email service
    mock_service = MagicMock(spec=EmailService)

    # Create an async mock that returns the email metadata
    async def mock_send_verification_email(
        recipient: str, user_name: str, verification_url: str
    ) -> dict[str, str]:
        return {
            "email_recipient": recipient,
            "email_provider": "aws",
            "email_send_at": "2025-01-17T12:00:00Z",
            "email_reference_id": "test-message-id",
        }

    mock_service.send_verification_email = AsyncMock(side_effect=mock_send_verification_email)

    # Import the FastAPI app
    from aci.control_plane.main import app

    # Override the dependency
    app.dependency_overrides[deps.get_email_service] = lambda: mock_service

    yield mock_service

    # Clean up the override
    del app.dependency_overrides[deps.get_email_service]


def _create_verified_user(db_session: Session, name: str, email: str, password: str) -> User:
    """Helper to create a verified user."""
    password_hash = utils.hash_user_password(password)
    user = crud.users.create_user(
        db_session=db_session,
        name=name,
        email=email,
        password_hash=password_hash,
        identity_provider=UserIdentityProvider.EMAIL,
        email_verified=True,
    )
    db_session.commit()
    return user


def _create_verification_record(
    db_session: Session,
    user: User,
    token: str | None = None,
    expired: bool = False,
    used: bool = False,
) -> tuple[str, UserVerification]:
    """Helper to create a verification record with optional token generation."""
    if token is None:
        token, token_hash, expires_at = token_utils.generate_verification_token(
            user_id=user.id,
            email=user.email,
            verification_type="email_verification",
        )
    else:
        token_hash = token_utils.hash_token(token)
        if expired:
            expires_at = datetime.datetime.now(datetime.UTC) - datetime.timedelta(hours=25)
        else:
            expires_at = datetime.datetime.now(datetime.UTC) + datetime.timedelta(hours=24)

    verification = UserVerification(
        user_id=user.id,
        type=UserVerificationType.EMAIL_VERIFICATION,
        token_hash=token_hash,
        expires_at=expires_at,
        email_metadata=None,
    )

    if used:
        verification.used_at = datetime.datetime.now(datetime.UTC)

    db_session.add(verification)
    db_session.commit()
    return token, verification


def _assert_email_service_called(
    mock_email_service: MagicMock, expected_email: str, expected_name: str
) -> None:
    """Assert email service was called with expected parameters."""
    mock_email_service.send_verification_email.assert_called_once()
    call_args = mock_email_service.send_verification_email.call_args
    assert call_args[1]["recipient"] == expected_email
    assert call_args[1]["user_name"] == expected_name


def _assert_redirect_response(
    response: httpx.Response, expected_status: int, expected_location_contains: str
) -> None:
    """Assert redirect response matches expectations."""
    assert response.status_code == expected_status
    assert expected_location_contains in response.headers["location"]


def extract_token_from_mock(mock_email_service: MagicMock) -> str:
    """Extract verification token from mocked email service call."""
    call_args = mock_email_service.send_verification_email.call_args
    if call_args:
        verification_url = cast(str, call_args[1]["verification_url"])
        token = verification_url.split("token=")[-1]
        return token
    return ""


class TestEmailRegistration:
    def test_register_with_email_sends_verification(
        self,
        db_session: Session,
        test_client: TestClient,
        mock_email_service: MagicMock,
    ) -> None:
        """Test that registration sends verification email and sets email_verified=False."""
        response = test_client.post(
            f"{config.APP_ROOT_PATH}{config.ROUTER_PREFIX_AUTH}/register/email",
            json={
                "name": "Test User",
                "email": "newuser@example.com",
                "password": "SecurePass123!",
            },
        )

        assert response.status_code == 201

        # Check email service was called
        _assert_email_service_called(mock_email_service, "newuser@example.com", "Test User")

        # Check user was created with email_verified=False
        user = crud.users.get_user_by_email(db_session, "newuser@example.com")
        assert user is not None
        assert user.email_verified is False
        assert user.identity_provider == UserIdentityProvider.EMAIL

        # Check verification record was created
        verification = (
            db_session.query(UserVerification).filter(UserVerification.user_id == user.id).first()
        )
        assert verification is not None
        assert verification.type == "email_verification"
        assert verification.used_at is None

    def test_register_existing_unverified_email(
        self,
        db_session: Session,
        unverified_user: User,
        test_client: TestClient,
        mock_email_service: MagicMock,
    ) -> None:
        """Test re-registration with unverified email updates info and resends verification."""
        # Try to register again with same email
        response = test_client.post(
            f"{config.APP_ROOT_PATH}{config.ROUTER_PREFIX_AUTH}/register/email",
            json={
                "name": "Updated Name",
                "email": unverified_user.email,
                "password": "NewPassword123!",
            },
        )

        assert response.status_code == 201

        # Check email service was called
        mock_email_service.send_verification_email.assert_called_once()

        # Check user info was updated
        db_session.refresh(unverified_user)
        assert unverified_user.name == "Updated Name"
        # Password update is tested by successful login in other tests

        # Check new verification was created
        new_verifications = (
            db_session.query(UserVerification)
            .filter(
                UserVerification.user_id == unverified_user.id,
                UserVerification.type == "email_verification",
                UserVerification.used_at.is_(None),
            )
            .all()
        )
        assert len(new_verifications) >= 1

    def test_register_existing_verified_email(
        self, test_client: TestClient, verified_user: User, mock_email_service: MagicMock
    ) -> None:
        """Test registration fails for already verified emails."""

        # Try to register with same email
        response = test_client.post(
            f"{config.APP_ROOT_PATH}{config.ROUTER_PREFIX_AUTH}/register/email",
            json={
                "name": "Another User",
                "email": verified_user.email,
                "password": "NewPassword123!",
            },
        )

        assert response.status_code == 400
        # Check error message exists in response
        error_response = response.json()
        assert (
            "error" in error_response or "message" in error_response or "detail" in error_response
        )
        # The error indicates the email is already in use


class TestEmailLogin:
    def test_login_blocked_for_unverified_email(
        self, test_client: TestClient, unverified_user: User
    ) -> None:
        """Test that login is blocked for unverified email accounts."""
        response = test_client.post(
            f"{config.APP_ROOT_PATH}{config.ROUTER_PREFIX_AUTH}/login/email",
            json={
                "email": unverified_user.email,
                "password": "TestPassword123!",
            },
        )

        assert response.status_code == 403
        assert "Email not verified" in response.json()["detail"]

    def test_login_allowed_for_verified_email(
        self, test_client: TestClient, verified_user: User
    ) -> None:
        """Test that login works for verified email accounts."""

        response = test_client.post(
            f"{config.APP_ROOT_PATH}{config.ROUTER_PREFIX_AUTH}/login/email",
            json={
                "email": verified_user.email,
                "password": "VerifiedPass123!",
            },
        )

        assert response.status_code == 200  # Successful login
        assert "refresh_token" in response.cookies


class TestEmailUserVerification:
    def test_verify_email_valid_token(
        self, test_client: TestClient, db_session: Session, unverified_user: User
    ) -> None:
        """Test successful email verification with valid token."""
        # Generate a valid token and create verification record
        token, verification = _create_verification_record(db_session, unverified_user)

        # Verify email
        response = test_client.get(
            f"{config.APP_ROOT_PATH}{config.ROUTER_PREFIX_AUTH}/verify-email",
            params={"token": token},
        )

        _assert_redirect_response(response, 302, "/auth/verify-success")

        # Check user is now verified
        db_session.refresh(unverified_user)
        assert unverified_user.email_verified is True

        # Check verification is marked as used
        db_session.refresh(verification)
        assert verification.used_at is not None

    def test_verify_email_invalid_token(self, test_client: TestClient) -> None:
        """Test email verification fails with invalid token."""
        response = test_client.get(
            f"{config.APP_ROOT_PATH}{config.ROUTER_PREFIX_AUTH}/verify-email",
            params={"token": "invalid-token-12345"},
        )

        _assert_redirect_response(response, 302, "/auth/verify-error")
        parsed = urlparse(response.headers["location"])
        query_params = parse_qs(parsed.query)
        assert query_params.get("error") == ["invalid_email_verification_token"]

    def test_verify_email_expired_token(
        self, test_client: TestClient, db_session: Session, unverified_user: User
    ) -> None:
        """Test email verification fails with expired token."""
        # Generate an expired token
        now = datetime.datetime.now(datetime.UTC)
        expired_at = now - datetime.timedelta(hours=25)

        payload = {
            "type": "email_verification",
            "email": unverified_user.email,
            "user_id": str(unverified_user.id),
            "iat": int((now - datetime.timedelta(hours=26)).timestamp()),
            "exp": int(expired_at.timestamp()),
        }

        # Create token with expired timestamp
        token = jwt.encode(payload, config.JWT_SIGNING_KEY, algorithm=config.JWT_ALGORITHM)
        token_hash = token_utils.hash_token(token)

        # Create verification record
        verification = UserVerification(
            user_id=unverified_user.id,
            type=UserVerificationType.EMAIL_VERIFICATION,
            token_hash=token_hash,
            expires_at=expired_at,
            email_metadata=None,
        )
        db_session.add(verification)
        db_session.commit()

        # Try to verify with expired token
        response = test_client.get(
            f"{config.APP_ROOT_PATH}{config.ROUTER_PREFIX_AUTH}/verify-email",
            params={"token": token},
        )

        _assert_redirect_response(response, 302, "/auth/verify-error")
        parsed = urlparse(response.headers["location"])
        query_params = parse_qs(parsed.query)
        assert query_params.get("error") == ["email_verification_token_expired"]

        # User should still be unverified
        db_session.refresh(unverified_user)
        assert unverified_user.email_verified is False

    def test_verify_email_already_used_token(
        self, test_client: TestClient, db_session: Session, unverified_user: User
    ) -> None:
        """Test that verification token can't be reused."""
        # Generate token and create verification record marked as used
        token, _ = _create_verification_record(db_session, unverified_user, used=True)

        # Try to use the already-used token
        response = test_client.get(
            f"{config.APP_ROOT_PATH}{config.ROUTER_PREFIX_AUTH}/verify-email",
            params={"token": token},
        )

        _assert_redirect_response(response, 302, "/auth/verify-error")
        parsed = urlparse(response.headers["location"])
        query_params = parse_qs(parsed.query)
        assert query_params.get("error") == ["email_verification_token_not_found"]
