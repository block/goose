from datetime import UTC, datetime, timedelta
from uuid import UUID, uuid4

from fastapi.testclient import TestClient
from sqlalchemy.orm import Session

from aci.common.db import crud
from aci.common.db.sql_models import MCPToolCallLog, User
from aci.common.enums import MCPToolCallStatus
from aci.common.schemas.mcp_tool_call_log import (
    MCPToolCallLogCreate,
    MCPToolCallLogCursor,
    MCPToolCallLogResponse,
)
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


def test_empty(
    test_client: TestClient,
    dummy_access_token_admin: str,
) -> None:
    """Test getting logs when there are none."""
    response = test_client.get(
        f"{config.ROUTER_PREFIX_LOGS}/tool-calls",
        headers={"Authorization": f"Bearer {dummy_access_token_admin}"},
    )
    assert response.status_code == 200
    data = response.json()
    assert data["data"] == []
    assert data["next_cursor"] is None


def test_basic(
    test_client: TestClient,
    db_session: Session,
    dummy_member: User,
    dummy_access_token_member: str,
) -> None:
    """Test basic pagination with multiple logs."""
    # Create 5 logs with different timestamps (most recent first in the list)
    base_time = datetime.now(UTC)

    logs: list[MCPToolCallLog] = []
    for i in range(5):
        log = create_test_log(
            db_session=db_session,
            started_at=base_time - timedelta(seconds=i),
            organization_id=dummy_member.organization_memberships[0].organization_id,
            user_id=dummy_member.id,
        )
        logs.append(log)

    db_session.commit()

    # Request first page with limit=2
    response = test_client.get(
        f"{config.ROUTER_PREFIX_LOGS}/tool-calls?limit=2",
        headers={"Authorization": f"Bearer {dummy_access_token_member}"},
    )
    assert response.status_code == 200
    data = response.json()

    assert len(data["data"]) == 2
    assert data["data"][0]["id"] == str(logs[0].id)  # Most recent
    assert data["data"][1]["id"] == str(logs[1].id)
    assert data["next_cursor"] is not None

    # Request second page using cursor
    response = test_client.get(
        f"{config.ROUTER_PREFIX_LOGS}/tool-calls?limit=2&cursor={data['next_cursor']}",
        headers={"Authorization": f"Bearer {dummy_access_token_member}"},
    )
    assert response.status_code == 200
    data = response.json()

    assert len(data["data"]) == 2
    assert data["data"][0]["id"] == str(logs[2].id)
    assert data["data"][1]["id"] == str(logs[3].id)
    assert data["next_cursor"] is not None

    # Request third page - should have 1 item and no next cursor
    response = test_client.get(
        f"{config.ROUTER_PREFIX_LOGS}/tool-calls?limit=2&cursor={data['next_cursor']}",
        headers={"Authorization": f"Bearer {dummy_access_token_member}"},
    )
    assert response.status_code == 200
    data = response.json()

    assert len(data["data"]) == 1
    assert data["data"][0]["id"] == str(logs[4].id)
    assert data["next_cursor"] is None


def test_same_timestamp(
    test_client: TestClient,
    db_session: Session,
    dummy_member: User,
    dummy_access_token_member: str,
) -> None:
    """Test pagination with logs that have the same timestamp.

    When logs have the same timestamp, they should be ordered by id DESC
    to ensure stable, deterministic pagination.
    """
    # Create 3 logs with the same timestamp
    same_time = datetime.now(UTC)

    logs: list[MCPToolCallLog] = []
    for _ in range(3):
        log = create_test_log(
            db_session=db_session,
            started_at=same_time,
            organization_id=dummy_member.organization_memberships[0].organization_id,
            user_id=dummy_member.id,
        )
        logs.append(log)

    db_session.commit()

    # Sort logs by ID descending (expected order)
    logs_sorted = sorted(logs, key=lambda x: x.id, reverse=True)

    # Request first page with limit=2
    response = test_client.get(
        f"{config.ROUTER_PREFIX_LOGS}/tool-calls?limit=2",
        headers={"Authorization": f"Bearer {dummy_access_token_member}"},
    )
    assert response.status_code == 200
    data = response.json()

    assert len(data["data"]) == 2
    # Verify logs are ordered by id DESC
    assert data["data"][0]["id"] == str(logs_sorted[0].id)
    assert data["data"][1]["id"] == str(logs_sorted[1].id)
    first_page_ids = {data["data"][0]["id"], data["data"][1]["id"]}
    assert data["next_cursor"] is not None

    # Request second page - should have the third log
    response = test_client.get(
        f"{config.ROUTER_PREFIX_LOGS}/tool-calls?limit=2&cursor={data['next_cursor']}",
        headers={"Authorization": f"Bearer {dummy_access_token_member}"},
    )
    assert response.status_code == 200
    data = response.json()

    assert len(data["data"]) == 1
    # Verify it's the log with the smallest ID
    assert data["data"][0]["id"] == str(logs_sorted[2].id)
    # Verify no duplicate IDs
    assert data["data"][0]["id"] not in first_page_ids
    assert data["next_cursor"] is None


