import hashlib

import pytest

from aci.common.exceptions import MCPToolSanitizationError
from aci.common.mcp_tool_utils import (
    compare_tool_fields,
    diff_tools,
    normalize_and_hash_content,
    sanitize_canonical_name,
)
from aci.common.schemas.mcp_tool import MCPToolMetadata, MCPToolUpsert


class TestNormalizeAndHashContent:
    @pytest.mark.parametrize(
        "content,expected_normalized",
        [
            # String normalization tests
            ("Hello World", "helloworld"),  # Basic normalization
            ("HELLO WORLD", "helloworld"),  # Same normalized (case insensitive)
            ("Hello, World!", "helloworld"),  # Same normalized (punctuation removed)
            ("  Hello   World  ", "helloworld"),  # Same normalized (whitespace removed)
            ("Hello123World", "hello123world"),  # Different content with numbers
            ("", ""),  # Empty string
            ("123", "123"),  # Numbers only
            ("!@#$%", ""),  # Punctuation only (becomes empty)
            # Dict normalization tests
            ({"key": "value"}, '{"key":"value"}'),
            ({"b": 2, "a": 1}, '{"a":1,"b":2}'),  # Keys sorted
            ({"nested": {"z": 3, "a": 1}}, '{"nested":{"a":1,"z":3}}'),  # Nested sorting
            ({}, "{}"),  # Empty dict
            ({"key": None}, '{"key":null}'),  # None values
            ({"key": [3, 1, 2]}, '{"key":[3,1,2]}'),  # Arrays not sorted
        ],
    )
    def test_normalize_string_content(self, content: str | dict, expected_normalized: str) -> None:
        """Test string content normalization and hashing."""
        result = normalize_and_hash_content(content)

        expected_hash = hashlib.sha256(expected_normalized.encode("utf-8")).hexdigest()

        assert result == expected_hash

    def test_dict_content_order_independence(self) -> None:
        """Test that dict key order doesn't affect the hash."""
        dict1 = {"a": 1, "b": 2, "c": 3}
        dict2 = {"c": 3, "a": 1, "b": 2}

        hash1 = normalize_and_hash_content(dict1)
        hash2 = normalize_and_hash_content(dict2)
        assert hash1 == hash2

    def test_complex_nested_dict(self) -> None:
        """Test complex nested dictionary normalization."""
        complex_dict = {
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer", "minimum": 0},
            },
            "required": ["name"],
        }

        # Same dict with different key order
        reordered_dict = {
            "properties": {
                "age": {"minimum": 0, "type": "integer"},
                "name": {"type": "string"},
            },
            "type": "object",
            "required": ["name"],
        }

        hash1 = normalize_and_hash_content(complex_dict)
        hash2 = normalize_and_hash_content(reordered_dict)
        assert hash1 == hash2


class TestSanitizeCanonicalName:
    """Test the sanitize_canonical_name function."""

    @pytest.mark.parametrize(
        "input_name,expected_output",
        [
            # Basic sanitization
            ("hello_world", "HELLO_WORLD"),
            ("Hello-World", "HELLO_WORLD"),
            ("hello world", "HELLO_WORLD"),
            ("hello.world", "HELLO_WORLD"),
            ("hello@world", "HELLO_WORLD"),
            # Multiple special characters
            ("hello---world", "HELLO_WORLD"),
            ("hello___world", "HELLO_WORLD"),
            ("hello...world", "HELLO_WORLD"),
            # Leading/trailing underscores
            ("_hello_world_", "HELLO_WORLD"),
            ("__hello__world__", "HELLO_WORLD"),
            # Numbers and mixed case
            ("Hello123World", "HELLO123WORLD"),
            ("test_tool_v2", "TEST_TOOL_V2"),
            # Already valid names
            ("HELLO_WORLD", "HELLO_WORLD"),
            ("TEST123", "TEST123"),
            # Edge cases
            ("a", "A"),
            ("1", "1"),
            ("héllo_wörld", "H_LLO_W_RLD"),
            # Invalid names
            ("_", ""),  # This will raise an error
            ("", ""),  # Empty string
            ("   ", ""),  # Only whitespace
            ("___", ""),  # Only underscores
            ("!@#$%", ""),  # Only special characters
            ("---", ""),  # Only dashes
            ("...", ""),  # Only dots)
        ],
    )
    def test_valid_sanitization(self, input_name: str, expected_output: str) -> None:
        """Test valid tool name sanitization."""
        if expected_output == "":
            # Test that empty result raises an error
            with pytest.raises(MCPToolSanitizationError, match="empty after sanitization"):
                sanitize_canonical_name(input_name)
        else:
            result = sanitize_canonical_name(input_name)
            assert result == expected_output


