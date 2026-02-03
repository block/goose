from datetime import UTC, datetime
from uuid import UUID, uuid4

from fastapi.testclient import TestClient
from sqlalchemy.orm import Session

from aci.common.db import crud
from aci.common.db.sql_models import MCPToolCallLog, User
from aci.common.enums import MCPToolCallStatus
from aci.common.schemas.mcp_tool_call_log import MCPToolCallLogCreate
from aci.control_plane import config


def create_test_log(
    db_session: Session,
    organization_id: UUID,
    user_id: UUID,
    mcp_tool_name: str = "TOOL__TEST",
) -> MCPToolCallLog:
    """Helper to create a test log directly in the database."""
    log_create = MCPToolCallLogCreate(
        organization_id=organization_id,
        user_id=user_id,
        request_id=str(uuid4()),
        session_id=uuid4(),
        bundle_name="test_bundle",
        bundle_id=uuid4(),
        mcp_server_name="test_server",
        mcp_server_id=uuid4(),
        mcp_tool_name=mcp_tool_name,
        mcp_tool_id=uuid4(),
        mcp_server_configuration_name="test_config",
        mcp_server_configuration_id=uuid4(),
        arguments='{"arg": "value"}',
        result={"result": "success"},
        status=MCPToolCallStatus.SUCCESS,
        via_execute_tool=False,
        jsonrpc_payload={"jsonrpc": "2.0"},
        started_at=datetime.now(UTC),
        ended_at=datetime.now(UTC),
        duration_ms=1000,
    )
    return crud.mcp_tool_call_logs.create(db_session, log_create)


def test_admin_sees_all_org_logs(
    test_client: TestClient,
    db_session: Session,
    dummy_member_2: User,
    dummy_member_3: User,
    dummy_access_token_admin: str,
) -> None:
    """Test that admin can see all logs from their organization."""
    org_id = dummy_member_2.organization_memberships[0].organization_id

    # Create logs for member user
    log1 = create_test_log(
        db_session=db_session,
        organization_id=org_id,
        user_id=dummy_member_2.id,
    )

    # Create logs for another member in the same org
    log2 = create_test_log(
        db_session=db_session,
        organization_id=org_id,
        user_id=dummy_member_3.id,
    )

    db_session.commit()

    # Admin should see both logs from their org
    response = test_client.get(
        f"{config.ROUTER_PREFIX_LOGS}/tool-calls",
        headers={"Authorization": f"Bearer {dummy_access_token_admin}"},
    )
    assert response.status_code == 200
    data = response.json()

    assert len(data["data"]) == 2
    log_ids = {UUID(log["id"]) for log in data["data"]}
    assert log1.id in log_ids
    assert log2.id in log_ids


def test_admin_cannot_see_other_org_logs(
    test_client: TestClient,
    db_session: Session,
    dummy_admin: User,
    dummy_access_token_admin: str,
) -> None:
    """Test that admin cannot see logs from other organizations."""
    org_id = dummy_admin.organization_memberships[0].organization_id

    # Create log for dummy_admin's org
    log1 = create_test_log(
        db_session=db_session,
        organization_id=org_id,
        user_id=dummy_admin.id,
    )

    # Create log for another org
    _ = create_test_log(
        db_session=db_session,
        organization_id=uuid4(),
        user_id=dummy_admin.id,
    )

    db_session.commit()

    response = test_client.get(
        f"{config.ROUTER_PREFIX_LOGS}/tool-calls",
        headers={"Authorization": f"Bearer {dummy_access_token_admin}"},
    )
    assert response.status_code == 200
    data = response.json()

    assert len(data["data"]) == 1, "Should only see logs from dummy_admin's org"
    returned_log = data["data"][0]
    assert returned_log["id"] == str(log1.id)
    assert data["next_cursor"] is None