def test_filter_by_tool_name(
    test_client: TestClient,
    db_session: Session,
    dummy_member: User,
    dummy_access_token_member: str,
) -> None:
    """Test filtering logs by tool name."""
    same_time = datetime.now(UTC)

    # Create logs with different tool names
    for i in range(3):
        create_test_log(
            db_session=db_session,
            mcp_tool_name=f"TOOL_{i}",
            started_at=same_time,
            organization_id=dummy_member.organization_memberships[0].organization_id,
            user_id=dummy_member.id,
        )

    db_session.commit()

    # Filter by TOOL_0
    response = test_client.get(
        f"{config.ROUTER_PREFIX_LOGS}/tool-calls?mcp_tool_name=TOOL_0",
        headers={"Authorization": f"Bearer {dummy_access_token_member}"},
    )
    assert response.status_code == 200
    data = response.json()

    assert len(data["data"]) == 1
    assert data["data"][0]["mcp_tool_name"] == "TOOL_0"
    assert data["next_cursor"] is None


def test_filter_with_pagination(
    test_client: TestClient,
    db_session: Session,
    dummy_member: User,
    dummy_access_token_member: str,
) -> None:
    """Test filtering with pagination."""
    base_time = datetime.now(UTC)

    # Create 5 logs with TOOL_A
    for i in range(5):
        create_test_log(
            db_session=db_session,
            mcp_tool_name="TOOL_A",
            started_at=base_time - timedelta(seconds=i),
            organization_id=dummy_member.organization_memberships[0].organization_id,
            user_id=dummy_member.id,
        )

    # Create 2 logs with TOOL_B
    for i in range(2):
        create_test_log(
            db_session=db_session,
            mcp_tool_name="TOOL_B",
            started_at=base_time - timedelta(seconds=i),
            organization_id=dummy_member.organization_memberships[0].organization_id,
            user_id=dummy_member.id,
        )

    db_session.commit()

    # Filter by TOOL_A with pagination
    response = test_client.get(
        f"{config.ROUTER_PREFIX_LOGS}/tool-calls?mcp_tool_name=TOOL_A&limit=2",
        headers={"Authorization": f"Bearer {dummy_access_token_member}"},
    )
    assert response.status_code == 200
    data = response.json()

    assert len(data["data"]) == 2
    assert all(log["mcp_tool_name"] == "TOOL_A" for log in data["data"])
    assert data["next_cursor"] is not None

    # Get remaining page
    response = test_client.get(
        f"{config.ROUTER_PREFIX_LOGS}/tool-calls?mcp_tool_name=TOOL_A&limit=3&cursor={data['next_cursor']}",
        headers={"Authorization": f"Bearer {dummy_access_token_member}"},
    )
    assert response.status_code == 200
    data = response.json()

    assert len(data["data"]) == 3
    assert all(log["mcp_tool_name"] == "TOOL_A" for log in data["data"])
    assert data["next_cursor"] is None


def test_invalid_cursor(
    test_client: TestClient,
    dummy_member: User,
    dummy_access_token_member: str,
) -> None:
    """Test that invalid cursor returns 400."""
    response = test_client.get(
        f"{config.ROUTER_PREFIX_LOGS}/tool-calls?cursor=invalid_cursor",
        headers={"Authorization": f"Bearer {dummy_access_token_member}"},
    )
    assert response.status_code == 400
    assert "Invalid cursor" in response.json()["detail"]


def test_response_structure(
    test_client: TestClient,
    db_session: Session,
    dummy_member: User,
    dummy_access_token_member: str,
) -> None:
    """Test the response structure contains all expected fields."""
    log = create_test_log(
        db_session=db_session,
        started_at=datetime.now(UTC),
        mcp_tool_name="test_tool",
        organization_id=dummy_member.organization_memberships[0].organization_id,
        user_id=dummy_member.id,
    )
    db_session.commit()

    response = test_client.get(
        f"{config.ROUTER_PREFIX_LOGS}/tool-calls",
        headers={"Authorization": f"Bearer {dummy_access_token_member}"},
    )
    assert response.status_code == 200
    data = response.json()

    assert len(data["data"]) == 1
    log_data = data["data"][0]

    # Verify all expected fields are present
    input_log = MCPToolCallLogResponse.model_validate(log, from_attributes=True)
    returned_log = MCPToolCallLogResponse.model_validate(log_data)
    assert returned_log == input_log


def test_cursor_encoding_decoding(
    test_client: TestClient,
    db_session: Session,
    dummy_member: User,
    dummy_access_token_member: str,
) -> None:
    """Test that cursors are properly encoded and decoded."""
    base_time = datetime.now(UTC)

    # Create 2 logs
    log_new = create_test_log(
        db_session=db_session,
        started_at=base_time,
        organization_id=dummy_member.organization_memberships[0].organization_id,
        user_id=dummy_member.id,
    )
    log_old = create_test_log(
        db_session=db_session,
        started_at=base_time - timedelta(seconds=1),
        organization_id=dummy_member.organization_memberships[0].organization_id,
        user_id=dummy_member.id,
    )
    db_session.commit()

    # Get first page
    response = test_client.get(
        f"{config.ROUTER_PREFIX_LOGS}/tool-calls?limit=1",
        headers={"Authorization": f"Bearer {dummy_access_token_member}"},
    )
    assert response.status_code == 200
    data = response.json()

    # Decode and verify cursor
    cursor = data["next_cursor"]
    decoded_cursor = MCPToolCallLogCursor.decode(cursor)
    assert decoded_cursor.started_at == log_new.started_at
    assert decoded_cursor.id == log_new.id

    # Verify the cursor can be used to fetch next page
    response = test_client.get(
        f"{config.ROUTER_PREFIX_LOGS}/tool-calls?limit=1&cursor={cursor}",
        headers={"Authorization": f"Bearer {dummy_access_token_member}"},
    )
    assert response.status_code == 200
    data = response.json()
    assert len(data["data"]) == 1
    assert data["data"][0]["id"] == str(log_old.id), "The old log should be returned in second page"
    assert data["next_cursor"] is None
