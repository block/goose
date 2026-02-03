from aci.common.db.sql_models import VirtualMCPTool
from aci.common.enums import VirtualMCPToolType
from aci.common.schemas.virtual_mcp import VirtualMCPToolMetadata
from aci.virtual_mcp.executors.base_executor import ToolExecutor
from aci.virtual_mcp.executors.connector_tool_executor import ConnectorToolExecutor
from aci.virtual_mcp.executors.rest_tool_executor import RestFunctionExecutor


def get_tool_executor(tool: VirtualMCPTool) -> ToolExecutor:
    tool_type = VirtualMCPToolMetadata.model_validate(tool.tool_metadata).root.type
    match tool_type:
        case VirtualMCPToolType.REST:
            return RestFunctionExecutor()
        case VirtualMCPToolType.CONNECTOR:
            return ConnectorToolExecutor()
