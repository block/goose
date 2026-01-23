"""
goosed-sdk - Python SDK for goosed API

Main client implementation.
"""

import json
import time
from collections.abc import AsyncGenerator, Generator
from typing import Any

import httpx

from .exceptions import (
    GoosedAgentNotInitializedError,
    GoosedAuthError,
    GoosedConnectionError,
    GoosedException,
    GoosedNotFoundError,
    GoosedServerError,
)
from .types import (
    CallToolResponse,
    ExtensionResult,
    GoosedClientOptions,
    Session,
    SSEEvent,
    SystemInfo,
    TokenState,
    ToolInfo,
)


class GoosedClient:
    """Client for the goosed API."""

    def __init__(
        self,
        base_url: str | None = None,
        secret_key: str | None = None,
        timeout: float | None = None,
        options: GoosedClientOptions | None = None,
    ):
        """
        Initialize the GoosedClient.

        Args:
            base_url: Base URL of the goosed server (default: http://127.0.0.1:3000)
            secret_key: Secret key for authentication (default: test)
            timeout: Request timeout in seconds (default: 30.0)
            options: Alternative way to pass options as GoosedClientOptions
        """
        if options is not None:
            self._base_url = options.base_url.rstrip("/")
            self._secret_key = options.secret_key
            self._timeout = options.timeout
        else:
            self._base_url = (base_url or "http://127.0.0.1:3000").rstrip("/")
            self._secret_key = secret_key or "test"
            self._timeout = timeout or 30.0

    def _headers(self) -> dict[str, str]:
        """Get headers for API requests."""
        return {
            "Content-Type": "application/json",
            "x-secret-key": self._secret_key,
        }

    def _handle_response(self, response: httpx.Response) -> Any:
        """Handle HTTP response and raise appropriate exceptions."""
        if response.is_success:
            content_type = response.headers.get("content-type", "")
            if "application/json" in content_type:
                text = response.text
                if text == "":
                    return None
                return response.json()
            text = response.text
            if text == "":
                return None
            return text

        text = response.text
        status = response.status_code

        if status == 401:
            raise GoosedAuthError()
        elif status == 404:
            raise GoosedNotFoundError()
        elif status == 424:
            raise GoosedAgentNotInitializedError()
        elif status >= 500:
            raise GoosedServerError(text)
        else:
            raise GoosedException(f"HTTP {status}: {text}", status_code=status)

    def _get(self, path: str, params: dict[str, str] | None = None) -> Any:
        """Make a GET request."""
        url = f"{self._base_url}{path}"
        try:
            with httpx.Client(timeout=self._timeout) as client:
                response = client.get(url, headers=self._headers(), params=params)
                return self._handle_response(response)
        except httpx.ConnectError as e:
            raise GoosedConnectionError(str(e)) from e
        except httpx.TimeoutException as e:
            raise GoosedConnectionError("Request timed out") from e

    def _post(self, path: str, body: dict[str, Any] | None = None) -> Any:
        """Make a POST request."""
        url = f"{self._base_url}{path}"
        try:
            with httpx.Client(timeout=self._timeout) as client:
                response = client.post(
                    url,
                    headers=self._headers(),
                    json=body,
                )
                return self._handle_response(response)
        except httpx.ConnectError as e:
            raise GoosedConnectionError(str(e)) from e
        except httpx.TimeoutException as e:
            raise GoosedConnectionError("Request timed out") from e

    def _put(self, path: str, body: dict[str, Any] | None = None) -> Any:
        """Make a PUT request."""
        url = f"{self._base_url}{path}"
        try:
            with httpx.Client(timeout=self._timeout) as client:
                response = client.put(
                    url,
                    headers=self._headers(),
                    json=body,
                )
                return self._handle_response(response)
        except httpx.ConnectError as e:
            raise GoosedConnectionError(str(e)) from e
        except httpx.TimeoutException as e:
            raise GoosedConnectionError("Request timed out") from e

    def _delete(self, path: str) -> Any:
        """Make a DELETE request."""
        url = f"{self._base_url}{path}"
        try:
            with httpx.Client(timeout=self._timeout) as client:
                response = client.delete(url, headers=self._headers())
                return self._handle_response(response)
        except httpx.ConnectError as e:
            raise GoosedConnectionError(str(e)) from e
        except httpx.TimeoutException as e:
            raise GoosedConnectionError("Request timed out") from e

    # === Status APIs ===

    def status(self) -> str:
        """Check server status. Returns 'ok' if server is running."""
        return self._get("/status")

    def system_info(self) -> SystemInfo:
        """Get system information."""
        data = self._get("/system_info")
        return SystemInfo(
            app_version=data["app_version"],
            os=data["os"],
            os_version=data["os_version"],
            architecture=data["architecture"],
            provider=data["provider"],
            model=data["model"],
            enabled_extensions=data.get("enabled_extensions", []),
        )

    # === Agent APIs ===

    def start_session(self, working_dir: str) -> Session:
        """Create a new session with the specified working directory."""
        data = self._post("/agent/start", {"working_dir": working_dir})
        return self._parse_session(data)

    def resume_session(
        self, session_id: str, load_model_and_extensions: bool = True
    ) -> tuple[Session, list[ExtensionResult]]:
        """
        Resume an existing session.

        Args:
            session_id: ID of the session to resume
            load_model_and_extensions: Whether to load model and extensions

        Returns:
            Tuple of (session, extension_results)
        """
        data = self._post(
            "/agent/resume",
            {
                "session_id": session_id,
                "load_model_and_extensions": load_model_and_extensions,
            },
        )
        session = self._parse_session(data["session"])
        extension_results = [
            ExtensionResult(name=r["name"], success=r["success"])
            for r in data.get("extension_results", [])
        ]
        return session, extension_results

    def restart_session(self, session_id: str) -> list[ExtensionResult]:
        """Restart the agent in a session."""
        data = self._post("/agent/restart", {"session_id": session_id})
        return [
            ExtensionResult(name=r["name"], success=r["success"])
            for r in data.get("extension_results", [])
        ]

    def stop_session(self, session_id: str) -> None:
        """Stop an active session."""
        self._post("/agent/stop", {"session_id": session_id})

    def get_tools(self, session_id: str, extension_name: str | None = None) -> list[ToolInfo]:
        """Get available tools for a session."""
        params: dict[str, str] = {"session_id": session_id}
        if extension_name:
            params["extension_name"] = extension_name
        data = self._get("/agent/tools", params)
        return [
            ToolInfo(
                name=t["name"],
                description=t["description"],
                parameters=t.get("parameters", []),
                permission=t.get("permission"),
            )
            for t in data
        ]

    def call_tool(self, session_id: str, name: str, arguments: dict[str, Any]) -> CallToolResponse:
        """Call a tool directly."""
        data = self._post(
            "/agent/call_tool",
            {
                "session_id": session_id,
                "name": name,
                "arguments": arguments,
            },
        )
        return CallToolResponse(
            content=data["content"],
            is_error=data["is_error"],
        )

    # === Chat APIs ===

    def send_message(self, session_id: str, text: str) -> Generator[SSEEvent, None, None]:
        """
        Send a message and stream SSE events.

        Args:
            session_id: ID of the session
            text: Message text to send

        Yields:
            SSEEvent objects
        """
        message = {
            "role": "user",
            "created": int(time.time()),
            "content": [{"type": "text", "text": text}],
            "metadata": {"userVisible": True, "agentVisible": True},
        }

        url = f"{self._base_url}/reply"
        body = {"session_id": session_id, "user_message": message}

        try:
            with httpx.Client(timeout=None) as client, client.stream(
                "POST",
                url,
                headers=self._headers(),
                json=body,
            ) as response:
                if not response.is_success:
                    # Read the error response
                    response.read()
                    self._handle_response(response)

                buffer = ""
                data_lines: list[str] = []

                for chunk in response.iter_text():
                    buffer += chunk
                    lines = buffer.split("\n")
                    buffer = lines.pop()

                    for line in lines:
                        trimmed = line.rstrip("\r")
                        if trimmed == "":
                            if data_lines:
                                data = json.loads("\n".join(data_lines))
                                data_lines = []
                                yield self._parse_sse_event(data)
                            continue
                        if trimmed.startswith("data:"):
                            data_lines.append(trimmed[5:].lstrip())

                if data_lines:
                    data = json.loads("\n".join(data_lines))
                    yield self._parse_sse_event(data)

        except httpx.ConnectError as e:
            raise GoosedConnectionError(str(e)) from e
        except httpx.TimeoutException as e:
            raise GoosedConnectionError("Request timed out") from e

    def chat(self, session_id: str, text: str) -> str:
        """
        Send a message and get the full response.

        Args:
            session_id: ID of the session
            text: Message text to send

        Returns:
            The assistant's response text
        """
        response_text = ""
        for event in self.send_message(session_id, text):
            if event.type == "Message" and event.message:
                content = event.message.get("content", [])
                for c in content:
                    if c.get("type") == "text" and c.get("text"):
                        response_text += c["text"]
            elif event.type == "Error":
                raise GoosedException(event.error or "Unknown error")
        return response_text

    # === Session APIs ===

    def list_sessions(self) -> list[Session]:
        """List all sessions."""
        data = self._get("/sessions")
        return [self._parse_session(s) for s in data.get("sessions", [])]

    def get_session(self, session_id: str) -> Session:
        """Get session details."""
        data = self._get(f"/sessions/{session_id}")
        return self._parse_session(data)

    def update_session_name(self, session_id: str, name: str) -> None:
        """Update a session's name."""
        self._put(f"/sessions/{session_id}/name", {"name": name})

    def delete_session(self, session_id: str) -> None:
        """Delete a session."""
        self._delete(f"/sessions/{session_id}")

    def export_session(self, session_id: str) -> str:
        """Export session data."""
        return self._get(f"/sessions/{session_id}/export")

    # === Helper methods ===

    def _parse_session(self, data: dict[str, Any]) -> Session:
        """Parse session data from API response."""
        return Session(
            id=data["id"],
            name=data["name"],
            working_dir=data["working_dir"],
            session_type=data["session_type"],
            created_at=data["created_at"],
            updated_at=data["updated_at"],
            user_set_name=data.get("user_set_name"),
            message_count=data.get("message_count"),
            total_tokens=data.get("total_tokens"),
            input_tokens=data.get("input_tokens"),
            output_tokens=data.get("output_tokens"),
            provider_name=data.get("provider_name"),
            conversation=data.get("conversation"),
        )

    def _parse_sse_event(self, data: dict[str, Any]) -> SSEEvent:
        """Parse SSE event data."""
        token_state = None
        if "token_state" in data:
            ts = data["token_state"]
            token_state = TokenState(
                input_tokens=ts.get("inputTokens", 0),
                output_tokens=ts.get("outputTokens", 0),
                total_tokens=ts.get("totalTokens", 0),
                accumulated_input_tokens=ts.get("accumulatedInputTokens", 0),
                accumulated_output_tokens=ts.get("accumulatedOutputTokens", 0),
                accumulated_total_tokens=ts.get("accumulatedTotalTokens", 0),
            )
        return SSEEvent(
            type=data["type"],
            message=data.get("message"),
            token_state=token_state,
            reason=data.get("reason"),
            error=data.get("error"),
        )


