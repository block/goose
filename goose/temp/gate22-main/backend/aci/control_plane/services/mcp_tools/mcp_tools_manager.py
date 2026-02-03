from dataclasses import dataclass
from datetime import UTC, datetime

from sqlalchemy.orm import Session

from aci.common import auth_credentials_manager as acm
from aci.common import embeddings, mcp_tool_utils
from aci.common.db import crud
from aci.common.db.sql_models import MCPServer
from aci.common.enums import ConnectedAccountOwnership
from aci.common.logging_setup import get_logger
from aci.common.openai_client import get_openai_client
from aci.common.schemas.mcp_server import MCPServerPartialUpdate
from aci.common.schemas.mcp_tool import MCPToolEmbeddingFields, MCPToolMetadata, MCPToolUpsert
from aci.control_plane.exceptions import MCPToolsManagerError, MCPToolsNormalizationError
from aci.control_plane.services.mcp_tools.mcp_tools_fetcher import MCPToolsFetcher

logger = get_logger(__name__)


@dataclass
class MCPToolsDiff:
    tools_created: list[str]
    tools_deleted: list[str]
    tools_updated: list[str]
    tools_unchanged: list[str]


class MCPToolsManager:
    def __init__(self, mcp_server: MCPServer):
        self.mcp_server = mcp_server

    async def refresh_mcp_tools(self, db_session: Session) -> MCPToolsDiff:
        # Remove this check if we want to support refreshing tools for Public MCP servers
        if self.mcp_server.organization_id is None:
            logger.error(f"MCP server has no organization id: {self.mcp_server.id}")
            raise MCPToolsManagerError("MCP server has no organization id")

        mcp_server_configuration = (
            crud.mcp_server_configurations.get_operational_mcp_server_configuration_mcp_server_id(
                db_session,
                mcp_server_id=self.mcp_server.id,
            )
        )
        if mcp_server_configuration is None:
            raise MCPToolsManagerError("MCP server has no operational mcp server configuration")

        auth_config = acm.get_auth_config(self.mcp_server, mcp_server_configuration)
        auth_credentials = await acm.get_auth_credentials(
            db_session,
            mcp_server_configuration.id,
            ConnectedAccountOwnership.OPERATIONAL,
            user_id=None,
        )
        # Fetch the tools
        fetcher = MCPToolsFetcher(timeout_seconds=30)
        tools = await fetcher.fetch_tools(self.mcp_server, auth_config, auth_credentials)
        logger.info(f"Fetched {len(tools)} tools")

        # Transform the data into our schema
        latest_mcp_tool_upserts = []

        # Get the existing tools in the database
        existing_mcp_tool_upserts = [
            MCPToolUpsert.model_validate(tool, from_attributes=True)
            for tool in self.mcp_server.tools
        ]
        existing_mcp_tool_upserts_dict = {tool.name: tool for tool in existing_mcp_tool_upserts}

        try:
            for tool in tools:
                sanitized_tool_name = mcp_tool_utils.sanitize_canonical_name(tool.name)

                # Note: mcp_server.name should be validated in MCPServerUpsert.validate_name()
                tool_name = f"{self.mcp_server.name}__{sanitized_tool_name}"

                tags = (
                    existing_mcp_tool_upserts_dict[tool_name].tags
                    if tool_name in existing_mcp_tool_upserts_dict
                    else []
                )
                mcp_tool_upsert = MCPToolUpsert(
                    name=tool_name,
                    description=tool.description if tool.description is not None else "",
                    input_schema=tool.inputSchema,
                    # Tags are not provided in MCP Server, and is set by users. So here we fill the
                    # tags from the existing tools if present, to avoid treating it as a change and
                    # avoid updating it unnecessarily.
                    tags=tags,
                    tool_metadata=MCPToolMetadata(
                        canonical_tool_name=tool.name,
                        canonical_tool_description_hash=mcp_tool_utils.normalize_and_hash_content(
                            tool.description
                        )
                        if tool.description is not None
                        else "",
                        canonical_tool_input_schema_hash=mcp_tool_utils.normalize_and_hash_content(
                            tool.inputSchema
                        ),
                    ),
                )
                logger.debug(f"Fetched MCP tool: {tool.name} --> {mcp_tool_upsert.name}")

                # TODO: Check and handle for duplicate tool names after sanitization
                latest_mcp_tool_upserts.append(mcp_tool_upsert)
        except Exception as e:
            # If any tool failed, abort the whole operation.
            logger.error(f"Error transforming tools: {e}")
            raise MCPToolsNormalizationError(f"Error transforming tools: {e}") from e

        # Diff the latest tools vs the existing tools in database
        (
            tools_to_create,
            tools_to_delete,
            tools_updated_embedding_fields,
            tools_updated_non_embedding_fields,
            tools_unchanged,
        ) = mcp_tool_utils.diff_tools(existing_mcp_tool_upserts, latest_mcp_tool_upserts)

        logger.debug(
            f"Tools to create ({len(tools_to_create)}): {[tool.name for tool in tools_to_create]}"
        )
        logger.debug(
            f"Tools to delete ({len(tools_to_delete)}): {[tool.name for tool in tools_to_delete]}"
        )
        logger.debug(
            f"Tools update with embedding ({len(tools_updated_embedding_fields)}): {[tool.name for tool in tools_updated_embedding_fields]}"  # noqa: E501
        )
        logger.debug(
            f"Tools update without embedding ({len(tools_updated_non_embedding_fields)}): {[tool.name for tool in tools_updated_non_embedding_fields]}"  # noqa: E501
        )
        logger.debug(
            f"Tools unchanged ({len(tools_unchanged)}): {[tool.name for tool in tools_unchanged]}"
        )

        # TODO: Existing generate_mcp_tool_embeddings() implementation embeds tool sequentailly.
        # We should make use of OpenAI API with batch embedding. Then we should put "new_mcp_tools"
        # and "updated_mcp_tools" into a same batch to improve performance.

        # Embed and create the tools that are new or updated
        new_mcp_tool_embeddings = self._embed_mcp_tools(tools_to_create)
        crud.mcp_tools.create_mcp_tools(db_session, tools_to_create, new_mcp_tool_embeddings)

        # Embed tools that has changed in the embedding fields, and update tools that has changes
        # in either embedding or non-embedding fields. (Set embedding to `None` for tools that has
        # no change in non-embedding fields)
        updated_mcp_tool_embeddings = self._embed_mcp_tools(tools_updated_embedding_fields)
        crud.mcp_tools.update_mcp_tools(
            db_session,
            tools_updated_embedding_fields + tools_updated_non_embedding_fields,
            updated_mcp_tool_embeddings + [None] * len(tools_updated_non_embedding_fields),
        )

        # Delete the tools that are old
        crud.mcp_tools.delete_mcp_tools_by_names(
            db_session, [mcp_tool.name for mcp_tool in tools_to_delete]
        )

        # TODO: Remove stale records from mcp_tool_configurations.enabled_tools that are no longer
        # exists after deletion

        # TODO: Storing the version history of the mcp tools after tool updates detected

        # Update the last synced at time
        crud.mcp_servers.update_mcp_server(
            db_session, self.mcp_server, MCPServerPartialUpdate(last_synced_at=datetime.now(UTC))
        )

        return MCPToolsDiff(
            tools_created=[tool.name for tool in tools_to_create],
            tools_deleted=[tool.name for tool in tools_to_delete],
            tools_updated=[
                tool.name
                for tool in tools_updated_embedding_fields + tools_updated_non_embedding_fields
            ],
            tools_unchanged=[tool.name for tool in tools_unchanged],
        )

    def _embed_mcp_tools(self, mcp_tools: list[MCPToolUpsert]) -> list[list[float]]:
        return embeddings.generate_mcp_tool_embeddings(
            get_openai_client(),
            [
                MCPToolEmbeddingFields.model_validate(mcp_tool.model_dump())
                for mcp_tool in mcp_tools
            ],
        )
