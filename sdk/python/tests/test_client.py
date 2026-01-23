"""Tests for goosed SDK Python client."""


from goosed_sdk import GoosedClient


class TestStatusAPIs:
    """Tests for status APIs."""

    def test_status_returns_ok(self, client: GoosedClient):
        """Test that status() returns 'ok'."""
        result = client.status()
        assert result == "ok"

    def test_system_info_returns_version(self, client: GoosedClient):
        """Test that system_info() returns version information."""
        info = client.system_info()
        assert info.app_version
        assert info.provider
        assert info.model


class TestSessionManagement:
    """Tests for session management APIs."""

    def test_create_and_delete_session(self, client: GoosedClient, working_dir: str):
        """Test creating and deleting a session."""
        session = client.start_session(working_dir)
        assert session.id
        assert session.working_dir == working_dir

        client.delete_session(session.id)

    def test_list_sessions(self, client: GoosedClient):
        """Test listing sessions."""
        sessions = client.list_sessions()
        assert isinstance(sessions, list)

    def test_resume_session(self, client: GoosedClient, working_dir: str):
        """Test resuming a session."""
        session = client.start_session(working_dir)

        try:
            resumed, extension_results = client.resume_session(session.id)
            assert resumed.id == session.id
            assert isinstance(extension_results, list)
        finally:
            client.delete_session(session.id)

    def test_update_session_name(self, client: GoosedClient, working_dir: str):
        """Test updating a session's name."""
        session = client.start_session(working_dir)

        try:
            client.update_session_name(session.id, "Python Test Session")
            updated = client.get_session(session.id)
            assert updated.name == "Python Test Session"
        finally:
            client.delete_session(session.id)


class TestAgentAPIs:
    """Tests for agent APIs."""

    def test_get_tools(self, client: GoosedClient, working_dir: str):
        """Test getting tools."""
        session = client.start_session(working_dir)

        try:
            client.resume_session(session.id)
            tools = client.get_tools(session.id)
            assert isinstance(tools, list)
            assert len(tools) > 0
            assert tools[0].name
        finally:
            client.stop_session(session.id)
            client.delete_session(session.id)

    def test_call_tool(self, client: GoosedClient, working_dir: str):
        """Test calling a tool."""
        session = client.start_session(working_dir)

        try:
            client.resume_session(session.id)
            result = client.call_tool(
                session.id, "todo__todo_write", {"content": "Python SDK Test TODO"}
            )
            assert result.is_error is False
            assert len(result.content) > 0
        finally:
            client.stop_session(session.id)
            client.delete_session(session.id)

    def test_restart_session(self, client: GoosedClient, working_dir: str):
        """Test restarting a session."""
        session = client.start_session(working_dir)

        try:
            client.resume_session(session.id)
            results = client.restart_session(session.id)
            assert isinstance(results, list)
        finally:
            client.stop_session(session.id)
            client.delete_session(session.id)


class TestChatAPIs:
    """Tests for chat APIs."""

    def test_send_message_stream(self, client: GoosedClient, working_dir: str):
        """Test sending a message with streaming."""
        session = client.start_session(working_dir)

        try:
            client.resume_session(session.id)

            events = []
            for event in client.send_message(session.id, "Say hello"):
                events.append(event)

            assert len(events) > 0
            event_types = [e.type for e in events]
            assert "Finish" in event_types
        finally:
            client.stop_session(session.id)
            client.delete_session(session.id)

    def test_chat(self, client: GoosedClient, working_dir: str):
        """Test chat (non-streaming)."""
        session = client.start_session(working_dir)

        try:
            client.resume_session(session.id)
            response = client.chat(session.id, "Say hello in one word")
            assert isinstance(response, str)
            assert len(response) > 0
        finally:
            client.stop_session(session.id)
            client.delete_session(session.id)


class TestExportAPIs:
    """Tests for export APIs."""

    def test_export_session(self, client: GoosedClient, working_dir: str):
        """Test exporting a session."""
        session = client.start_session(working_dir)

        try:
            exported = client.export_session(session.id)
            assert isinstance(exported, str)
            assert session.id in exported
        finally:
            client.delete_session(session.id)
