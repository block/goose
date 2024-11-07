from unittest.mock import MagicMock
import pytest
from goose.toolkit.memory import Memory


@pytest.fixture
def memory_toolkit(tmp_path):
    """Create a memory toolkit instance with temporary directories"""
    mock_notifier = MagicMock()
    toolkit = Memory(notifier=mock_notifier)
    # Override memory directories for testing
    toolkit.local_memory_dir = tmp_path / ".goose/memory"
    toolkit.global_memory_dir = tmp_path / ".config/goose/memory"
    toolkit._ensure_memory_dirs()
    return toolkit


def test_remember_global(memory_toolkit):
    """Test storing a memory in global scope"""
    result = memory_toolkit.remember("Test memory", "test_category", tags="tag1 tag2", scope="global")
    assert "test_category" in result
    assert "tag1" in result
    assert "tag2" in result
    assert "global" in result

    # Verify file content
    memory_file = memory_toolkit.global_memory_dir / "test_category.txt"
    content = memory_file.read_text()
    assert "#tag1 tag2" in content
    assert "Test memory" in content


def test_remember_local(memory_toolkit):
    """Test storing a memory in local scope"""
    result = memory_toolkit.remember("Local test", "local_category", tags="local", scope="local")
    assert "local_category" in result
    assert "local" in result

    # Verify file content
    memory_file = memory_toolkit.local_memory_dir / "local_category.txt"
    content = memory_file.read_text()
    assert "#local" in content
    assert "Local test" in content


def test_search_by_text(memory_toolkit):
    """Test searching memories by text"""
    memory_toolkit.remember("Test memory one", "category1", scope="global")
    memory_toolkit.remember("Test memory two", "category2", scope="global")

    result = memory_toolkit.search("memory")
    assert "Test memory one" in result
    assert "Test memory two" in result


def test_search_by_tag(memory_toolkit):
    """Test searching memories by tag"""
    memory_toolkit.remember("Tagged memory", "tagged", tags="findme test", scope="global")
    memory_toolkit.remember("Another tagged", "tagged", tags="findme other", scope="global")

    result = memory_toolkit.search("findme")
    assert "Tagged memory" in result
    assert "Another tagged" in result


def test_search_specific_category(memory_toolkit):
    """Test searching in a specific category"""
    memory_toolkit.remember("Memory in cat1", "cat1", scope="global")
    memory_toolkit.remember("Memory in cat2", "cat2", scope="global")

    result = memory_toolkit.search("Memory", category="cat1")
    assert "Memory in cat1" in result
    assert "Memory in cat2" not in result


def test_list_categories(memory_toolkit):
    """Test listing memory categories"""
    memory_toolkit.remember("Global memory", "global_cat", scope="global")
    memory_toolkit.remember("Local memory", "local_cat", scope="local")

    result = memory_toolkit.list_categories()
    assert "global_cat" in result
    assert "local_cat" in result

    # Test scope filtering
    global_only = memory_toolkit.list_categories(scope="global")
    assert "global_cat" in global_only
    assert "local_cat" not in global_only


def test_forget_category(memory_toolkit):
    """Test removing a category"""
    memory_toolkit.remember("Memory to forget", "forget_me", scope="global")
    assert (memory_toolkit.global_memory_dir / "forget_me.txt").exists()

    result = memory_toolkit.forget_category("forget_me", scope="global")
    assert "Successfully removed" in result
    assert not (memory_toolkit.global_memory_dir / "forget_me.txt").exists()


def test_invalid_category_name(memory_toolkit):
    """Test that invalid category names are sanitized"""
    result = memory_toolkit.remember("Test memory", "test/category!", tags="tag", scope="global")
    assert "test_category_" in result

    # Verify file was created with sanitized name
    files = list(memory_toolkit.global_memory_dir.glob("*.txt"))
    assert len(files) == 1
    assert "test_category_" in files[0].name


def test_system_prompt_includes_memories(memory_toolkit):
    """Test that the system prompt includes existing memories"""
    # Add some test memories
    memory_toolkit.remember("Global test memory", "global_cat", tags="tag1 tag2", scope="global")
    memory_toolkit.remember("Local test memory", "local_cat", tags="tag3", scope="local")

    system_prompt = memory_toolkit.system()

    # Check that the base prompt is included
    assert "I have access to a memory system" in system_prompt

    # Check that memories are included
    assert "Global memories:" in system_prompt
    assert "Category: global_cat" in system_prompt
    assert "Global test memory" in system_prompt
    assert "[tags: tag1 tag2]" in system_prompt

    assert "Local memories:" in system_prompt
    assert "Category: local_cat" in system_prompt
    assert "Local test memory" in system_prompt
    assert "[tags: tag3]" in system_prompt


def test_system_prompt_empty_memories(memory_toolkit):
    """Test that the system prompt handles no existing memories gracefully"""
    system_prompt = memory_toolkit.system()

    # Check that the base prompt is included
    assert "I have access to a memory system" in system_prompt

    # Check that empty memory state is handled
    assert "No existing memories found" in system_prompt
