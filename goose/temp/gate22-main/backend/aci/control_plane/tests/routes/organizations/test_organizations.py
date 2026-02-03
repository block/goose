import pytest
from fastapi.testclient import TestClient
from sqlalchemy.orm import Session

from aci.common.db import crud
from aci.common.db.sql_models import Organization, User
from aci.common.enums import OrganizationRole, UserIdentityProvider
from aci.common.logging_setup import get_logger
from aci.common.schemas.organization import (
    CreateOrganizationRequest,
    OrganizationInfo,
    OrganizationMembershipInfo,
    UpdateOrganizationMemberRoleRequest,
)
from aci.control_plane import config

logger = get_logger(__name__)


def test_create_organization(
    db_session: Session,
    test_client: TestClient,
    dummy_user_without_org: User,
    dummy_access_token_no_orgs: str,
) -> None:
    test_input = CreateOrganizationRequest(
        name="Test Org",
        description="Test Description",
    )

    response = test_client.post(
        config.ROUTER_PREFIX_ORGANIZATIONS,
        json=test_input.model_dump(mode="json"),
        headers={"Authorization": f"Bearer {dummy_access_token_no_orgs}"},
    )

    assert response.status_code == 201
    organization = OrganizationInfo.model_validate(response.json())
    assert organization.name == test_input.name
    assert organization.description == test_input.description

    # Check if organization is created in database
    db_org = crud.organizations.get_organization_by_name(db_session, test_input.name)
    assert db_org is not None
    assert db_org.name == "Test Org"

    # Check if user is added to organization as admin
    organization_membership = crud.organizations.get_organization_membership(
        db_session, organization.organization_id, dummy_user_without_org.id
    )
    assert organization_membership is not None
    assert organization_membership.user_id == dummy_user_without_org.id
    assert organization_membership.role == OrganizationRole.ADMIN


def test_create_organization_with_existing_name(
    db_session: Session,
    test_client: TestClient,
    dummy_access_token_no_orgs: str,
) -> None:
    test_input = CreateOrganizationRequest(
        name="Test Org",
        description="Test Description",
    )

    # Add a organization with the same name
    crud.organizations.create_organization(db_session, test_input.name)
    db_session.commit()

    # Try to create a organization with the same name
    response = test_client.post(
        config.ROUTER_PREFIX_ORGANIZATIONS,
        json=test_input.model_dump(mode="json"),
        headers={"Authorization": f"Bearer {dummy_access_token_no_orgs}"},
    )
    assert response.status_code == 400
    assert response.json() == {"detail": "Organization name already been used"}


@pytest.mark.parametrize(
    "access_token_fixture",
    [
        "dummy_access_token_no_orgs",
        "dummy_access_token_admin",
        "dummy_access_token_member",
        "dummy_access_token_admin_act_as_member",
        "dummy_access_token_another_org",
    ],
)
def test_list_organization_members(
    request: pytest.FixtureRequest,
    db_session: Session,
    test_client: TestClient,
    access_token_fixture: str,
    dummy_organization: Organization,
    dummy_user: User,
) -> None:
    access_token = request.getfixturevalue(access_token_fixture)

    response = test_client.get(
        f"{config.ROUTER_PREFIX_ORGANIZATIONS}/{dummy_organization.id}/members",
        headers={"Authorization": f"Bearer {access_token}"},
    )

    if access_token_fixture in ["dummy_access_token_no_orgs", "dummy_access_token_another_org"]:
        assert response.status_code == 403
        return

    assert response.status_code == 200

    organization_members = [
        OrganizationMembershipInfo.model_validate(member) for member in response.json()
    ]

    # Only the current user is in the organization
    assert len(organization_members) == 1
    assert organization_members[0].user_id == dummy_user.id

    # Add a new member to the organization
    new_member = crud.users.create_user(
        db_session,
        name="New Member",
        email="new_member@example.com",
        password_hash=None,
        identity_provider=UserIdentityProvider.EMAIL,
        email_verified=True,
    )
    crud.organizations.add_user_to_organization(
        db_session, dummy_organization.id, new_member.id, OrganizationRole.MEMBER
    )
    db_session.commit()

    response = test_client.get(
        f"{config.ROUTER_PREFIX_ORGANIZATIONS}/{dummy_organization.id}/members",
        headers={"Authorization": f"Bearer {access_token}"},
    )
    organization_members = [
        OrganizationMembershipInfo.model_validate(member) for member in response.json()
    ]

    # Now there should be two members
    assert len(organization_members) == 2
    # Sorted by created_at desc
    assert organization_members[0].user_id == new_member.id