class TestDiffTools:
    """Test the diff_tools function."""

    def create_test_tool(
        self,
        name: str,
        description: str = "Test description",
        input_schema: dict | None = None,
        tags: list[str] | None = None,
        canonical_name: str | None = None,
        desc_hash: str = "desc_hash",
        schema_hash: str = "schema_hash",
    ) -> MCPToolUpsert:
        """Helper to create a test MCPToolUpsert."""
        if input_schema is None:
            input_schema = {"type": "object", "properties": {}}
        if tags is None:
            tags = ["test"]
        if canonical_name is None:
            canonical_name = name.upper()

        return MCPToolUpsert(
            name=name,
            description=description,
            input_schema=input_schema,
            tags=tags,
            tool_metadata=MCPToolMetadata(
                canonical_tool_name=canonical_name,
                canonical_tool_description_hash=desc_hash,
                canonical_tool_input_schema_hash=schema_hash,
            ),
        )

    def test_empty_lists(self) -> None:
        """Test diff_tools with empty lists."""
        result = diff_tools([], [])
        new, deleted, embedding_updated, non_embedding_updated, unchanged = result

        assert new == []
        assert deleted == []
        assert embedding_updated == []
        assert non_embedding_updated == []
        assert unchanged == []

    def test_new_tools_only(self) -> None:
        """Test diff_tools with only new tools."""
        new_tools = [
            self.create_test_tool("TOOL__A"),
            self.create_test_tool("TOOL__B"),
        ]

        result = diff_tools([], new_tools)
        new, deleted, embedding_updated, non_embedding_updated, unchanged = result

        assert len(new) == 2
        assert new[0].name == "TOOL__A"
        assert new[1].name == "TOOL__B"
        assert deleted == []
        assert embedding_updated == []
        assert non_embedding_updated == []
        assert unchanged == []

    def test_deleted_tools_only(self) -> None:
        """Test diff_tools with only deleted tools."""
        old_tools = [
            self.create_test_tool("TOOL__A"),
            self.create_test_tool("TOOL__B"),
        ]

        result = diff_tools(old_tools, [])
        new, deleted, embedding_updated, non_embedding_updated, unchanged = result

        assert new == []
        assert len(deleted) == 2
        assert deleted[0].name == "TOOL__A"
        assert deleted[1].name == "TOOL__B"
        assert embedding_updated == []
        assert non_embedding_updated == []
        assert unchanged == []

    def test_unchanged_tools(self) -> None:
        """Test diff_tools with unchanged tools."""
        tool_a = self.create_test_tool("TOOL__A")
        tool_b = self.create_test_tool("TOOL__B")
        old_tools = [tool_a, tool_b]

        # Create identical new tools
        new_tool_a = self.create_test_tool("TOOL__A")
        new_tool_b = self.create_test_tool("TOOL__B")
        new_tools = [new_tool_a, new_tool_b]

        result = diff_tools(old_tools, new_tools)
        new, deleted, embedding_updated, non_embedding_updated, unchanged = result

        assert new == []
        assert deleted == []
        assert embedding_updated == []
        assert non_embedding_updated == []
        assert len(unchanged) == 2

    def test_embedding_fields_changed(self) -> None:
        """Test diff_tools with embedding fields changed."""
        old_tool = self.create_test_tool("TOOL__A", desc_hash="old_hash")
        new_tool = self.create_test_tool("TOOL__A", desc_hash="new_hash")

        result = diff_tools([old_tool], [new_tool])
        new, deleted, embedding_updated, non_embedding_updated, unchanged = result

        assert new == []
        assert deleted == []
        assert len(embedding_updated) == 1
        assert embedding_updated[0].name == "TOOL__A"
        assert non_embedding_updated == []
        assert unchanged == []

    def test_non_embedding_fields_changed(self) -> None:
        """Test diff_tools with non-embedding fields changed."""
        old_tool = self.create_test_tool("TOOL__A", tags=["old_tag"])
        new_tool = self.create_test_tool("TOOL__A", tags=["new_tag"])

        result = diff_tools([old_tool], [new_tool])
        new, deleted, embedding_updated, non_embedding_updated, unchanged = result

        assert new == []
        assert deleted == []
        assert embedding_updated == []
        assert len(non_embedding_updated) == 1
        assert non_embedding_updated[0].name == "TOOL__A"
        assert unchanged == []

    @pytest.mark.parametrize(
        "old_canonical_name,new_canonical_name,old_desc_hash,new_desc_hash,old_schema_hash,new_schema_hash,expected_category",
        [
            # Canonical name changed
            ("OLD_NAME", "NEW_NAME", "hash1", "hash1", "hash2", "hash2", "embedding"),
            # Description hash changed
            ("NAME", "NAME", "old_hash", "new_hash", "hash2", "hash2", "embedding"),
            # Schema hash changed
            ("NAME", "NAME", "hash1", "hash1", "old_hash", "new_hash", "embedding"),
            # No embedding fields changed
            ("NAME", "NAME", "hash1", "hash1", "hash2", "hash2", "non_embedding"),
        ],
    )
    def test_embedding_vs_non_embedding_changes(
        self,
        old_canonical_name: str,
        new_canonical_name: str,
        old_desc_hash: str,
        new_desc_hash: str,
        old_schema_hash: str,
        new_schema_hash: str,
        expected_category: str,
    ) -> None:
        """Test that changes are correctly categorized as embedding vs non-embedding."""
        old_tool = self.create_test_tool(
            "TOOL__A",
            canonical_name=old_canonical_name,
            desc_hash=old_desc_hash,
            schema_hash=old_schema_hash,
            tags=["old_tag"],
        )
        new_tool = self.create_test_tool(
            "TOOL__A",
            canonical_name=new_canonical_name,
            desc_hash=new_desc_hash,
            schema_hash=new_schema_hash,
            tags=["new_tag"] if expected_category == "non_embedding" else ["old_tag"],
        )

        result = diff_tools([old_tool], [new_tool])
        new, deleted, embedding_updated, non_embedding_updated, unchanged = result

        if expected_category == "embedding":
            assert len(embedding_updated) == 1
            assert len(non_embedding_updated) == 0
        else:
            assert len(embedding_updated) == 0
            assert len(non_embedding_updated) == 1

        assert len(unchanged) == 0

    def test_complex_diff_scenario(self) -> None:
        """Test a complex scenario with all types of changes."""
        # Old tools
        tool_unchanged = self.create_test_tool("TOOL__UNCHANGED")
        tool_to_delete = self.create_test_tool("TOOL__DELETE")
        tool_embedding_change = self.create_test_tool("TOOL__EMB_CHANGE", desc_hash="old_hash")
        tool_non_embedding_change = self.create_test_tool("TOOL__NON_EMB_CHANGE", tags=["old"])

        old_tools = [
            tool_unchanged,
            tool_to_delete,
            tool_embedding_change,
            tool_non_embedding_change,
        ]

        # New tools
        new_tool_unchanged = self.create_test_tool("TOOL__UNCHANGED")
        new_tool_create = self.create_test_tool("TOOL__NEW")
        new_tool_embedding_change = self.create_test_tool("TOOL__EMB_CHANGE", desc_hash="new_hash")
        new_tool_non_embedding_change = self.create_test_tool("TOOL__NON_EMB_CHANGE", tags=["new"])

        new_tools = [
            new_tool_unchanged,
            new_tool_create,
            new_tool_embedding_change,
            new_tool_non_embedding_change,
        ]

        result = diff_tools(old_tools, new_tools)
        new, deleted, embedding_updated, non_embedding_updated, unchanged = result

        assert len(new) == 1
        assert new[0].name == "TOOL__NEW"

        assert len(deleted) == 1
        assert deleted[0].name == "TOOL__DELETE"

        assert len(embedding_updated) == 1
        assert embedding_updated[0].name == "TOOL__EMB_CHANGE"

        assert len(non_embedding_updated) == 1
        assert non_embedding_updated[0].name == "TOOL__NON_EMB_CHANGE"

        assert len(unchanged) == 1
        assert unchanged[0].name == "TOOL__UNCHANGED"


