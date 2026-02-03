from datetime import UTC, datetime, timedelta
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
    started_at: datetime,
    organization_id: UUID,
    user_id: UUID,
    mcp_tool_name: str | None = None,
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
        mcp_tool_name=mcp_tool_name or "test_tool",
        mcp_tool_id=uuid4(),
        mcp_server_configuration_name="test_config",
        mcp_server_configuration_id=uuid4(),
        arguments='{"arg": "value"}',
        result={"result": "success"},
        status=MCPToolCallStatus.SUCCESS,
        via_execute_tool=False,
        jsonrpc_payload={"jsonrpc": "2.0"},
        started_at=started_at,
        ended_at=started_at + timedelta(seconds=1),
        duration_ms=1000,
    )
    return crud.mcp_tool_call_logs.create(db_session, log_create)


def test_default_7_day_filter(
    test_client: TestClient,
    db_session: Session,
    dummy_member: User,
    dummy_access_token_member: str,
) -> None:
    """Test that logs older than 7 days are not returned by default."""
    current_time = datetime.now(UTC)
    org_id = dummy_member.organization_memberships[0].organization_id

    # Create log from 8 days ago (should be excluded)
    log_old = create_test_log(
        db_session=db_session,
        started_at=current_time - timedelta(days=8),
        organization_id=org_id,
        user_id=dummy_member.id,
    )

    # Create log from 6 days ago (should be included)
    log_recent = create_test_log(
        db_session=db_session,
        started_at=current_time - timedelta(days=6),
        organization_id=org_id,
        user_id=dummy_member.id,
    )

    # Create log from now (should be included)
    log_now = create_test_log(
        db_session=db_session,
        started_at=current_time,
        organization_id=org_id,
        user_id=dummy_member.id,
    )

    db_session.commit()

    # Request without specifying start_time
    response = test_client.get(
        f"{config.ROUTER_PREFIX_LOGS}/tool-calls",
        headers={"Authorization": f"Bearer {dummy_access_token_member}"},
    )
    assert response.status_code == 200
    data = response.json()

    # Should only see 2 logs (6 days ago and now)
    assert len(data["data"]) == 2
    log_ids = {UUID(log["id"]) for log in data["data"]}
    assert log_old.id not in log_ids
    assert log_recent.id in log_ids
    assert log_now.id in log_ids


def test_custom_start_time_within_7_days(
    test_client: TestClient,
    db_session: Session,
    dummy_member: User,
    dummy_access_token_member: str,
) -> None:
    """Test filtering with custom start_time within 7 days."""
    current_time = datetime.now(UTC)
    org_id = dummy_member.organization_memberships[0].organization_id

    # Create logs at different times
    _ = create_test_log(
        db_session=db_session,
        started_at=current_time - timedelta(days=6),
        organization_id=org_id,
        user_id=dummy_member.id,
    )

    _ = create_test_log(
        db_session=db_session,
        started_at=current_time - timedelta(days=3),
        organization_id=org_id,
        user_id=dummy_member.id,
    )

    log_1_day = create_test_log(
        db_session=db_session,
        started_at=current_time - timedelta(days=1),
        organization_id=org_id,
        user_id=dummy_member.id,
    )

    db_session.commit()

    # Request with start_time = 2 days ago
    start_time = current_time - timedelta(days=2)
    response = test_client.get(
        f"{config.ROUTER_PREFIX_LOGS}/tool-calls",
        params={"start_time": start_time.isoformat()},
        headers={"Authorization": f"Bearer {dummy_access_token_member}"},
    )
    assert response.status_code == 200
    data = response.json()

    # Should only see 1 log (1 day ago)
    assert len(data["data"]) == 1
    assert UUID(data["data"][0]["id"]) == log_1_day.id


