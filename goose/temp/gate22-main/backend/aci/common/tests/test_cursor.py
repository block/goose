import base64
import binascii
import json
from datetime import datetime
from uuid import uuid4

import pytest

from aci.common.schemas.mcp_tool_call_log import MCPToolCallLogCursor


def test_encode_decode_roundtrip() -> None:
    ts = datetime(2024, 1, 2, 3, 4, 5, 123456)
    id = uuid4()

    cur = MCPToolCallLogCursor.encode(ts, id)
    print(cur)
    decoded = MCPToolCallLogCursor.decode(cur)

    assert decoded.started_at == ts
    assert decoded.id == id


def test_encode_is_deterministic() -> None:
    ts = datetime(2024, 9, 10, 11, 12, 13)
    id = uuid4()

    c1 = MCPToolCallLogCursor.encode(ts, id)
    c2 = MCPToolCallLogCursor.encode(ts, id)

    assert c1 == c2


def test_encode_outputs_urlsafe_base64_chars() -> None:
    ts = datetime(2023, 5, 6, 7, 8, 9)
    id = uuid4()
    cur = MCPToolCallLogCursor.encode(ts, id)

    # urlsafe base64 uses A-Z a-z 0-9 - _ and optionally '=' padding
    allowed = set("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_=")
    assert set(cur) <= allowed


def test_decode_rejects_non_base64() -> None:
    with pytest.raises(binascii.Error):
        MCPToolCallLogCursor.decode("not-base64!!")  # invalid alphabet


def test_decode_rejects_missing_id() -> None:
    payload = {"started_at": "2024-01-02T03:04:05"}  # no id
    cur = base64.urlsafe_b64encode(json.dumps(payload).encode()).decode()

    with pytest.raises(KeyError):
        MCPToolCallLogCursor.decode(cur)


def test_decode_rejects_invalid_timestamp() -> None:
    payload = {"started_at": "not-a-timestamp", "id": str(uuid4())}
    cur = base64.urlsafe_b64encode(json.dumps(payload).encode()).decode()

    with pytest.raises(ValueError):
        MCPToolCallLogCursor.decode(cur)


def test_decode_rejects_invalid_uuid() -> None:
    payload = {"started_at": "2024-01-02T03:04:05", "id": "invalid-uuid"}
    cur = base64.urlsafe_b64encode(json.dumps(payload).encode()).decode()

    with pytest.raises(ValueError):
        MCPToolCallLogCursor.decode(cur)
