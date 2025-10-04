"""Simple, ready-to-use Goose client"""

import logging
from contextlib import contextmanager
from typing import Any, Dict, Iterator, List, Optional, Union

import httpx

from ._generated._generated.models import (
    Session,
    StartAgentRequest,
)
from .core import BaseClient
from .streaming import StreamingMixin

logger = logging.getLogger(__name__)


class GooseClient(BaseClient, StreamingMixin):
    """Simple Goose client for 80% of use cases"""

    def __init__(self, api_key: str, base_url: str = "http://localhost:3000"):
        """Initialize the Goose client.

        Args:
            api_key: The API key for authentication
            base_url: The base URL for the Goose server
        """
        super().__init__(api_key, base_url)
        self._active_session: Optional[str] = None

    # --- Session Management ---

    def create_session(self, working_dir: str = "/tmp") -> Session:
        """Create a new session.

        Args:
            working_dir: Working directory for the session

        Returns:
            The created Session object
        """
        request = StartAgentRequest(working_dir=working_dir)
        session = self._agent_api.start_agent(start_agent_request=request)
        self._active_session = session.id
        logger.info(f"Created session {session.id} with working_dir={working_dir}")
        return session

    @contextmanager
    def session(self, working_dir: str = "/tmp"):
        """Context manager for session lifecycle.

        Args:
            working_dir: Working directory for the session

        Yields:
            The created Session object
        """
        session = self.create_session(working_dir)
        old_active = self._active_session
        self._active_session = session.id
        logger.debug(f"Entering session context {session.id}")

        try:
            yield session
        finally:
            self._active_session = old_active
            try:
                self._sessions_api.delete_session(session.id)
                logger.debug(f"Deleted session {session.id}")
            except Exception as e:
                logger.warning(f"Failed to delete session {session.id}: {e}")

    def list_sessions(self) -> List[Session]:
        """List all sessions.

        Returns:
            List of Session objects
        """
        response = self._sessions_api.list_sessions()
        sessions = response.sessions if hasattr(response, "sessions") else []
        logger.debug(f"Listed {len(sessions)} sessions")
        return sessions

    def get_session(self, session_id: str) -> Session:
        """Get a specific session.

        Args:
            session_id: The session ID to retrieve

        Returns:
            The Session object
        """
        session = self._sessions_api.get_session(session_id)
        logger.debug(f"Retrieved session {session_id}")
        return session

    def delete_session(self, session_id: str) -> bool:
        """Delete a session.

        Args:
            session_id: The session ID to delete

        Returns:
            True if successful, False otherwise
        """
        try:
            self._sessions_api.delete_session(session_id)
            logger.info(f"Deleted session {session_id}")
            if self._active_session == session_id:
                self._active_session = None
            return True
        except Exception as e:
            logger.error(f"Failed to delete session {session_id}: {e}")
            return False

    # --- Messaging ---

    def send_message(
        self, text: str, session_id: Optional[str] = None, stream: bool = False
    ) -> Union[str, Iterator[Dict[str, Any]]]:
        """Send a message to a session.

        Args:
            text: The message text to send
            session_id: Optional session ID (uses active session if not provided)
            stream: Whether to stream the response

        Returns:
            Either the complete response text or an iterator of stream events
        """
        # Use provided session or active session or create temporary
        if not session_id:
            if not self._active_session:
                temp_session = self.create_session()
                session_id = temp_session.id
                logger.debug(f"Created temporary session {session_id}")
            else:
                session_id = self._active_session

        # Create message with proper structure
        message_dict = {"role": "user", "content": [{"type": "text", "text": text}]}

        logger.debug(f"Sending message to session {session_id}: {text[:50]}...")

        # Stream or collect response
        if stream:
            return self.stream_reply(session_id, [message_dict])
        else:
            # Collect full response
            response_text = ""

            for event in self.stream_reply(session_id, [message_dict]):
                if event.get("type") == "Message":
                    msg = event.get("message", {})
                    for content in msg.get("content", []):
                        if content.get("type") == "text":
                            response_text += content.get("text", "")
                elif event.get("type") == "Error":
                    error_msg = event.get("error", "Unknown error")
                    logger.error(f"Error in response: {error_msg}")
                    raise Exception(f"Error: {error_msg}")

            logger.debug(f"Received response: {response_text[:100]}...")
            return response_text

    def chat(self, text: str, session_id: Optional[str] = None) -> str:
        """Simple chat interface - always returns text.

        Args:
            text: The message text to send
            session_id: Optional session ID (uses active session if not provided)

        Returns:
            The response text
        """
        return self.send_message(text, session_id, stream=False)

    def stream_chat(self, text: str, session_id: Optional[str] = None) -> Iterator[str]:
        """Stream chat responses as text chunks.

        Args:
            text: The message text to send
            session_id: Optional session ID (uses active session if not provided)

        Yields:
            Text chunks from the response
        """
        for event in self.send_message(text, session_id, stream=True):
            if event.get("type") == "Message":
                msg = event.get("message", {})
                for content in msg.get("content", []):
                    if content.get("type") == "text":
                        text_chunk = content.get("text", "")
                        if text_chunk:
                            yield text_chunk
            elif event.get("type") == "Error":
                error_msg = event.get("error", "Unknown error")
                logger.error(f"Stream error: {error_msg}")
                break

    # --- Tools & Extensions ---

    def list_tools(self, session_id: Optional[str] = None) -> List[Dict[str, Any]]:
        """List available tools.

        Args:
            session_id: Optional session ID (uses active session if not provided)

        Returns:
            List of tool information dictionaries
        """
        if not session_id:
            session_id = self._active_session or self.create_session().id

        try:
            tools = self._agent_api.get_tools(session_id=session_id)
            logger.debug(f"Listed {len(tools)} tools for session {session_id}")
            return tools
        except Exception as e:
            logger.error(f"Failed to list tools: {e}")
            return []

    def list_extensions(self) -> List[Dict[str, Any]]:
        """List available extensions.

        Returns:
            List of extension information dictionaries
        """
        try:
            response = self._config_api.get_extensions()
            extensions = response.extensions if hasattr(response, "extensions") else []
            logger.debug(f"Listed {len(extensions)} extensions")
            return extensions
        except Exception as e:
            logger.error(f"Failed to list extensions: {e}")
            return []

    # --- Convenience Methods ---

    def ask(self, question: str) -> str:
        """One-shot question answering.

        Args:
            question: The question to ask

        Returns:
            The answer text
        """
        with self.session() as session:
            return self.chat(question, session.id)

    @property
    def is_healthy(self) -> bool:
        """Check server health.

        Returns:
            True if server is healthy, False otherwise
        """
        return self.health_check()

    @property
    def active_session_id(self) -> Optional[str]:
        """Get the active session ID.

        Returns:
            The active session ID or None
        """
        return self._active_session

    def set_active_session(self, session_id: str) -> None:
        """Set the active session.

        Args:
            session_id: The session ID to make active
        """
        self._active_session = session_id
        logger.debug(f"Set active session to {session_id}")

    def clear_active_session(self) -> None:
        """Clear the active session."""
        self._active_session = None
        logger.debug("Cleared active session")

    # --- Tool Confirmation ---

    def confirm_permission(
        self,
        confirmation_id: str,
        action: str,
        session_id: Optional[str] = None,
        principal_type: str = "Tool",
    ) -> bool:
        """Confirm or deny a tool permission request.

        Args:
            confirmation_id: The ID of the tool confirmation request
            action: The action to take ("allow_once", "always_allow", or "deny")
            session_id: Optional session ID (uses active session if not provided)
            principal_type: Type of principal (default: "Tool")

        Returns:
            True if confirmation was successful, False otherwise
        """
        if not session_id:
            session_id = self._active_session
            if not session_id:
                logger.error("No session ID provided and no active session")
                return False

        try:
            # Send confirmation directly via HTTP since we need the /confirm endpoint
            with httpx.Client() as client:
                response = client.post(
                    f"{self.base_url}/confirm",
                    headers={"X-Secret-Key": self.api_key},
                    json={
                        "id": confirmation_id,
                        "principalType": principal_type,
                        "action": action,
                        "sessionId": session_id,
                    },
                )
                response.raise_for_status()
            logger.info(f"Confirmed permission {confirmation_id} with action {action}")
            return True
        except Exception as e:
            logger.error(f"Failed to confirm permission: {e}")
            return False

    def stream_chat_with_confirmations(
        self, text: str, session_id: Optional[str] = None, auto_confirm: Optional[str] = None
    ) -> Iterator[Union[str, Dict[str, Any]]]:
        """Stream chat with tool confirmation support.

        This method yields both text chunks and tool confirmation requests.
        When a tool confirmation is received, it yields the confirmation dict
        so the caller can handle it (e.g., prompt user or auto-confirm).

        Args:
            text: The message text to send
            session_id: Optional session ID (uses active session if not provided)
            auto_confirm: Optional auto-confirmation action ("allow_once", "always_allow", "deny", or None)
                         If provided, automatically confirms tools with this action

        Yields:
            Either text chunks (str) or tool confirmation dicts with structure:
            {
                'type': 'toolConfirmationRequest',
                'id': str,
                'toolName': str,
                'arguments': dict,
                'prompt': Optional[str]
            }
        """
        for event in self.send_message(text, session_id, stream=True):
            if event.get("type") == "Message":
                msg = event.get("message", {})

                # Check for tool confirmations
                tool_confirmations = self.get_tool_confirmations(event)
                for confirmation in tool_confirmations:
                    if auto_confirm:
                        # Auto-confirm with the specified action
                        self.confirm_permission(
                            confirmation["id"], auto_confirm, session_id or self._active_session
                        )
                        logger.debug(
                            f"Auto-confirmed tool {confirmation['toolName']} with action {auto_confirm}"
                        )
                    else:
                        # Yield the confirmation for the caller to handle
                        yield confirmation

                # Yield text content as before
                for content in msg.get("content", []):
                    if content.get("type") == "text":
                        text_chunk = content.get("text", "")
                        if text_chunk:
                            yield text_chunk
            elif event.get("type") == "Error":
                error_msg = event.get("error", "Unknown error")
                logger.error(f"Stream error: {error_msg}")
                break