def test_start_time_before_7_days_adjusted(
    test_client: TestClient,
    db_session: Session,
    dummy_member: User,
    dummy_access_token_member: str,
) -> None:
    """Test that start_time before 7 days is adjusted to 7 days ago."""
    current_time = datetime.now(UTC)
    org_id = dummy_member.organization_memberships[0].organization_id

    # Create log from 8 days ago
    _ = create_test_log(
        db_session=db_session,
        started_at=current_time - timedelta(days=8),
        organization_id=org_id,
        user_id=dummy_member.id,
    )

    # Create log from 6 days ago
    log_recent = create_test_log(
        db_session=db_session,
        started_at=current_time - timedelta(days=6),
        organization_id=org_id,
        user_id=dummy_member.id,
    )

    db_session.commit()

    # Request with start_time = 10 days ago (should be adjusted to 7 days ago)
    start_time = current_time - timedelta(days=10)
    response = test_client.get(
        f"{config.ROUTER_PREFIX_LOGS}/tool-calls",
        params={"start_time": start_time.isoformat()},
        headers={"Authorization": f"Bearer {dummy_access_token_member}"},
    )
    assert response.status_code == 200
    data = response.json()

    # Should only see 1 log (6 days ago), not the 8 days ago one
    assert len(data["data"]) == 1
    assert UUID(data["data"][0]["id"]) == log_recent.id


def test_end_time_filter(
    test_client: TestClient,
    db_session: Session,
    dummy_member: User,
    dummy_access_token_member: str,
) -> None:
    """Test filtering with end_time."""
    current_time = datetime.now(UTC)
    org_id = dummy_member.organization_memberships[0].organization_id

    # Create logs at different times
    log_6_days = create_test_log(
        db_session=db_session,
        started_at=current_time - timedelta(days=6),
        organization_id=org_id,
        user_id=dummy_member.id,
    )

    log_3_days = create_test_log(
        db_session=db_session,
        started_at=current_time - timedelta(days=3),
        organization_id=org_id,
        user_id=dummy_member.id,
    )

    log_1_day = create_test_log(
        db_session=db_session,
        started_at=current_time - timedelta(days=1),
        organization_id=org_id,
        user_id=dummy_member.id,
    )

    db_session.commit()

    # Request with end_time = 2 days ago
    end_time = current_time - timedelta(days=2)
    response = test_client.get(
        f"{config.ROUTER_PREFIX_LOGS}/tool-calls",
        params={"end_time": end_time.isoformat()},
        headers={"Authorization": f"Bearer {dummy_access_token_member}"},
    )
    assert response.status_code == 200
    data = response.json()

    # Should see 2 logs (6 days and 3 days ago)
    assert len(data["data"]) == 2
    log_ids = {UUID(log["id"]) for log in data["data"]}
    assert log_6_days.id in log_ids
    assert log_3_days.id in log_ids
    assert log_1_day.id not in log_ids


def test_start_and_end_time_filter(
    test_client: TestClient,
    db_session: Session,
    dummy_member: User,
    dummy_access_token_member: str,
) -> None:
    """Test filtering with both start_time and end_time."""
    current_time = datetime.now(UTC)
    org_id = dummy_member.organization_memberships[0].organization_id

    # Create logs at different times
    _ = create_test_log(
        db_session=db_session,
        started_at=current_time - timedelta(days=6),
        organization_id=org_id,
        user_id=dummy_member.id,
    )

    log_4_days = create_test_log(
        db_session=db_session,
        started_at=current_time - timedelta(days=4),
        organization_id=org_id,
        user_id=dummy_member.id,
    )

    _ = create_test_log(
        db_session=db_session,
        started_at=current_time - timedelta(days=2),
        organization_id=org_id,
        user_id=dummy_member.id,
    )

    db_session.commit()

    # Request with start_time = 5 days ago and end_time = 3 days ago
    start_time = current_time - timedelta(days=5)
    end_time = current_time - timedelta(days=3)
    response = test_client.get(
        f"{config.ROUTER_PREFIX_LOGS}/tool-calls",
        params={"start_time": start_time.isoformat(), "end_time": end_time.isoformat()},
        headers={"Authorization": f"Bearer {dummy_access_token_member}"},
    )
    assert response.status_code == 200
    data = response.json()

    # Should only see 1 log (4 days ago)
    assert len(data["data"]) == 1
    assert UUID(data["data"][0]["id"]) == log_4_days.id