def test_member_sees_only_own_logs(
    test_client: TestClient,
    db_session: Session,
    dummy_member_2: User,
    dummy_member_3: User,
    dummy_access_token_member_2: str,
) -> None:
    """Test that member can only see their own logs."""
    org_id = dummy_member_2.organization_memberships[0].organization_id

    # Create logs for member user
    log1 = create_test_log(
        db_session=db_session,
        organization_id=org_id,
        user_id=dummy_member_2.id,
    )
    log2 = create_test_log(
        db_session=db_session,
        organization_id=org_id,
        user_id=dummy_member_2.id,
    )

    # Create logs for another member in the same org
    log3 = create_test_log(
        db_session=db_session,
        organization_id=org_id,
        user_id=dummy_member_3.id,
    )

    db_session.commit()

    # Member should only see their own logs
    response = test_client.get(
        f"{config.ROUTER_PREFIX_LOGS}/tool-calls",
        headers={"Authorization": f"Bearer {dummy_access_token_member_2}"},
    )
    assert response.status_code == 200
    data = response.json()

    assert len(data["data"]) == 2
    log_ids = {UUID(log["id"]) for log in data["data"]}
    assert log1.id in log_ids
    assert log2.id in log_ids
    assert log3.id not in log_ids, "Should not see another member's log"


def test_member_cannot_see_other_org_logs(
    test_client: TestClient,
    db_session: Session,
    dummy_member_2: User,
    dummy_access_token_member_2: str,
) -> None:
    """Test that member cannot see logs from other organizations."""

    # Create log for random org
    create_test_log(
        db_session=db_session,
        organization_id=uuid4(),
        user_id=uuid4(),
    )

    db_session.commit()

    # Member from another org should not see this log
    response = test_client.get(
        f"{config.ROUTER_PREFIX_LOGS}/tool-calls",
        headers={"Authorization": f"Bearer {dummy_access_token_member_2}"},
    )
    assert response.status_code == 200
    data = response.json()

    assert len(data["data"]) == 0, "Should not see logs from another org"


def test_admin_acting_as_member_sees_only_own_logs(
    test_client: TestClient,
    db_session: Session,
    dummy_admin: User,
    dummy_member_2: User,
    dummy_access_token_admin_act_as_member: str,
) -> None:
    """Test that admin acting as member can only see their own logs."""
    org_id = dummy_admin.organization_memberships[0].organization_id

    # Create logs for admin user
    log1 = create_test_log(
        db_session=db_session,
        organization_id=org_id,
        user_id=dummy_admin.id,
    )

    # Create logs for another member in the same org
    _ = create_test_log(
        db_session=db_session,
        organization_id=org_id,
        user_id=dummy_member_2.id,
    )

    db_session.commit()

    # Admin acting as member should only see their own logs
    response = test_client.get(
        f"{config.ROUTER_PREFIX_LOGS}/tool-calls",
        headers={"Authorization": f"Bearer {dummy_access_token_admin_act_as_member}"},
    )
    assert response.status_code == 200
    data = response.json()

    assert len(data["data"]) == 1
    assert data["data"][0]["id"] == str(log1.id)
    assert data["next_cursor"] is None


def test_access_control_with_pagination_and_filter(
    test_client: TestClient,
    db_session: Session,
    dummy_admin: User,
    dummy_member_2: User,
    dummy_access_token_admin: str,
    dummy_access_token_member_2: str,
) -> None:
    """Test that access control works correctly with pagination."""
    org_id = dummy_admin.organization_memberships[0].organization_id

    # Create multiple logs for both users in the org
    for _ in range(3):
        create_test_log(
            db_session=db_session,
            organization_id=org_id,
            user_id=dummy_admin.id,
            mcp_tool_name="TOOL__A",
        )
        create_test_log(
            db_session=db_session,
            organization_id=org_id,
            user_id=dummy_member_2.id,
            mcp_tool_name="TOOL__B",
        )

    db_session.commit()

    # Admin should see all 6 logs from their org with pagination
    response = test_client.get(
        f"{config.ROUTER_PREFIX_LOGS}/tool-calls?limit=4",
        headers={"Authorization": f"Bearer {dummy_access_token_admin}"},
    )
    assert response.status_code == 200
    data = response.json()

    assert len(data["data"]) == 4
    assert data["next_cursor"] is not None

    # Get next page
    response = test_client.get(
        f"{config.ROUTER_PREFIX_LOGS}/tool-calls?limit=4&cursor={data['next_cursor']}",
        headers={"Authorization": f"Bearer {dummy_access_token_admin}"},
    )
    assert response.status_code == 200
    data = response.json()

    assert len(data["data"]) == 2
    assert data["next_cursor"] is None

    # Admin should see only TOOL__A logs with filter
    response = test_client.get(
        f"{config.ROUTER_PREFIX_LOGS}/tool-calls?limit=2&mcp_tool_name=TOOL__A",
        headers={"Authorization": f"Bearer {dummy_access_token_admin}"},
    )
    assert response.status_code == 200
    data = response.json()
    assert len(data["data"]) == 2
    assert data["next_cursor"] is not None

    # Get next page
    response = test_client.get(
        f"{config.ROUTER_PREFIX_LOGS}/tool-calls?limit=2&cursor={data['next_cursor']}&mcp_tool_name=TOOL__A",
        headers={"Authorization": f"Bearer {dummy_access_token_admin}"},
    )
    assert response.status_code == 200
    data = response.json()
    assert len(data["data"]) == 1
    assert data["next_cursor"] is None

    # Member should see 3 logs
    response = test_client.get(
        f"{config.ROUTER_PREFIX_LOGS}/tool-calls?limit=2",
        headers={"Authorization": f"Bearer {dummy_access_token_member_2}"},
    )
    assert response.status_code == 200
    data = response.json()
    assert len(data["data"]) == 2
    assert data["next_cursor"] is not None

    # Get next page
    response = test_client.get(
        f"{config.ROUTER_PREFIX_LOGS}/tool-calls?limit=2&cursor={data['next_cursor']}",
        headers={"Authorization": f"Bearer {dummy_access_token_member_2}"},
    )
    assert response.status_code == 200
    data = response.json()
    assert len(data["data"]) == 1
    assert data["next_cursor"] is None