@pytest.mark.parametrize(
    "access_token_fixture",
    [
        "dummy_access_token_no_orgs",
        "dummy_access_token_admin",
        "dummy_access_token_member",
        "dummy_access_token_admin_act_as_member",
        "dummy_access_token_another_org",
    ],
)
@pytest.mark.parametrize("remove_member_role", [OrganizationRole.MEMBER, OrganizationRole.ADMIN])
def test_remove_organization_member(
    request: pytest.FixtureRequest,
    db_session: Session,
    test_client: TestClient,
    access_token_fixture: str,
    dummy_organization: Organization,
    dummy_user: User,
    remove_member_role: OrganizationRole,
) -> None:
    access_token = request.getfixturevalue(access_token_fixture)

    # Add a new member to the organization
    new_member = crud.users.create_user(
        db_session,
        name="New Member",
        email="new_member@example.com",
        password_hash=None,
        identity_provider=UserIdentityProvider.EMAIL,
        email_verified=True,
    )
    crud.organizations.add_user_to_organization(
        db_session, dummy_organization.id, new_member.id, remove_member_role
    )
    db_session.commit()

    # Check if the new member is added to the organization
    organization_membership = crud.organizations.get_organization_membership(
        db_session, dummy_organization.id, new_member.id
    )
    assert organization_membership is not None

    # Test removing the new member
    response = test_client.delete(
        f"{config.ROUTER_PREFIX_ORGANIZATIONS}/{dummy_organization.id}/members/{new_member.id}",
        headers={"Authorization": f"Bearer {access_token}"},
    )

    if access_token_fixture in ["dummy_access_token_no_orgs", "dummy_access_token_another_org"]:
        # Non-member cannot leave the organization
        assert response.status_code == 403
        return

    elif access_token_fixture in [
        "dummy_access_token_member",
        "dummy_access_token_admin_act_as_member",
    ]:
        # Non-admin cannot remove other members from the organization
        assert response.status_code == 403
        assert (
            response.json()["detail"]
            == "Non-admin cannot remove other members from the organization"
        )
        return

    else:
        assert response.status_code == 200

        # Check if the new member is removed from the organization
        organization_membership = crud.organizations.get_organization_membership(
            db_session, dummy_organization.id, new_member.id
        )
        assert organization_membership is None


