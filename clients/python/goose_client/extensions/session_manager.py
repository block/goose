"""Advanced session management extension"""

import logging
from datetime import datetime
from typing import Any, Dict, Optional

from .base import Extension

logger = logging.getLogger(__name__)


class SessionInfo:
    """Information about a saved session"""

    def __init__(self, session_id: str, name: str, working_dir: str = "/tmp"):
        self.id = session_id
        self.name = name
        self.working_dir = working_dir
        self.created_at = datetime.now()
        self.last_accessed = datetime.now()
        self.metadata: Dict[str, Any] = {}

    def touch(self):
        """Update last accessed time"""
        self.last_accessed = datetime.now()


class SessionManagerExtension(Extension):
    """Advanced session management capabilities"""

    def __init__(self, client, max_saved_sessions: int = 10):
        """Initialize the session manager extension.

        Args:
            client: The GooseClient instance to extend
            max_saved_sessions: Maximum number of sessions to keep saved
        """
        super().__init__(client)
        self._sessions: Dict[str, SessionInfo] = {}
        self._session_by_id: Dict[str, str] = {}  # id -> name mapping
        self.max_saved_sessions = max_saved_sessions

    def install(self):
        """Add session management methods to client"""
        self.client.session_manager = self
        logger.info("SessionManager extension installed")

    def uninstall(self):
        """Remove session management from client"""
        if hasattr(self.client, "session_manager"):
            delattr(self.client, "session_manager")
        logger.info("SessionManager extension uninstalled")

    def save_session(
        self, session_id: str, name: str, metadata: Optional[Dict[str, Any]] = None
    ) -> bool:
        """Save session with a name.

        Args:
            session_id: The session ID to save
            name: A friendly name for the session
            metadata: Optional metadata to store with the session

        Returns:
            True if saved successfully, False otherwise
        """
        try:
            # Get session details from server
            session = self.client.get_session(session_id)

            # Create session info
            info = SessionInfo(
                session_id=session_id,
                name=name,
                working_dir=getattr(session, "working_dir", "/tmp"),
            )

            if metadata:
                info.metadata = metadata

            # Check if we need to remove old sessions
            if len(self._sessions) >= self.max_saved_sessions:
                # Remove oldest session
                oldest = min(self._sessions.values(), key=lambda x: x.last_accessed)
                self.forget_session(oldest.name)

            # Save the session
            self._sessions[name] = info
            self._session_by_id[session_id] = name

            logger.info(f"Saved session {session_id} as '{name}'")
            return True

        except Exception as e:
            logger.error(f"Failed to save session {session_id}: {e}")
            return False

    def load_session(self, name: str) -> Optional[str]:
        """Load saved session by name.

        Args:
            name: The name of the saved session

        Returns:
            The session ID if found, None otherwise
        """
        if name in self._sessions:
            info = self._sessions[name]
            info.touch()

            # Set as active session
            self.client.set_active_session(info.id)

            logger.info(f"Loaded session '{name}' (ID: {info.id})")
            return info.id

        logger.warning(f"Session '{name}' not found")
        return None

    def forget_session(self, name: str) -> bool:
        """Remove a saved session from memory (doesn't delete from server).

        Args:
            name: The name of the saved session

        Returns:
            True if removed, False if not found
        """
        if name in self._sessions:
            info = self._sessions[name]
            del self._sessions[name]
            del self._session_by_id[info.id]
            logger.info(f"Forgot session '{name}'")
            return True
        return False

    def list_saved(self) -> Dict[str, Dict[str, Any]]:
        """List saved sessions.

        Returns:
            Dictionary mapping names to session information
        """
        return {
            name: {
                "id": info.id,
                "working_dir": info.working_dir,
                "created_at": info.created_at.isoformat(),
                "last_accessed": info.last_accessed.isoformat(),
                "metadata": info.metadata,
            }
            for name, info in self._sessions.items()
        }

    def get_session_by_name(self, name: str) -> Optional[SessionInfo]:
        """Get session info by name.

        Args:
            name: The name of the saved session

        Returns:
            SessionInfo if found, None otherwise
        """
        return self._sessions.get(name)

    def get_session_name(self, session_id: str) -> Optional[str]:
        """Get the name of a session by its ID.

        Args:
            session_id: The session ID

        Returns:
            The session name if found, None otherwise
        """
        return self._session_by_id.get(session_id)

    def rename_session(self, old_name: str, new_name: str) -> bool:
        """Rename a saved session.

        Args:
            old_name: Current name of the session
            new_name: New name for the session

        Returns:
            True if renamed successfully, False otherwise
        """
        if old_name in self._sessions and new_name not in self._sessions:
            info = self._sessions[old_name]
            del self._sessions[old_name]
            self._sessions[new_name] = info
            logger.info(f"Renamed session '{old_name}' to '{new_name}'")
            return True
        return False

    def update_metadata(self, name: str, metadata: Dict[str, Any]) -> bool:
        """Update metadata for a saved session.

        Args:
            name: Name of the session
            metadata: New metadata to merge with existing

        Returns:
            True if updated, False if session not found
        """
        if name in self._sessions:
            self._sessions[name].metadata.update(metadata)
            self._sessions[name].touch()
            return True
        return False