def test_invalid_time_range_returns_empty(
    test_client: TestClient,
    db_session: Session,
    dummy_member: User,
    dummy_access_token_member: str,
) -> None:
    """Test that invalid time range (end_time < start_time) returns empty response."""
    current_time = datetime.now(UTC)
    org_id = dummy_member.organization_memberships[0].organization_id

    # Create a log
    create_test_log(
        db_session=db_session,
        started_at=current_time - timedelta(days=3),
        organization_id=org_id,
        user_id=dummy_member.id,
    )

    db_session.commit()

    # Request with end_time < start_time
    start_time = current_time - timedelta(days=2)
    end_time = current_time - timedelta(days=4)
    response = test_client.get(
        f"{config.ROUTER_PREFIX_LOGS}/tool-calls",
        params={"start_time": start_time.isoformat(), "end_time": end_time.isoformat()},
        headers={"Authorization": f"Bearer {dummy_access_token_member}"},
    )
    assert response.status_code == 200
    data = response.json()

    # Should return empty response
    assert data["data"] == []
    assert data["next_cursor"] is None


def test_time_filter_with_pagination(
    test_client: TestClient,
    db_session: Session,
    dummy_member: User,
    dummy_access_token_member: str,
) -> None:
    """Test that time filters work correctly with pagination."""
    current_time = datetime.now(UTC)
    org_id = dummy_member.organization_memberships[0].organization_id

    # Create 5 logs within time range
    logs_in_range = []
    for i in range(5):
        log = create_test_log(
            db_session=db_session,
            started_at=current_time - timedelta(days=3) + timedelta(hours=i),
            organization_id=org_id,
            user_id=dummy_member.id,
        )
        logs_in_range.append(log)

    # Create 2 logs outside time range
    log_old = create_test_log(
        db_session=db_session,
        started_at=current_time - timedelta(days=6),
        organization_id=org_id,
        user_id=dummy_member.id,
    )

    log_future = create_test_log(
        db_session=db_session,
        started_at=current_time,
        organization_id=org_id,
        user_id=dummy_member.id,
    )

    db_session.commit()

    # Request first page with time filter
    start_time = current_time - timedelta(days=4)
    end_time = current_time - timedelta(days=2)
    response = test_client.get(
        f"{config.ROUTER_PREFIX_LOGS}/tool-calls",
        params={"limit": 2, "start_time": start_time.isoformat(), "end_time": end_time.isoformat()},
        headers={"Authorization": f"Bearer {dummy_access_token_member}"},
    )
    assert response.status_code == 200
    data = response.json()

    # Should see 2 logs from within the time range
    assert len(data["data"]) == 2
    assert data["next_cursor"] is not None

    # Collect all logs across pages
    all_logs = data["data"].copy()
    cursor = data["next_cursor"]

    while cursor is not None:
        response = test_client.get(
            f"{config.ROUTER_PREFIX_LOGS}/tool-calls",
            params={
                "limit": 2,
                "cursor": cursor,
                "start_time": start_time.isoformat(),
                "end_time": end_time.isoformat(),
            },
            headers={"Authorization": f"Bearer {dummy_access_token_member}"},
        )
        assert response.status_code == 200
        data = response.json()
        all_logs.extend(data["data"])
        cursor = data["next_cursor"]

    # Should have 5 total logs (all within time range)
    assert len(all_logs) == 5
    log_ids = {UUID(log["id"]) for log in all_logs}
    expected_ids = {log.id for log in logs_in_range}
    assert log_ids == expected_ids
    assert log_old.id not in log_ids
    assert log_future.id not in log_ids


def test_time_filter_with_tool_name_filter(
    test_client: TestClient,
    db_session: Session,
    dummy_member: User,
    dummy_access_token_member: str,
) -> None:
    """Test that time filters work correctly with tool name filter."""
    current_time = datetime.now(UTC)
    org_id = dummy_member.organization_memberships[0].organization_id

    # Create logs with different tool names and times
    _ = create_test_log(
        db_session=db_session,
        mcp_tool_name="TOOL_A",
        started_at=current_time - timedelta(days=6),
        organization_id=org_id,
        user_id=dummy_member.id,
    )

    log_a_recent = create_test_log(
        db_session=db_session,
        mcp_tool_name="TOOL_A",
        started_at=current_time - timedelta(days=2),
        organization_id=org_id,
        user_id=dummy_member.id,
    )

    _ = create_test_log(
        db_session=db_session,
        mcp_tool_name="TOOL_B",
        started_at=current_time - timedelta(days=2),
        organization_id=org_id,
        user_id=dummy_member.id,
    )

    db_session.commit()

    # Request with time filter and tool name filter
    start_time = current_time - timedelta(days=3)
    response = test_client.get(
        f"{config.ROUTER_PREFIX_LOGS}/tool-calls",
        params={"start_time": start_time.isoformat(), "mcp_tool_name": "TOOL_A"},
        headers={"Authorization": f"Bearer {dummy_access_token_member}"},
    )
    assert response.status_code == 200
    data = response.json()

    # Should only see 1 log (TOOL_A from 2 days ago)
    assert len(data["data"]) == 1
    assert UUID(data["data"][0]["id"]) == log_a_recent.id
    assert data["data"][0]["mcp_tool_name"] == "TOOL_A"


