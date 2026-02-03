from enum import IntEnum
from typing import Any, Literal

from mcp import types as mcp_types
from pydantic import BaseModel, ConfigDict, Field


class JSONRPCPayload(BaseModel):
    jsonrpc: Literal["2.0"]
    id: int | str | None = None
    method: str
    params: dict[str, Any] | None = None


class JSONRPCInitializeRequest(BaseModel):
    class InitializeParams(BaseModel):
        protocol_version: str = Field(alias="protocolVersion")
        # TODO: use a more specific type for capabilities and clientInfo?
        capabilities: dict = Field(default_factory=dict)
        client_info: dict = Field(default_factory=dict, alias="clientInfo")

    jsonrpc: Literal["2.0"] = "2.0"
    id: int | str
    method: Literal["initialize"]
    params: InitializeParams


class JSONRPCToolsListRequest(BaseModel):
    jsonrpc: Literal["2.0"] = "2.0"
    id: int | str
    method: Literal["tools/list"]


class JSONRPCToolsCallRequest(BaseModel):
    # TODO: slight modification to the mcp_types.CallToolRequestParams
    # might need to change back if we want to support direct tool call without going through the
    # "EXECUTE_TOOL" tool
    class CallToolRequestParams(mcp_types.RequestParams):
        """Parameters for calling a tool."""

        name: str
        arguments: dict[str, Any]
        model_config = ConfigDict(extra="allow")

    jsonrpc: Literal["2.0"] = "2.0"
    id: int | str
    method: Literal["tools/call"]
    # TODO: use more mcp sdk types like CallToolRequestParams for the rest of the types
    params: CallToolRequestParams


class JSONRPCNotificationInitialized(BaseModel):
    jsonrpc: Literal["2.0"] = "2.0"
    method: Literal["notifications/initialized"]
    params: dict = Field(default_factory=dict)


class JSONRPCPingRequest(BaseModel):
    jsonrpc: Literal["2.0"] = "2.0"
    id: int | str
    method: Literal["ping"]


class JSONRPCSuccessResponse(BaseModel):
    jsonrpc: Literal["2.0"] = "2.0"
    id: int | str
    result: dict = Field(default_factory=dict)


class JSONRPCErrorResponse(BaseModel):
    class ErrorData(BaseModel):
        """Error information for JSON-RPC error responses."""

        code: int
        """The error type that occurred."""

        message: str
        """
        A short description of the error. The message SHOULD be limited to a concise single
        sentence.
        """

        data: Any | None = None
        """
        Additional information about the error. The value of this member is defined by the
        sender (e.g. detailed error information, nested errors etc.).
        """
        model_config = ConfigDict(extra="allow")

    jsonrpc: Literal["2.0"] = "2.0"
    id: int | str | None = None
    error: ErrorData

    model_config = ConfigDict(extra="allow")


class JSONRPCErrorCode(IntEnum):
    PARSE_ERROR = -32700
    INVALID_REQUEST = -32600
    METHOD_NOT_FOUND = -32601
    INVALID_METHOD_PARAMS = -32602
    INTERNAL_ERROR = -32603
