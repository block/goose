import json

from mcp import types as mcp_types
from pydantic import BaseModel, ConfigDict, Field
from sqlalchemy.orm import Session

from aci.common.db import crud
from aci.common.db.sql_models import MCPServerBundle
from aci.common.embeddings import generate_embedding
from aci.common.logging_setup import get_logger
from aci.common.openai_client import get_openai_client
from aci.mcp.logging import LogEvent
from aci.mcp.routes.jsonrpc import (
    JSONRPCErrorCode,
    JSONRPCErrorResponse,
    JSONRPCSuccessResponse,
    JSONRPCToolsCallRequest,
)

logger = get_logger(__name__)


class SearchToolsInputSchema(BaseModel):
    intent: str | None = Field(
        None,
        description=(
            "Use this to find relevant tools you might need. Returned results of this "
            "tool will be sorted by relevance to the intent."
        ),
    )
    limit: int = Field(
        100,
        ge=1,
        description="The maximum number of tools to return from the search per response.",
    )
    offset: int = Field(
        0,
        ge=0,
        description="Pagination offset.",
    )

    model_config = ConfigDict(extra="forbid")


SEARCH_TOOLS = mcp_types.Tool(
    name="SEARCH_TOOLS",
    description="This tool allows you to find relevant tools and their schemas that can help complete your tasks.",  # noqa: E501
    inputSchema=SearchToolsInputSchema.model_json_schema(),
)


async def handle_search_tools(
    db_session: Session,
    mcp_server_bundle: MCPServerBundle,
    jsonrpc_tools_call_request: JSONRPCToolsCallRequest,
) -> JSONRPCSuccessResponse | JSONRPCErrorResponse:
    arguments = jsonrpc_tools_call_request.params.arguments
    logger.info(f"handle search tools, arguments={arguments}")

    try:
        validated_input = SearchToolsInputSchema.model_validate(arguments)
    except Exception as e:
        logger.exception(f"Error validating search tools input: {e}")
        return JSONRPCErrorResponse(
            id=jsonrpc_tools_call_request.id,
            error=JSONRPCErrorResponse.ErrorData(
                code=JSONRPCErrorCode.INVALID_METHOD_PARAMS,
                message=str(e),
            ),
        )

    try:
        tool_call_result = await _search_tools(
            db_session,
            mcp_server_bundle,
            validated_input.intent,
            validated_input.limit,
            validated_input.offset,
        )
    except Exception as e:
        logger.exception(f"Error searching tools: {e}")
        return JSONRPCErrorResponse(
            id=jsonrpc_tools_call_request.id,
            error=JSONRPCErrorResponse.ErrorData(
                code=JSONRPCErrorCode.INTERNAL_ERROR,
                message=str(e),
            ),
        )

    return JSONRPCSuccessResponse(
        id=jsonrpc_tools_call_request.id,
        result=tool_call_result.model_dump(exclude_none=True),
    )


async def _search_tools(
    db_session: Session,
    mcp_server_bundle: MCPServerBundle,
    intent: str | None,
    limit: int,
    offset: int,
) -> mcp_types.CallToolResult:
    # TODO: use anyio.to_thread.run_sync to run the embedding generation?
    intent_embedding = generate_embedding(get_openai_client(), intent) if intent else None
    mcp_server_configurations = (
        crud.mcp_server_bundles.get_mcp_server_configurations_of_mcp_server_bundle(
            db_session,
            mcp_server_bundle,
        )
    )
    mcp_servers = [
        mcp_server_configuration.mcp_server
        for mcp_server_configuration in mcp_server_configurations
    ]

    # Get all disabled tools from all mcp server configurations of the bundle
    disabled_tool_ids = []
    for mcp_server_configuration in mcp_server_configurations:
        if not mcp_server_configuration.all_tools_enabled:
            all_tool_ids = [tool.id for tool in mcp_server_configuration.mcp_server.tools]
            disabled_tool_ids.extend(
                list(set(all_tool_ids) - set(mcp_server_configuration.enabled_tools))
            )

    mcp_tools = crud.mcp_tools.search_mcp_tools(
        db_session,
        [mcp_server.id for mcp_server in mcp_servers],
        disabled_tool_ids,
        intent_embedding,
        limit,
        offset,
    )

    logger.info(
        "search tools completed",
        extra={
            "event": LogEvent.SEARCH_TOOLS,
            f"{LogEvent.SEARCH_TOOLS}": {
                "intent": intent,
                "mcp_server_names": [mcp_server.name for mcp_server in mcp_servers],
                "limit": limit,
                "offset": offset,
                "search_results": [mcp_tool.name for mcp_tool in mcp_tools],
                "number_of_disabled_tools": len(disabled_tool_ids),
            },
        },
    )
    # TODO: consider using mcp_types.Tool if more fields are supported in the future
    mcp_tool_schemas = [
        {
            "name": mcp_tool.name,
            "description": mcp_tool.description,
            "inputSchema": mcp_tool.input_schema,
        }
        for mcp_tool in mcp_tools
    ]
    # TODO: should each tool be in its own contentblock or all in one?
    content: list[mcp_types.ContentBlock] = [
        mcp_types.TextContent(type="text", text=json.dumps(mcp_tool_schema))
        for mcp_tool_schema in mcp_tool_schemas
    ]
    return mcp_types.CallToolResult(content=content)
