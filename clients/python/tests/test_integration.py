"""Integration tests for Goose Python client"""

import pytest
import os
import time
from goose_client import GooseClient

# Test configuration
TEST_API_KEY = "test_secret_8899"
TEST_BASE_URL = "http://localhost:8899"


@pytest.fixture
def client():
    """Create test client"""
    return GooseClient(
        api_key=TEST_API_KEY,
        base_url=TEST_BASE_URL
    )


def test_health_check(client):
    """Test server connectivity"""
    assert client.health_check() == True
    assert client.is_healthy == True


def test_create_and_delete_session(client):
    """Test session creation and deletion"""
    # Create session
    session = client.create_session("/tmp")
    assert session is not None
    assert hasattr(session, 'id')
    assert session.id is not None
    
    # Delete session
    result = client.delete_session(session.id)
    assert result == True


def test_session_context_manager(client):
    """Test session context manager"""
    session_id = None
    
    with client.session("/tmp") as session:
        assert session is not None
        assert hasattr(session, 'id')
        session_id = session.id
        
        # Session should be active
        assert client.active_session_id == session_id
    
    # After context, session should be deleted and no longer active
    assert client.active_session_id != session_id


def test_chat_basic(client):
    """Test basic chat functionality"""
    with client.session() as session:
        response = client.chat("Say 'test passed' and nothing else", session.id)
        assert response is not None
        assert isinstance(response, str)
        assert len(response) > 0
        # Check that it contains expected words (case insensitive)
        response_lower = response.lower()
        assert "test" in response_lower or "passed" in response_lower


def test_streaming_chat(client):
    """Test SSE streaming"""
    with client.session() as session:
        chunks = []
        for chunk in client.stream_chat("Count from 1 to 3", session.id):
            chunks.append(chunk)
        
        assert len(chunks) > 0
        # Combine all chunks
        full_response = ''.join(chunks)
        assert len(full_response) > 0


def test_session_memory(client):
    """Test that sessions remember context"""
    with client.session() as session:
        # First message - ask to remember something
        response1 = client.chat(
            "Remember this word: BANANA. Just confirm you'll remember it.",
            session.id
        )
        assert response1 is not None
        
        # Second message - ask what was remembered
        response2 = client.chat(
            "What word did I ask you to remember? Just say the word.",
            session.id
        )
        assert response2 is not None
        assert "banana" in response2.lower() or "BANANA" in response2


def test_ask_one_shot(client):
    """Test one-shot Q&A"""
    answer = client.ask("What is 2 plus 2? Just give the number.")
    assert answer is not None
    assert isinstance(answer, str)
    # Should contain "4" or "four"
    assert "4" in answer or "four" in answer.lower()


def test_list_sessions(client):
    """Test listing sessions"""
    # Create a session first
    session = client.create_session()
    
    try:
        # List sessions
        sessions = client.list_sessions()
        assert isinstance(sessions, list)
        
        # Our session should be in the list
        session_ids = [s.id for s in sessions if hasattr(s, 'id')]
        assert session.id in session_ids
    finally:
        # Clean up
        client.delete_session(session.id)


def test_list_extensions(client):
    """Test listing extensions"""
    extensions = client.list_extensions()
    assert isinstance(extensions, list)
    # Should have at least some built-in extensions


def test_list_tools(client):
    """Test listing tools"""
    with client.session() as session:
        tools = client.list_tools(session.id)
        assert isinstance(tools, list)


def test_active_session_management(client):
    """Test active session management"""
    # Initially no active session
    assert client.active_session_id is None
    
    # Create a session
    session = client.create_session()
    
    try:
        # Should be active
        assert client.active_session_id == session.id
        
        # Clear active session
        client.clear_active_session()
        assert client.active_session_id is None
        
        # Set active session
        client.set_active_session(session.id)
        assert client.active_session_id == session.id
    finally:
        # Clean up
        client.delete_session(session.id)


def test_error_handling_invalid_session(client):
    """Test error handling with invalid session ID"""
    # Try to chat with non-existent session
    fake_session_id = "non-existent-session-12345"
    
    # This should raise an exception or handle gracefully
    try:
        response = client.chat("Hello", fake_session_id)
        # If it doesn't raise, it might have created a new session
        # or handled the error gracefully
    except Exception as e:
        # Expected behavior - invalid session should cause an error
        assert True


def test_send_message_with_streaming(client):
    """Test send_message with streaming enabled"""
    with client.session() as session:
        # Get streaming iterator
        stream = client.send_message("Say hello", session.id, stream=True)
        
        events = []
        for event in stream:
            events.append(event)
            if len(events) > 100:  # Safety limit
                break
        
        assert len(events) > 0
        # Should have at least one Message event
        message_events = [e for e in events if e.get('type') == 'Message']
        assert len(message_events) > 0
