import copy
from typing import Any

from aci.common.db.sql_models import VirtualMCPTool
from aci.common.enums import MCPToolSchemaFormat
from aci.common.logging_setup import get_logger

logger = get_logger(__name__)


# TODO: test processor
def filter_visible_properties(parameters_schema: dict) -> dict:
    """
    Filter the schema to include only visible properties and remove the 'visible' field itself.
    Ideally, visible and required should be defined for type "object", but we don't make that
    assumption here.
    """

    # create a separate function to avoid deep copying the schema at each recursive call
    def filter(schema: dict) -> dict:
        # if the schema is not an object return the schema as is
        if schema.get("type") != "object":
            return schema

        visible: list[str] | None = schema.pop(
            "visible", None
        )  # use pop to remove visible field itself
        properties: dict | None = schema.get("properties")
        required: list[str] | None = schema.get("required")

        # NOTE: if visible is not present, assume all properties are visible
        if visible is None:
            visible = list(schema.get("properties", {}).keys())

        # only continue if properties are defined
        if properties is not None:
            # Filter properties to include only visible properties
            filtered_properties = {
                key: value for key, value in properties.items() if key in visible
            }

            # if required is defined, update the required list to include only visible properties
            if required is not None:
                schema["required"] = [key for key in required if key in visible]

            # Recursively filter nested properties
            for key, value in filtered_properties.items():
                filtered_properties[key] = filter(value)

            # Update the schema with filtered properties
            schema["properties"] = filtered_properties

        return schema

    # Create a deep copy of the schema once
    filtered_parameters_schema = copy.deepcopy(parameters_schema)
    return filter(filtered_parameters_schema)


def inject_required_but_invisible_defaults(parameters_schema: dict, input_data: dict) -> dict:
    """
    Recursively injects required but invisible properties with their default values
    into the input data.
    """
    for prop, subschema in parameters_schema.get("properties", {}).items():
        # check if the property is not set by user and is required but invisible
        if (
            prop not in input_data
            and prop in parameters_schema.get("required", [])
            and prop not in parameters_schema.get("visible", [])
        ):
            # check if it has a default value, which should exist for non-object types
            if "default" in subschema:
                input_data[prop] = subschema["default"]
            else:
                # If no default value, but it's an object, initialize it as an empty dict
                if subschema.get("type") == "object":
                    input_data[prop] = {}
                else:
                    raise Exception(
                        f"No default value found for property: {prop}, type: {subschema.get('type')}"  # noqa: E501
                    )
        # Recursively inject defaults for nested objects
        if isinstance(input_data.get(prop), dict):
            inject_required_but_invisible_defaults(subschema, input_data[prop])

    return input_data


def remove_none_values(data: Any) -> Any:
    if isinstance(data, dict):
        return {k: remove_none_values(v) for k, v in data.items() if v is not None}
    elif isinstance(data, list):
        return [remove_none_values(item) for item in data if item is not None]
    else:
        return data


def format_tool_schema(
    tool: VirtualMCPTool, format: MCPToolSchemaFormat = MCPToolSchemaFormat.ANTHROPIC
) -> dict:
    # Strip the server name prefix from tool name, e.g., "GMAIL__SEND_EMAIL" -> "SEND_EMAIL"
    tool_name = tool.name.split("__")[1]
    input_schema = filter_visible_properties(tool.input_schema)

    match format:
        case MCPToolSchemaFormat.OPENAI:
            return {
                "type": "function",
                "function": {
                    "name": tool_name,
                    "description": tool.description,
                    "parameters": input_schema,
                },
            }

        case MCPToolSchemaFormat.OPENAI_RESPONSES:
            return {
                "type": "function",
                "name": tool_name,
                "description": tool.description,
                "parameters": input_schema,
            }

        case MCPToolSchemaFormat.ANTHROPIC:
            return {
                "name": tool_name,
                "description": tool.description,
                "inputSchema": input_schema,
            }