class TestCompareToolFields:
    """Test the compare_tool_fields function."""

    def create_test_tool(
        self,
        name: str = "TEST__TOOL",
        description: str = "Test description",
        input_schema: dict | None = None,
        tags: list[str] | None = None,
        tool_metadata: MCPToolMetadata | None = None,
    ) -> MCPToolUpsert:
        """Helper to create a test MCPToolUpsert with default values."""
        if input_schema is None:
            input_schema = {"type": "object"}
        if tags is None:
            tags = ["test"]
        if tool_metadata is None:
            tool_metadata = MCPToolMetadata(
                canonical_tool_name="TEST_TOOL",
                canonical_tool_description_hash="desc_hash",
                canonical_tool_input_schema_hash="schema_hash",
            )

        return MCPToolUpsert(
            name=name,
            description=description,
            input_schema=input_schema,
            tags=tags,
            tool_metadata=tool_metadata,
        )

    def test_no_changes(self) -> None:
        """Test when no non-embedding fields have changed."""
        tool1 = self.create_test_tool()
        tool2 = self.create_test_tool()

        fields_changed, embedding_fields_changed = compare_tool_fields(tool1, tool2)
        assert not fields_changed
        assert not embedding_fields_changed

    def test_tags_changed(self) -> None:
        """Test when tags field has changed."""
        tool1 = self.create_test_tool(tags=["tag1", "tag2"])
        tool2 = self.create_test_tool(tags=["tag1", "tag3"])

        fields_changed, embedding_fields_changed = compare_tool_fields(tool1, tool2)
        assert fields_changed
        assert not embedding_fields_changed

    def test_embedding_fields_ignored(self) -> None:
        """Test that changes to embedding fields don't affect this function."""
        tool1 = self.create_test_tool(
            name="TOOL1__TEST",
            description="Description 1",
            input_schema={"type": "string"},
        )
        tool2 = self.create_test_tool(
            name="TOOL2__TEST",
            description="Description 2",
            input_schema={"type": "number"},
        )

        # Even though name, description, and input_schema changed,
        # the hashes in metadata did not change, so should return False
        fields_changed, embedding_fields_changed = compare_tool_fields(tool1, tool2)
        assert not fields_changed
        assert not embedding_fields_changed

    def test_tool_metadata_changes_ignored(self) -> None:
        """Test that tool_metadata changes don't affect this function."""
        metadata1 = MCPToolMetadata(
            canonical_tool_name="NAME1",
            canonical_tool_description_hash="hash1",
            canonical_tool_input_schema_hash="schema1",
        )
        metadata2 = MCPToolMetadata(
            canonical_tool_name="NAME2",
            canonical_tool_description_hash="hash2",
            canonical_tool_input_schema_hash="schema2",
        )

        tool1 = self.create_test_tool(tool_metadata=metadata1)
        tool2 = self.create_test_tool(tool_metadata=metadata2)

        # the hashes in metadata changed, so should return True
        fields_changed, embedding_fields_changed = compare_tool_fields(tool1, tool2)
        assert fields_changed
        assert embedding_fields_changed

    def test_canonical_name_case_sensitivity(self) -> None:
        """Test that canonical tool name comparison is case sensitive."""
        tool1 = self.create_test_tool(
            tool_metadata=MCPToolMetadata(
                canonical_tool_name="test_tool",
                canonical_tool_description_hash="desc_hash",
                canonical_tool_input_schema_hash="schema_hash",
            )
        )
        tool2 = self.create_test_tool(
            tool_metadata=MCPToolMetadata(
                canonical_tool_name="TEST_TOOL",
                canonical_tool_description_hash="desc_hash",
                canonical_tool_input_schema_hash="schema_hash",
            )
        )

        fields_changed, embedding_fields_changed = compare_tool_fields(tool1, tool2)
        assert fields_changed
        assert embedding_fields_changed
