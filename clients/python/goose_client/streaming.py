"""SSE streaming support - essential feature"""

import json
import logging
from typing import Any, AsyncIterator, Dict, Iterator, List, Optional

import httpx
from httpx_sse import aconnect_sse, connect_sse

logger = logging.getLogger(__name__)


class StreamingMixin:
    """Streaming support for Goose client"""

    def stream_reply(
        self, session_id: str, messages: List[Dict[str, Any]], timeout: float = 60.0
    ) -> Iterator[Dict[str, Any]]:
        """Stream responses from /reply endpoint.

        Args:
            session_id: The session ID to send messages to
            messages: List of message dictionaries
            timeout: Request timeout in seconds

        Yields:
            Parsed SSE event data as dictionaries
        """
        request_body = {"session_id": session_id, "messages": messages}

        logger.debug(f"Starting SSE stream for session {session_id}")

        try:
            with httpx.Client(timeout=timeout) as client:
                with connect_sse(
                    client,
                    "POST",
                    f"{self.base_url}/reply",
                    headers={"X-Secret-Key": self.api_key},
                    json=request_body,
                ) as event_source:
                    for sse in event_source.iter_sse():
                        if sse.data:
                            try:
                                event_data = json.loads(sse.data)
                                logger.debug(
                                    f"Received SSE event: {event_data.get('type', 'unknown')}"
                                )
                                yield event_data
                            except json.JSONDecodeError as e:
                                logger.warning(f"Failed to parse SSE data: {e}")
                                continue
        except httpx.TimeoutException:
            logger.error(f"Stream timeout after {timeout} seconds")
            yield {"type": "Error", "error": f"Request timeout after {timeout} seconds"}
        except httpx.HTTPError as e:
            logger.error(f"HTTP error during streaming: {e}")
            yield {"type": "Error", "error": str(e)}
        except Exception as e:
            logger.error(f"Unexpected error during streaming: {e}")
            yield {"type": "Error", "error": str(e)}

    async def astream_reply(
        self, session_id: str, messages: List[Dict[str, Any]], timeout: float = 60.0
    ) -> AsyncIterator[Dict[str, Any]]:
        """Async streaming from /reply endpoint.

        Args:
            session_id: The session ID to send messages to
            messages: List of message dictionaries
            timeout: Request timeout in seconds

        Yields:
            Parsed SSE event data as dictionaries
        """
        request_body = {"session_id": session_id, "messages": messages}

        logger.debug(f"Starting async SSE stream for session {session_id}")

        try:
            async with httpx.AsyncClient(timeout=timeout) as client:
                async with aconnect_sse(
                    client,
                    "POST",
                    f"{self.base_url}/reply",
                    headers={"X-Secret-Key": self.api_key},
                    json=request_body,
                ) as event_source:
                    async for sse in event_source.aiter_sse():
                        if sse.data:
                            try:
                                event_data = json.loads(sse.data)
                                logger.debug(
                                    f"Received async SSE event: {event_data.get('type', 'unknown')}"
                                )
                                yield event_data
                            except json.JSONDecodeError as e:
                                logger.warning(f"Failed to parse SSE data: {e}")
                                continue
        except httpx.TimeoutException:
            logger.error(f"Async stream timeout after {timeout} seconds")
            yield {"type": "Error", "error": f"Request timeout after {timeout} seconds"}
        except httpx.HTTPError as e:
            logger.error(f"Async HTTP error during streaming: {e}")
            yield {"type": "Error", "error": str(e)}
        except Exception as e:
            logger.error(f"Unexpected async error during streaming: {e}")
            yield {"type": "Error", "error": str(e)}

    def parse_stream_content(self, event: Dict[str, Any]) -> Optional[str]:
        """Helper to extract text content from stream events.

        Args:
            event: SSE event dictionary

        Returns:
            Extracted text content or None
        """
        if event.get("type") == "Message":
            message = event.get("message", {})
            text_parts = []
            for content in message.get("content", []):
                if content.get("type") == "text":
                    text_parts.append(content.get("text", ""))
            return "".join(text_parts) if text_parts else None
        return None

    def has_tool_confirmation(self, event: Dict[str, Any]) -> bool:
        """Check if event contains a tool confirmation request.

        Args:
            event: SSE event dictionary

        Returns:
            True if event has tool confirmation request
        """
        if event.get("type") == "Message":
            message = event.get("message", {})
            for content in message.get("content", []):
                if content.get("type") == "toolConfirmationRequest":
                    return True
        return False

    def get_tool_confirmations(self, event: Dict[str, Any]) -> List[Dict[str, Any]]:
        """Extract tool confirmation requests from event.

        Args:
            event: SSE event dictionary

        Returns:
            List of tool confirmation request dictionaries
        """
        confirmations = []
        if event.get("type") == "Message":
            message = event.get("message", {})
            for content in message.get("content", []):
                if content.get("type") == "toolConfirmationRequest":
                    confirmations.append(content)
        return confirmations