def test_admin_time_filter(
    test_client: TestClient,
    db_session: Session,
    dummy_admin: User,
    dummy_member_2: User,
    dummy_access_token_admin: str,
) -> None:
    """Test that time filters work correctly for admin seeing all org logs."""
    current_time = datetime.now(UTC)
    org_id = dummy_admin.organization_memberships[0].organization_id

    # Create logs for different users at different times
    log_admin_old = create_test_log(
        db_session=db_session,
        started_at=current_time - timedelta(days=6),
        organization_id=org_id,
        user_id=dummy_admin.id,
    )

    log_admin_recent = create_test_log(
        db_session=db_session,
        started_at=current_time - timedelta(days=2),
        organization_id=org_id,
        user_id=dummy_admin.id,
    )

    log_member_recent = create_test_log(
        db_session=db_session,
        started_at=current_time - timedelta(days=2),
        organization_id=org_id,
        user_id=dummy_member_2.id,
    )

    db_session.commit()

    # Admin requests with time filter
    start_time = current_time - timedelta(days=3)
    response = test_client.get(
        f"{config.ROUTER_PREFIX_LOGS}/tool-calls",
        params={"start_time": start_time.isoformat()},
        headers={"Authorization": f"Bearer {dummy_access_token_admin}"},
    )
    assert response.status_code == 200
    data = response.json()

    # Should see 2 logs from both users (within time range)
    assert len(data["data"]) == 2
    log_ids = {UUID(log["id"]) for log in data["data"]}
    assert log_admin_old.id not in log_ids
    assert log_admin_recent.id in log_ids
    assert log_member_recent.id in log_ids


def test_exact_boundary_times(
    test_client: TestClient,
    db_session: Session,
    dummy_member: User,
    dummy_access_token_member: str,
) -> None:
    """Test filtering with exact boundary timestamps."""
    current_time = datetime.now(UTC)
    org_id = dummy_member.organization_memberships[0].organization_id

    # Create logs at exact times
    time_1 = current_time - timedelta(days=5)
    time_2 = current_time - timedelta(days=4)
    time_3 = current_time - timedelta(days=3)

    log_1 = create_test_log(
        db_session=db_session,
        started_at=time_1,
        organization_id=org_id,
        user_id=dummy_member.id,
    )

    log_2 = create_test_log(
        db_session=db_session,
        started_at=time_2,
        organization_id=org_id,
        user_id=dummy_member.id,
    )

    log_3 = create_test_log(
        db_session=db_session,
        started_at=time_3,
        organization_id=org_id,
        user_id=dummy_member.id,
    )

    db_session.commit()

    # Request with start_time = time_2 (should include time_2 and time_3)
    response = test_client.get(
        f"{config.ROUTER_PREFIX_LOGS}/tool-calls",
        params={"start_time": time_2.isoformat()},
        headers={"Authorization": f"Bearer {dummy_access_token_member}"},
    )
    assert response.status_code == 200
    data = response.json()

    assert len(data["data"]) == 2
    log_ids = {UUID(log["id"]) for log in data["data"]}
    assert log_1.id not in log_ids
    assert log_2.id in log_ids
    assert log_3.id in log_ids

    # Request with end_time = time_2 (should include time_1 and time_2)
    response = test_client.get(
        f"{config.ROUTER_PREFIX_LOGS}/tool-calls",
        params={"end_time": time_2.isoformat()},
        headers={"Authorization": f"Bearer {dummy_access_token_member}"},
    )
    assert response.status_code == 200
    data = response.json()

    assert len(data["data"]) == 2
    log_ids = {UUID(log["id"]) for log in data["data"]}
    assert log_1.id in log_ids
    assert log_2.id in log_ids
    assert log_3.id not in log_ids