# def test_access_control_with_filter(
#     test_client: TestClient,
#     db_session: Session,
#     dummy_admin: User,
#     dummy_member_2: User,
#     dummy_access_token_admin: str,
#     dummy_access_token_member: str,
# ) -> None:
#     """Test that access control works correctly with tool name filter."""
#     org_id = dummy_admin.organization_memberships[0].organization_id

#     # Create logs with different tool names for different users
#     log1 = create_test_log(
#         db_session=db_session,
#         request_id="admin_tool_a",
#         organization_id=org_id,
#         user_id=dummy_admin.id,
#     )
#     log1.mcp_tool_name = "tool_a"

#     log2 = create_test_log(
#         db_session=db_session,
#         request_id="another_member_tool_a",
#         organization_id=org_id,
#         user_id=dummy_member_2.id,
#     )
#     log2.mcp_tool_name = "tool_a"

#     log3 = create_test_log(
#         db_session=db_session,
#         request_id="another_member_tool_b",
#         organization_id=org_id,
#         user_id=dummy_member_2.id,
#     )
#     log3.mcp_tool_name = "tool_b"

#     db_session.commit()

#     # Admin should see both tool_a logs
#     response = test_client.get(
#         f"{config.ROUTER_PREFIX_LOGS}/tool-calls?mcp_tool_name=tool_a",
#         headers={"Authorization": f"Bearer {dummy_access_token_admin}"},
#     )
#     assert response.status_code == 200
#     data = response.json()

#     assert len(data["data"]) == 2
#     request_ids = {log["request_id"] for log in data["data"]}
#     assert "admin_tool_a" in request_ids
#     assert "another_member_tool_a" in request_ids

#     # Change dummy_admin to member role for next test
#     membership = dummy_admin.organization_memberships[0]
#     from aci.common.enums import OrganizationRole

#     membership.role = OrganizationRole.MEMBER
#     db_session.commit()

#     # Create new token for member (reusing dummy_admin as member)
#     from aci.common import utils
#     from aci.common.schemas.auth import ActAsInfo
#     from aci.control_plane.tests.conftest import (
#         test_jwt_access_token_expire_minutes,
#         test_jwt_algorithm,
#         test_jwt_signing_key,
#     )

#     member_token = utils.sign_token(
#         user=dummy_admin,
#         act_as=ActAsInfo(organization_id=org_id, role=OrganizationRole.MEMBER),
#         jwt_signing_key=test_jwt_signing_key,
#         jwt_algorithm=test_jwt_algorithm,
#         jwt_access_token_expire_minutes=test_jwt_access_token_expire_minutes,
#     )

#     # Member should only see their own tool_a log
#     response = test_client.get(
#         f"{config.ROUTER_PREFIX_LOGS}/tool-calls?mcp_tool_name=tool_a",
#         headers={"Authorization": f"Bearer {member_token}"},
#     )
#     assert response.status_code == 200
#     data = response.json()

#     assert len(data["data"]) == 1
#     assert data["data"][0]["request_id"] == "admin_tool_a"