# Async client for use with asyncio
class AsyncGoosedClient:
    """Async client for the goosed API."""

    def __init__(
        self,
        base_url: str | None = None,
        secret_key: str | None = None,
        timeout: float | None = None,
        options: GoosedClientOptions | None = None,
    ):
        """
        Initialize the AsyncGoosedClient.

        Args:
            base_url: Base URL of the goosed server (default: http://127.0.0.1:3000)
            secret_key: Secret key for authentication (default: test)
            timeout: Request timeout in seconds (default: 30.0)
            options: Alternative way to pass options as GoosedClientOptions
        """
        if options is not None:
            self._base_url = options.base_url.rstrip("/")
            self._secret_key = options.secret_key
            self._timeout = options.timeout
        else:
            self._base_url = (base_url or "http://127.0.0.1:3000").rstrip("/")
            self._secret_key = secret_key or "test"
            self._timeout = timeout or 30.0

    def _headers(self) -> dict[str, str]:
        """Get headers for API requests."""
        return {
            "Content-Type": "application/json",
            "x-secret-key": self._secret_key,
        }

    def _handle_response(self, response: httpx.Response) -> Any:
        """Handle HTTP response and raise appropriate exceptions."""
        if response.is_success:
            content_type = response.headers.get("content-type", "")
            if "application/json" in content_type:
                text = response.text
                if text == "":
                    return None
                return response.json()
            text = response.text
            if text == "":
                return None
            return text

        text = response.text
        status = response.status_code

        if status == 401:
            raise GoosedAuthError()
        elif status == 404:
            raise GoosedNotFoundError()
        elif status == 424:
            raise GoosedAgentNotInitializedError()
        elif status >= 500:
            raise GoosedServerError(text)
        else:
            raise GoosedException(f"HTTP {status}: {text}", status_code=status)

    async def _get(self, path: str, params: dict[str, str] | None = None) -> Any:
        """Make a GET request."""
        url = f"{self._base_url}{path}"
        try:
            async with httpx.AsyncClient(timeout=self._timeout) as client:
                response = await client.get(url, headers=self._headers(), params=params)
                return self._handle_response(response)
        except httpx.ConnectError as e:
            raise GoosedConnectionError(str(e)) from e
        except httpx.TimeoutException as e:
            raise GoosedConnectionError("Request timed out") from e

    async def _post(self, path: str, body: dict[str, Any] | None = None) -> Any:
        """Make a POST request."""
        url = f"{self._base_url}{path}"
        try:
            async with httpx.AsyncClient(timeout=self._timeout) as client:
                response = await client.post(
                    url,
                    headers=self._headers(),
                    json=body,
                )
                return self._handle_response(response)
        except httpx.ConnectError as e:
            raise GoosedConnectionError(str(e)) from e
        except httpx.TimeoutException as e:
            raise GoosedConnectionError("Request timed out") from e

    async def _put(self, path: str, body: dict[str, Any] | None = None) -> Any:
        """Make a PUT request."""
        url = f"{self._base_url}{path}"
        try:
            async with httpx.AsyncClient(timeout=self._timeout) as client:
                response = await client.put(
                    url,
                    headers=self._headers(),
                    json=body,
                )
                return self._handle_response(response)
        except httpx.ConnectError as e:
            raise GoosedConnectionError(str(e)) from e
        except httpx.TimeoutException as e:
            raise GoosedConnectionError("Request timed out") from e

    async def _delete(self, path: str) -> Any:
        """Make a DELETE request."""
        url = f"{self._base_url}{path}"
        try:
            async with httpx.AsyncClient(timeout=self._timeout) as client:
                response = await client.delete(url, headers=self._headers())
                return self._handle_response(response)
        except httpx.ConnectError as e:
            raise GoosedConnectionError(str(e)) from e
        except httpx.TimeoutException as e:
            raise GoosedConnectionError("Request timed out") from e

    # === Status APIs ===

    async def status(self) -> str:
        """Check server status. Returns 'ok' if server is running."""
        return await self._get("/status")

    async def system_info(self) -> SystemInfo:
        """Get system information."""
        data = await self._get("/system_info")
        return SystemInfo(
            app_version=data["app_version"],
            os=data["os"],
            os_version=data["os_version"],
            architecture=data["architecture"],
            provider=data["provider"],
            model=data["model"],
            enabled_extensions=data.get("enabled_extensions", []),
        )

    # === Agent APIs ===

    async def start_session(self, working_dir: str) -> Session:
        """Create a new session with the specified working directory."""
        data = await self._post("/agent/start", {"working_dir": working_dir})
        return self._parse_session(data)

    async def resume_session(
        self, session_id: str, load_model_and_extensions: bool = True
    ) -> tuple[Session, list[ExtensionResult]]:
        """Resume an existing session."""
        data = await self._post(
            "/agent/resume",
            {
                "session_id": session_id,
                "load_model_and_extensions": load_model_and_extensions,
            },
        )
        session = self._parse_session(data["session"])
        extension_results = [
            ExtensionResult(name=r["name"], success=r["success"])
            for r in data.get("extension_results", [])
        ]
        return session, extension_results

    async def restart_session(self, session_id: str) -> list[ExtensionResult]:
        """Restart the agent in a session."""
        data = await self._post("/agent/restart", {"session_id": session_id})
        return [
            ExtensionResult(name=r["name"], success=r["success"])
            for r in data.get("extension_results", [])
        ]

    async def stop_session(self, session_id: str) -> None:
        """Stop an active session."""
        await self._post("/agent/stop", {"session_id": session_id})

    async def get_tools(self, session_id: str, extension_name: str | None = None) -> list[ToolInfo]:
        """Get available tools for a session."""
        params: dict[str, str] = {"session_id": session_id}
        if extension_name:
            params["extension_name"] = extension_name
        data = await self._get("/agent/tools", params)
        return [
            ToolInfo(
                name=t["name"],
                description=t["description"],
                parameters=t.get("parameters", []),
                permission=t.get("permission"),
            )
            for t in data
        ]

    async def call_tool(
        self, session_id: str, name: str, arguments: dict[str, Any]
    ) -> CallToolResponse:
        """Call a tool directly."""
        data = await self._post(
            "/agent/call_tool",
            {
                "session_id": session_id,
                "name": name,
                "arguments": arguments,
            },
        )
        return CallToolResponse(
            content=data["content"],
            is_error=data["is_error"],
        )

    # === Chat APIs ===

    async def send_message(self, session_id: str, text: str) -> AsyncGenerator[SSEEvent, None]:
        """
        Send a message and stream SSE events.

        Args:
            session_id: ID of the session
            text: Message text to send

        Yields:
            SSEEvent objects
        """
        message = {
            "role": "user",
            "created": int(time.time()),
            "content": [{"type": "text", "text": text}],
            "metadata": {"userVisible": True, "agentVisible": True},
        }

        url = f"{self._base_url}/reply"
        body = {"session_id": session_id, "user_message": message}

        try:
            async with httpx.AsyncClient(timeout=None) as client, client.stream(
                "POST",
                url,
                headers=self._headers(),
                json=body,
            ) as response:
                if not response.is_success:
                    await response.aread()
                    self._handle_response(response)

                buffer = ""
                data_lines: list[str] = []

                async for chunk in response.aiter_text():
                    buffer += chunk
                    lines = buffer.split("\n")
                    buffer = lines.pop()

                    for line in lines:
                        trimmed = line.rstrip("\r")
                        if trimmed == "":
                            if data_lines:
                                data = json.loads("\n".join(data_lines))
                                data_lines = []
                                yield self._parse_sse_event(data)
                            continue
                        if trimmed.startswith("data:"):
                            data_lines.append(trimmed[5:].lstrip())

                if data_lines:
                    data = json.loads("\n".join(data_lines))
                    yield self._parse_sse_event(data)

        except httpx.ConnectError as e:
            raise GoosedConnectionError(str(e)) from e
        except httpx.TimeoutException as e:
            raise GoosedConnectionError("Request timed out") from e

    async def chat(self, session_id: str, text: str) -> str:
        """
        Send a message and get the full response.

        Args:
            session_id: ID of the session
            text: Message text to send

        Returns:
            The assistant's response text
        """
        response_text = ""
        async for event in self.send_message(session_id, text):
            if event.type == "Message" and event.message:
                content = event.message.get("content", [])
                for c in content:
                    if c.get("type") == "text" and c.get("text"):
                        response_text += c["text"]
            elif event.type == "Error":
                raise GoosedException(event.error or "Unknown error")
        return response_text

    # === Session APIs ===

    async def list_sessions(self) -> list[Session]:
        """List all sessions."""
        data = await self._get("/sessions")
        return [self._parse_session(s) for s in data.get("sessions", [])]

    async def get_session(self, session_id: str) -> Session:
        """Get session details."""
        data = await self._get(f"/sessions/{session_id}")
        return self._parse_session(data)

    async def update_session_name(self, session_id: str, name: str) -> None:
        """Update a session's name."""
        await self._put(f"/sessions/{session_id}/name", {"name": name})

    async def delete_session(self, session_id: str) -> None:
        """Delete a session."""
        await self._delete(f"/sessions/{session_id}")

    async def export_session(self, session_id: str) -> str:
        """Export session data."""
        return await self._get(f"/sessions/{session_id}/export")

    # === Helper methods ===

    def _parse_session(self, data: dict[str, Any]) -> Session:
        """Parse session data from API response."""
        return Session(
            id=data["id"],
            name=data["name"],
            working_dir=data["working_dir"],
            session_type=data["session_type"],
            created_at=data["created_at"],
            updated_at=data["updated_at"],
            user_set_name=data.get("user_set_name"),
            message_count=data.get("message_count"),
            total_tokens=data.get("total_tokens"),
            input_tokens=data.get("input_tokens"),
            output_tokens=data.get("output_tokens"),
            provider_name=data.get("provider_name"),
            conversation=data.get("conversation"),
        )

    def _parse_sse_event(self, data: dict[str, Any]) -> SSEEvent:
        """Parse SSE event data."""
        token_state = None
        if "token_state" in data:
            ts = data["token_state"]
            token_state = TokenState(
                input_tokens=ts.get("inputTokens", 0),
                output_tokens=ts.get("outputTokens", 0),
                total_tokens=ts.get("totalTokens", 0),
                accumulated_input_tokens=ts.get("accumulatedInputTokens", 0),
                accumulated_output_tokens=ts.get("accumulatedOutputTokens", 0),
                accumulated_total_tokens=ts.get("accumulatedTotalTokens", 0),
            )
        return SSEEvent(
            type=data["type"],
            message=data.get("message"),
            token_state=token_state,
            reason=data.get("reason"),
            error=data.get("error"),
        )
