import pytest
from fastapi.testclient import TestClient
from sqlalchemy.orm import Session

from aci.common.db.sql_models import User
from aci.common.schemas.user import UserSelfProfile
from aci.control_plane import config


@pytest.mark.parametrize(
    "access_token_fixture",
    [
        "dummy_access_token_no_orgs",
        "dummy_access_token_admin",
        "dummy_access_token_member",
    ],
)
def test_get_profile(
    request: pytest.FixtureRequest,
    test_client: TestClient,
    dummy_user: User,
    dummy_user_without_org: User,
    access_token_fixture: str,
) -> None:
    access_token = request.getfixturevalue(access_token_fixture)

    response = test_client.get(
        f"{config.ROUTER_PREFIX_USERS}/me/profile",
        headers={"Authorization": f"Bearer {access_token}"},
    )
    assert response.status_code == 200
    userinfo = UserSelfProfile.model_validate(response.json())

    if access_token_fixture in ["dummy_access_token_no_orgs"]:
        assert len(userinfo.organizations) == 0
        assert userinfo.user_id == dummy_user_without_org.id
        assert userinfo.name == dummy_user_without_org.name
        assert userinfo.email == dummy_user_without_org.email
    else:
        assert userinfo.user_id == dummy_user.id
        assert userinfo.name == dummy_user.name
        assert userinfo.email == dummy_user.email

        assert len(userinfo.organizations) == 1
        organization_membership = dummy_user.organization_memberships[0]
        assert userinfo.organizations[0].organization_id == organization_membership.organization_id
        assert (
            userinfo.organizations[0].organization_name == organization_membership.organization.name
        )
        assert userinfo.organizations[0].role == organization_membership.role


def test_get_profile_non_existence_user(
    test_client: TestClient,
    db_session: Session,
    dummy_user_without_org: User,
    dummy_access_token_no_orgs: str,
) -> None:
    # Remove the user
    db_session.query(User).filter(User.id == dummy_user_without_org.id).delete()
    db_session.commit()

    response = test_client.get(
        f"{config.ROUTER_PREFIX_USERS}/me/profile",
        headers={"Authorization": f"Bearer {dummy_access_token_no_orgs}"},
    )
    assert response.status_code == 404
    assert response.json() == {"detail": "User not found"}
