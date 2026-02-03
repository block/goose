import json

from e2b_code_interpreter import Sandbox
from mcp import types as mcp_types

from aci.common.logging_setup import get_logger
from aci.common.schemas.virtual_mcp import VirtualMCPAuthTokenData
from aci.virtual_mcp.executors.connectors.base import BaseConnector

logger = get_logger(__name__)


class E2b(BaseConnector):
    """
    E2B.dev Sandbox Connector using Code Interpreter.
    """

    def __init__(
        self,
        auth_token_data: VirtualMCPAuthTokenData,
    ):
        super().__init__(auth_token_data)
        self.api_key = auth_token_data.token

    def run_code(
        self,
        code: str,
    ) -> mcp_types.CallToolResult:
        """
        Execute code in E2B sandbox and return the result.
        """
        with Sandbox(api_key=self.api_key) as sandbox:
            execution = sandbox.run_code(code)

            result = {"text": execution.text}

            return mcp_types.CallToolResult(
                structuredContent=result,
                content=[mcp_types.TextContent(type="text", text=json.dumps(result))],
            )
