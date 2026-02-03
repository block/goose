import importlib
from typing import override

from mcp import types as mcp_types

from aci.common.db.sql_models import VirtualMCPTool
from aci.common.logging_setup import get_logger
from aci.common.schemas.virtual_mcp import VirtualMCPAuthTokenData
from aci.virtual_mcp.executors.base_executor import ToolExecutor
from aci.virtual_mcp.executors.connectors.base import BaseConnector

logger = get_logger(__name__)


def _parse_tool_name(tool_name: str) -> tuple[str, str, str]:
    """
    Parse tool name to get module name, class name and method name.
    e.g. "BRAVE_SEARCH__WEB_SEARCH" ->
    "aci.virtual_mcp.connectors.brave_search", "BraveSearch", "web_search"
    """
    app_name, method_name = tool_name.split("__", 1)
    module_name = f"aci.virtual_mcp.executors.connectors.{app_name.lower()}"
    class_name = "".join(word.capitalize() for word in app_name.split("_"))
    method_name = method_name.lower()

    return module_name, class_name, method_name


class ConnectorToolExecutor(ToolExecutor):
    """
    Tool executor for connector-based tools.
    """

    @override
    def _execute(
        self,
        tool: VirtualMCPTool,
        tool_arguments: dict,
        auth_token_data: VirtualMCPAuthTokenData | None,
    ) -> mcp_types.CallToolResult:
        """
        Execute a tool by importing the connector module and calling the method.
        """
        module_name, class_name, method_name = _parse_tool_name(tool.name)
        logger.debug(
            f"executing connector based tool, tool_name={tool.name}, module_name={module_name}, "
            f"class_name={class_name}, method_name={method_name}"
        )

        try:
            connector_class: type[BaseConnector] = getattr(
                importlib.import_module(module_name), class_name
            )
        except (ImportError, AttributeError) as e:
            logger.exception(
                f"Failed to find connector class, tool_name={tool.name}, module_name={module_name},"
                f" class_name={class_name}, error={e}"
            )
            return mcp_types.CallToolResult(
                isError=True,
                content=[
                    mcp_types.TextContent(
                        type="text", text=f"Connector class not found for tool {tool.name}"
                    )
                ],
            )
        # TODO: caching? singleton per virtual mcp server per enduser account? executing in a
        # thread pool?
        # another tricky thing is the access token expiration if using long-live cached objects
        connector_instance = connector_class(auth_token_data)

        return connector_instance.execute(method_name, tool_arguments)
