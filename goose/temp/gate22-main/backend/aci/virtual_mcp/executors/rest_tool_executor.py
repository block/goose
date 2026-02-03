import base64
import json
from typing import override

import httpx
from httpx import HTTPStatusError
from mcp import types as mcp_types

from aci.common.db.sql_models import VirtualMCPTool
from aci.common.enums import HttpLocation
from aci.common.logging_setup import get_logger
from aci.common.schemas.virtual_mcp import RestVirtualMCPToolMetadata, VirtualMCPAuthTokenData
from aci.virtual_mcp.executors.base_executor import ToolExecutor

logger = get_logger(__name__)


class RestFunctionExecutor(ToolExecutor):
    """
    Tool executor for REST type tools.
    """

    def _inject_credentials(
        self,
        headers: dict,
        query: dict,
        body: dict,
        cookies: dict,
        auth_token_data: VirtualMCPAuthTokenData,
    ) -> None:
        token_with_prefix = (
            f"{auth_token_data.prefix} {auth_token_data.token}"
            if auth_token_data.prefix
            else auth_token_data.token
        )
        match auth_token_data.location:
            case HttpLocation.HEADER:
                headers[auth_token_data.name] = token_with_prefix
            case HttpLocation.QUERY:
                query[auth_token_data.name] = token_with_prefix
            case HttpLocation.BODY:
                body[auth_token_data.name] = token_with_prefix
            case HttpLocation.COOKIE:
                cookies[auth_token_data.name] = token_with_prefix
            case HttpLocation.PATH:
                # should never happen
                logger.error("path location is not supported for auth token")
                raise NotImplementedError("path location is not supported for auth token")

    @override
    def _execute(
        self,
        tool: VirtualMCPTool,
        tool_arguments: dict,
        auth_token_data: VirtualMCPAuthTokenData | None,
    ) -> mcp_types.CallToolResult:
        try:
            # Extract parameters by location
            path: dict = tool_arguments.get("path", {})
            query: dict = tool_arguments.get("query", {})
            headers: dict = tool_arguments.get("header", {})
            cookies: dict = tool_arguments.get("cookie", {})
            body: dict = tool_arguments.get("body", {})

            tool_metadata = RestVirtualMCPToolMetadata.model_validate(tool.tool_metadata)
            # Replace placeholder in url path with actual parameters.
            url = tool_metadata.endpoint
            if path:
                for path_param_name, path_param_value in path.items():
                    url = url.replace(f"{{{path_param_name}}}", str(path_param_value))

            if auth_token_data:
                self._inject_credentials(headers, query, body, cookies, auth_token_data)

            request = httpx.Request(
                method=tool_metadata.method,
                url=url,
                params=query if query else None,
                headers=headers if headers else None,
                cookies=cookies if cookies else None,
                json=body if body else None,
            )

            logger.info(
                f"Executing tool via raw http request, tool_name={tool.name}, method={request.method}, url={request.url}"  # noqa: E501
            )

            return self._send_request(request)
        except Exception as e:
            logger.exception(
                f"Error executing tool, tool_name={tool.name}, tool_arguments={tool_arguments}, error={e}"  # noqa: E501
            )
            return mcp_types.CallToolResult(
                isError=True,
                content=[mcp_types.TextContent(type="text", text=str(e))],
            )

    def _send_request(self, request: httpx.Request) -> mcp_types.CallToolResult:
        # TODO: one client for all requests? cache the client? concurrency control? async client?
        # TODO: add retry
        timeout = httpx.Timeout(10.0, read=30.0)
        with httpx.Client(timeout=timeout) as client:
            try:
                response = client.send(request)
            except Exception as e:
                logger.exception(f"Failed to send function execution http request, error={e}")
                return mcp_types.CallToolResult(
                    isError=True, content=[mcp_types.TextContent(type="text", text=str(e))]
                )

            try:
                response.raise_for_status()
            except httpx.HTTPStatusError as e:
                logger.exception(f"HTTP error occurred for function execution, error={e}")
                return mcp_types.CallToolResult(
                    isError=True,
                    content=[
                        mcp_types.TextContent(
                            type="text", text=self._parse_error_message(response, e)
                        )
                    ],
                )

            try:
                data = self._parse_response_data_to_str(response)
            except Exception:
                logger.exception("Error parsing response data")
                return mcp_types.CallToolResult(
                    isError=True,
                    content=[
                        mcp_types.TextContent(type="text", text="Error parsing response data")
                    ],
                )

            return mcp_types.CallToolResult(content=[mcp_types.TextContent(type="text", text=data)])

    # TODO: we return everything as Text response for now, including image, audio, etc.
    def _parse_response_data_to_str(self, response: httpx.Response) -> str:
        if not response.content:
            return ""

        content_type = (response.headers.get("content-type") or "").lower()

        if "application/json" in content_type or content_type.endswith("+json"):
            try:
                data = response.json()
            except Exception:
                logger.exception("Error parsing function execution http response as JSON")
                return response.text
            if isinstance(data, str):
                return data
            return json.dumps(data, ensure_ascii=False)

        if content_type.startswith("text/") or content_type in {
            "application/xml",
            "application/javascript",
            "application/x-www-form-urlencoded",
        }:
            encoding = response.encoding or "utf-8"
            return response.content.decode(encoding, errors="replace")

        # fallback for images/audio/binary → base64
        return base64.b64encode(response.content).decode("ascii")

    def _parse_error_message(self, response: httpx.Response, error: HTTPStatusError) -> str:
        """
        Get the error message from the response or fallback to the error message from the
        HTTPStatusError. Usually the response json contains more details about the error.
        """
        try:
            return str(response.json())
        except Exception:
            return str(error)
