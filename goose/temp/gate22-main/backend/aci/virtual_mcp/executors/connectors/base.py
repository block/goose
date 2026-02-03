from mcp import types as mcp_types

from aci.common.logging_setup import get_logger
from aci.common.schemas.virtual_mcp import VirtualMCPAuthTokenData

logger = get_logger(__name__)


class BaseConnector:
    """
    Base class for all connectors.
    Each connector is tied to a specific virtual mcp server.
    """

    def __init__(
        self,
        auth_token_data: VirtualMCPAuthTokenData | None,
    ):
        self.auth_token_data = auth_token_data

    def execute(self, method_name: str, tool_arguments: dict) -> mcp_types.CallToolResult:
        """
        Args:
            method_name: The name of the method to execute. It should be the lowercased name of the
                         tool name (without the server name prefix). e.g.:
                         method name: "send_email"
                         tool name: "GMAIL__SEND_EMAIL"
            tool_arguments: The arguments to pass to the method.

        Returns:
            The result (including error case) of the method execution.

        This method is the main entry point for executing a tool.
        """
        method = getattr(self, method_name, None)
        if not method:
            logger.error(
                f"Method not found, method_name={method_name}, class_name={self.__class__.__name__}"
            )
            return mcp_types.CallToolResult(
                isError=True,
                content=[
                    mcp_types.TextContent(type="text", text=f"Method {method_name} not found")
                ],
            )

        try:
            # TODO: all connectors should return mcp_types.CallToolResult
            # any way to enforce this?
            result = method(**tool_arguments)
            if isinstance(result, mcp_types.CallToolResult):
                return result
            else:
                logger.error(
                    f"Method {method_name} returned an unexpected type={type(result)}, "
                    f"class_name={self.__class__.__name__}"
                )
                return mcp_types.CallToolResult(
                    isError=True,
                    content=[
                        mcp_types.TextContent(
                            type="text", text=f"Method {method_name} returned an unexpected type"
                        )
                    ],
                )
        except Exception as e:
            logger.exception(
                f"Error executing method, method_name={method_name}, "
                f"class_name={self.__class__.__name__}, error={e}"
            )
            return mcp_types.CallToolResult(
                isError=True,
                content=[mcp_types.TextContent(type="text", text="Internal connector error")],
            )
