import hashlib
import json
import re

from aci.common.exceptions import MCPToolSanitizationError
from aci.common.logging_setup import get_logger
from aci.common.schemas.mcp_tool import MCPToolUpsert

logger = get_logger(__name__)


def normalize_and_hash_content(content: str | dict) -> str:
    """
    Normalize content and generate a hash to detect meaningful changes while ignoring formatting.

    For strings: keeps only letters and numbers (removes punctuation, whitespace, etc.)
    For objects: converts to normalized JSON with sorted keys
    """
    if isinstance(content, str):
        # Normalize string content:
        # 1. Convert to lowercase for case-insensitive comparison
        # 2. Keep only letters and numbers (remove all punctuation, whitespace, etc.)
        normalized = re.sub(r"[^a-z0-9]", "", content.lower())
    else:
        # For objects (like inputSchema), convert to normalized JSON
        # Sort keys to ensure consistent ordering
        normalized = json.dumps(content, sort_keys=True, separators=(",", ":"))

    # Generate SHA-256 hash of normalized content
    # Note: switch to md5 if this causes performance issues in the future
    return hashlib.sha256(normalized.encode("utf-8")).hexdigest()


def sanitize_canonical_name(canonical_name: str) -> str:
    """
    Convert MCP tool name / MCP server name to comply with naming rules: uppercase letters, numbers,
    underscores only, no consecutive underscores
    """
    # Convert to uppercase
    sanitized = canonical_name.upper()

    # Replace any non-alphanumeric characters (except underscores) with underscores
    sanitized = re.sub(r"[^A-Z0-9_]", "_", sanitized)

    # Remove consecutive underscores by replacing multiple underscores with single underscore
    sanitized = re.sub(r"_+", "_", sanitized)

    # Remove leading and trailing underscores
    sanitized = sanitized.strip("_")

    if not sanitized:
        raise MCPToolSanitizationError(f"Tool name '{canonical_name}' is empty after sanitization.")

    return sanitized


def diff_tools(
    old_tools: list[MCPToolUpsert],
    new_tools: list[MCPToolUpsert],
) -> tuple[
    list[MCPToolUpsert],
    list[MCPToolUpsert],
    list[MCPToolUpsert],
    list[MCPToolUpsert],
    list[MCPToolUpsert],
]:
    """
    Diff the two given lists of MCP tools.
    Note:
    - Tools with same `tool.name` will be treated as a same tool.
    - Tools renamed will be treated as a removal of the old tool and a creationg of a new tool, even
      if the description and input schema are the same.
    - For performance, use the hashes to check if description and input schema has changed.

    Returns a tuple of:
        - A list of tools that is new and should be created.
        - A list of tools that is old and should be deleted.
        - A list of tools that has changed in the embedding fields and should be updated.
        - A list of tools that has changed in the non-embedding fields and should be updated.
        - A list of tools that is unchanged.
    """
    new_tools_to_create = []
    old_tools_to_delete = []
    tools_to_update_with_re_embedding = []
    tools_to_update_without_re_embedding = []
    tools_unchanged = []

    # Create dict for lookup by tool name
    # Tools with same `tool.name` will be treated as a same tool. Also, assume the `tool.name` is
    # unique in each list.
    old_tools_dict = {tool.name: tool for tool in old_tools}
    new_tools_dict = {tool.name: tool for tool in new_tools}

    for new_tool_name, new_tool in new_tools_dict.items():
        # Tool not found in old_tools_dict, should be created
        if new_tool_name not in old_tools_dict:
            new_tools_to_create.append(new_tool)
        else:
            # Tool exists in both lists, check if any of the fields has changed
            old_tool = old_tools_dict[new_tool_name]
            fields_changed, embedding_fields_changed = compare_tool_fields(old_tool, new_tool)
            if embedding_fields_changed:
                tools_to_update_with_re_embedding.append(new_tool)
            elif fields_changed:
                tools_to_update_without_re_embedding.append(new_tool)
            else:
                tools_unchanged.append(new_tool)

    # Find tools that should be deleted (in old_tools but not in new_tools)
    for tool_name, old_tool in old_tools_dict.items():
        if tool_name not in new_tools_dict:
            old_tools_to_delete.append(old_tool)

    return (
        new_tools_to_create,
        old_tools_to_delete,
        tools_to_update_with_re_embedding,
        tools_to_update_without_re_embedding,
        tools_unchanged,
    )


def compare_tool_fields(old_tool: MCPToolUpsert, new_tool: MCPToolUpsert) -> tuple[bool, bool]:
    """
    Return whether any of the fields has been changed, and whether any of the changes involves the
    fields that has been used for embedding.

    Note: For large fields like description / input schema, we compare using the hashes if available

    Returns:
        - A tuple of:
            - Whether any of the fields has been changed.
            - Whether any of the changes involves the fields that has been used for embedding.
    """

    non_embedding_fields = set(MCPToolUpsert.model_fields.keys())
    # Obtain non-embedding fields by removing the following fields from all model fields
    non_embedding_fields.difference_update({"name", "description", "input_schema", "tool_metadata"})

    non_embedding_fields_changed = old_tool.model_dump(
        include=non_embedding_fields
    ) != new_tool.model_dump(include=non_embedding_fields)

    # Embedding fields includes: canonical_tool_name, description, input_schema
    if old_tool.tool_metadata.canonical_tool_name != new_tool.tool_metadata.canonical_tool_name:
        embedding_fields_changed = True
    elif (
        old_tool.tool_metadata.canonical_tool_description_hash
        != new_tool.tool_metadata.canonical_tool_description_hash
    ):
        embedding_fields_changed = True
    elif (
        old_tool.tool_metadata.canonical_tool_input_schema_hash
        != new_tool.tool_metadata.canonical_tool_input_schema_hash
    ):
        embedding_fields_changed = True
    else:
        embedding_fields_changed = False

    return non_embedding_fields_changed or embedding_fields_changed, embedding_fields_changed
