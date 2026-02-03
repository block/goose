from abc import ABC, abstractmethod

import jsonschema
from mcp import types as mcp_types

from aci.common.db.sql_models import VirtualMCPTool
from aci.common.enums import VirtualMCPToolType
from aci.common.logging_setup import get_logger
from aci.common.schemas.virtual_mcp import VirtualMCPAuthTokenData, VirtualMCPToolMetadata
from aci.virtual_mcp.utils import (
    filter_visible_properties,
    inject_required_but_invisible_defaults,
    remove_none_values,
)

logger = get_logger(__name__)


class ToolExecutor(ABC):
    """
    Base class for tool executors.
    """

    def execute(
        self,
        tool: VirtualMCPTool,
        tool_arguments: dict,
        auth_token_data: VirtualMCPAuthTokenData | None,
    ) -> mcp_types.CallToolResult | mcp_types.ErrorData:
        """
        Execute the tool call based on end-user input and auth token data.
        Input validation, default values injection (rest type tools) and auth token data injection
        are done here.
        """
        logger.info(f"Executing tool, tool_name={tool.name}, tool_arguments={tool_arguments}")
        try:
            tool_arguments = self._preprocess_tool_arguments(tool, tool_arguments)
        except jsonschema.ValidationError as e:
            logger.exception(
                f"Failed to preprocess tool arguments, tool_name={tool.name}, tool_arguments={tool_arguments}, error={e}"  # noqa: E501
            )
            return mcp_types.ErrorData(
                code=mcp_types.INVALID_PARAMS,
                message=f"Invalid tool arguments: {e.message}",
            )
        except Exception as e:
            logger.exception(
                f"Failed to preprocess tool arguments, tool_name={tool.name}, tool_arguments={tool_arguments}, error={e}"  # noqa: E501
            )
            return mcp_types.ErrorData(
                code=mcp_types.INTERNAL_ERROR,
                message="Failed to preprocess tool arguments",
            )

        return self._execute(tool, tool_arguments, auth_token_data)

    def _preprocess_tool_arguments(
        self,
        tool: VirtualMCPTool,
        tool_arguments: dict,
    ) -> dict:
        tool_metadata = VirtualMCPToolMetadata.model_validate(tool.tool_metadata)

        jsonschema.validate(
            instance=tool_arguments,
            # NOTE: filter_visible_properties only applies to tools of REST type.
            # connector type tool schemas should not contain the "visible" field.
            schema=filter_visible_properties(tool.input_schema)
            if tool_metadata.root.type == VirtualMCPToolType.REST
            else tool.input_schema,
        )

        # inject non-visible defaults, note that should pass the original parameters schema not
        # just visible ones.
        # NOTE: only inject defaults for tools of REST type.
        if tool_metadata.root.type == VirtualMCPToolType.REST:
            tool_arguments = inject_required_but_invisible_defaults(
                tool.input_schema, tool_arguments
            )

        # TODO: better way to remove None values? and if it's ok to remove all of them?
        tool_arguments = remove_none_values(tool_arguments)

        return tool_arguments

    @abstractmethod
    def _execute(
        self,
        tool: VirtualMCPTool,
        tool_arguments: dict,
        auth_token_data: VirtualMCPAuthTokenData | None,
    ) -> mcp_types.CallToolResult:
        """
        No error should be raised here.
        i.e., use CallToolResult.isError to indicate the error.
        """
        pass