@pytest.mark.parametrize(
    "access_token_fixture",
    [
        "dummy_access_token_no_orgs",
        "dummy_access_token_admin",
        "dummy_access_token_member",
        "dummy_access_token_admin_act_as_member",
        "dummy_access_token_another_org",
    ],
)
@pytest.mark.parametrize("initial_admin_count", [1, 2])
def test_leave_organization(
    request: pytest.FixtureRequest,
    db_session: Session,
    test_client: TestClient,
    access_token_fixture: str,
    dummy_organization: Organization,
    dummy_user: User,
    initial_admin_count: int,
) -> None:
    access_token = request.getfixturevalue(access_token_fixture)

    # Add initial admins to the organization
    org_members = crud.organizations.get_organization_members(db_session, dummy_organization.id)
    current_admin_count = len(
        [member for member in org_members if member.role == OrganizationRole.ADMIN]
    )
    for i in range(initial_admin_count - current_admin_count):
        new_admin = crud.users.create_user(
            db_session,
            name=f"New Admin {i}",
            email=f"new_admin{i}@example.com",
            password_hash=None,
            identity_provider=UserIdentityProvider.EMAIL,
            email_verified=True,
        )
        crud.organizations.add_user_to_organization(
            db_session, dummy_organization.id, new_admin.id, OrganizationRole.ADMIN
        )
    db_session.commit()

    # Test leaving the organization
    response = test_client.delete(
        f"{config.ROUTER_PREFIX_ORGANIZATIONS}/{dummy_organization.id}/members/{dummy_user.id}",
        headers={"Authorization": f"Bearer {access_token}"},
    )

    if access_token_fixture in ["dummy_access_token_no_orgs", "dummy_access_token_another_org"]:
        # Non-member cannot leave the organization
        assert response.status_code == 403
        return

    elif access_token_fixture in ["dummy_access_token_admin"] and initial_admin_count == 1:
        # Last admin cannot leave the organization
        assert response.status_code == 403
        assert response.json()["detail"] == "Cannot remove the last admin in the organization"
        return

    else:
        assert response.status_code == 200

        # Check if the user is removed from the organization
        organization_membership = crud.organizations.get_organization_membership(
            db_session, dummy_organization.id, dummy_user.id
        )
        assert organization_membership is None


@pytest.mark.parametrize(
    "access_token_fixture",
    [
        "dummy_access_token_no_orgs",
        "dummy_access_token_admin",
        "dummy_access_token_member",
        "dummy_access_token_admin_act_as_member",
        "dummy_access_token_another_org",
    ],
)
@pytest.mark.parametrize("change_self", [True, False])
@pytest.mark.parametrize("target_role", [OrganizationRole.MEMBER, OrganizationRole.ADMIN])
def test_update_organization_member_role(
    request: pytest.FixtureRequest,
    db_session: Session,
    test_client: TestClient,
    access_token_fixture: str,
    dummy_organization: Organization,
    dummy_user: User,
    change_self: bool,
    target_role: OrganizationRole,
) -> None:
    access_token = request.getfixturevalue(access_token_fixture)

    if change_self:
        target_user = dummy_user
    else:
        target_user = crud.users.create_user(
            db_session,
            name="New Member",
            email="new_member@example.com",
            password_hash=None,
            identity_provider=UserIdentityProvider.EMAIL,
            email_verified=True,
        )
        crud.organizations.add_user_to_organization(
            db_session, dummy_organization.id, target_user.id, OrganizationRole.ADMIN
        )
    db_session.commit()

    # Test updating the role of the target user
    input_body = UpdateOrganizationMemberRoleRequest(role=target_role)

    response = test_client.patch(
        f"{config.ROUTER_PREFIX_ORGANIZATIONS}/{dummy_organization.id}/members/{target_user.id}",
        headers={"Authorization": f"Bearer {access_token}"},
        json=input_body.model_dump(mode="json"),
    )

    # only admin can update the role
    if access_token_fixture not in ["dummy_access_token_admin"]:
        assert response.status_code == 403
        return

    # Get the target memberships
    target_membership = crud.organizations.get_organization_membership(
        db_session, dummy_organization.id, target_user.id
    )
    assert target_membership is not None

    memberships = crud.organizations.get_organization_members(db_session, dummy_organization.id)
    admin_count = len([member for member in memberships if member.role == OrganizationRole.ADMIN])

    # If target user is the only admin, it cannot be downgraded to member
    if (
        admin_count == 1
        and target_membership.role == OrganizationRole.ADMIN
        and target_role == OrganizationRole.MEMBER
    ):
        assert response.status_code == 403
        assert response.json()["detail"] == "Cannot downgrade the last admin in the organization"
    else:
        # Check if the role of the target user is updated
        assert response.status_code == 200
        assert target_membership.role == target_role
