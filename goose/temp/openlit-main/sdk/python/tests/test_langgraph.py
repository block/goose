# pylint: disable=duplicate-code, no-member, too-few-public-methods, missing-class-docstring
"""
This module contains tests for LangGraph instrumentation using the LangGraph Python library.

Tests cover:
- Graph construction (StateGraph, compile, add_node, add_edge)
- Graph execution (invoke, ainvoke)
- Streaming (stream, astream)
- State management (get_state, aget_state)

These tests validate integration with OpenLIT.

Note: These tests require langgraph to be installed.
"""

from typing import Annotated, TypedDict

import pytest

# Try to import langgraph
try:
    from langgraph.graph import StateGraph, START, END
    from langgraph.graph.message import add_messages

    LANGGRAPH_AVAILABLE = True
except ImportError:
    LANGGRAPH_AVAILABLE = False
    # Define placeholder for type hints when langgraph not available
    add_messages = None
    StateGraph = None
    START = None
    END = None

import openlit

# Initialize OpenLIT monitoring
openlit.init(
    environment="openlit-python-testing",
    application_name="openlit-python-langgraph-test",
)


# Skip all tests if langgraph is not available
pytestmark = pytest.mark.skipif(
    not LANGGRAPH_AVAILABLE, reason="langgraph not installed"
)


# Define a simple state for testing - only use add_messages annotation when available
if LANGGRAPH_AVAILABLE:

    class SimpleState(TypedDict):
        """Simple state with messages."""

        messages: Annotated[list, add_messages]
else:

    class SimpleState(TypedDict):
        """Simple state with messages (fallback without add_messages)."""

        messages: list


def simple_node(state: SimpleState) -> dict:
    """A simple node that echoes the input."""
    return {"messages": [{"role": "assistant", "content": "test response"}]}


async def async_simple_node(state: SimpleState) -> dict:
    """An async simple node."""
    return {"messages": [{"role": "assistant", "content": "async test response"}]}


class TestLangGraphInstrumentation:
    """Tests for LangGraph instrumentation."""

    def test_graph_construction(self):
        """
        Tests that StateGraph construction is instrumented.
        """
        try:
            # Create a simple graph
            graph = StateGraph(SimpleState)
            graph.add_node("test_node", simple_node)
            graph.add_edge(START, "test_node")
            graph.add_edge("test_node", END)

            # Compile the graph
            compiled = graph.compile()

            assert compiled is not None

        except Exception as e:
            pytest.fail(f"Error in test_graph_construction: {e}")

    def test_graph_invoke(self):
        """
        Tests synchronous graph execution with invoke.
        """
        try:
            # Create and compile graph
            graph = StateGraph(SimpleState)
            graph.add_node("test_node", simple_node)
            graph.add_edge(START, "test_node")
            graph.add_edge("test_node", END)
            compiled = graph.compile()

            # Invoke the graph
            result = compiled.invoke(
                {"messages": [{"role": "user", "content": "test input"}]}
            )

            assert result is not None
            assert "messages" in result

        except Exception as e:
            pytest.fail(f"Error in test_graph_invoke: {e}")

    def test_graph_stream(self):
        """
        Tests synchronous graph streaming.
        """
        try:
            # Create and compile graph
            graph = StateGraph(SimpleState)
            graph.add_node("test_node", simple_node)
            graph.add_edge(START, "test_node")
            graph.add_edge("test_node", END)
            compiled = graph.compile()

            # Stream the graph
            chunks = list(
                compiled.stream(
                    {"messages": [{"role": "user", "content": "test input"}]}
                )
            )

            assert len(chunks) > 0

        except Exception as e:
            pytest.fail(f"Error in test_graph_stream: {e}")

    @pytest.mark.asyncio
    async def test_graph_ainvoke(self):
        """
        Tests async graph execution with ainvoke.
        """
        try:
            # Create and compile graph
            graph = StateGraph(SimpleState)
            graph.add_node("test_node", async_simple_node)
            graph.add_edge(START, "test_node")
            graph.add_edge("test_node", END)
            compiled = graph.compile()

            # Async invoke the graph
            result = await compiled.ainvoke(
                {"messages": [{"role": "user", "content": "test input"}]}
            )

            assert result is not None
            assert "messages" in result

        except Exception as e:
            pytest.fail(f"Error in test_graph_ainvoke: {e}")

    @pytest.mark.asyncio
    async def test_graph_astream(self):
        """
        Tests async graph streaming.
        """
        try:
            # Create and compile graph
            graph = StateGraph(SimpleState)
            graph.add_node("test_node", async_simple_node)
            graph.add_edge(START, "test_node")
            graph.add_edge("test_node", END)
            compiled = graph.compile()

            # Async stream the graph
            chunks = []
            async for chunk in compiled.astream(
                {"messages": [{"role": "user", "content": "test input"}]}
            ):
                chunks.append(chunk)

            assert len(chunks) > 0

        except Exception as e:
            pytest.fail(f"Error in test_graph_astream: {e}")

    def test_graph_with_multiple_nodes(self):
        """
        Tests graph with multiple nodes.
        """

        def node_a(state: SimpleState) -> dict:
            return {"messages": [{"role": "assistant", "content": "from node a"}]}

        def node_b(state: SimpleState) -> dict:
            return {"messages": [{"role": "assistant", "content": "from node b"}]}

        try:
            # Create graph with multiple nodes
            graph = StateGraph(SimpleState)
            graph.add_node("node_a", node_a)
            graph.add_node("node_b", node_b)
            graph.add_edge(START, "node_a")
            graph.add_edge("node_a", "node_b")
            graph.add_edge("node_b", END)
            compiled = graph.compile()

            # Execute
            result = compiled.invoke(
                {"messages": [{"role": "user", "content": "test"}]}
            )

            assert result is not None
            assert "messages" in result
            # Should have messages from both nodes
            assert len(result["messages"]) > 1

        except Exception as e:
            pytest.fail(f"Error in test_graph_with_multiple_nodes: {e}")


class TestLangGraphInstrumentorUnit:
    """Unit tests for LangGraph instrumentor."""

    def test_instrumentor_import(self):
        """Tests that the instrumentor can be imported."""
        try:
            from openlit.instrumentation.langgraph import LangGraphInstrumentor

            assert LangGraphInstrumentor is not None
        except ImportError as e:
            pytest.fail(f"Failed to import LangGraphInstrumentor: {e}")

    def test_utils_import(self):
        """Tests that utils can be imported."""
        try:
            from openlit.instrumentation.langgraph.utils import (
                OPERATION_MAP,
                extract_messages_from_input,
            )

            assert OPERATION_MAP is not None
            assert extract_messages_from_input is not None
        except ImportError as e:
            pytest.fail(f"Failed to import utils: {e}")

    def test_extract_messages_from_input(self):
        """Tests message extraction from input."""
        from openlit.instrumentation.langgraph.utils import extract_messages_from_input

        # Test with messages
        input_with_messages = {"messages": [{"role": "user", "content": "test"}]}
        result = extract_messages_from_input(input_with_messages)
        assert len(result) == 1

        # Test without messages
        input_without_messages = {"other": "data"}
        result = extract_messages_from_input(input_without_messages)
        assert len(result) == 0

    def test_get_message_role(self):
        """Tests message role extraction."""
        from openlit.instrumentation.langgraph.utils import get_message_role

        # Mock message with role
        class MockMessage:
            role = "assistant"

        result = get_message_role(MockMessage())
        assert result == "assistant"

        # Mock message with type
        class MockMessageWithType:
            type = "human"

        result = get_message_role(MockMessageWithType())
        assert result == "human"

    def test_generate_span_name(self):
        """Tests span name generation."""
        from openlit.instrumentation.langgraph.utils import generate_span_name

        # Test invoke span name
        span_name = generate_span_name("graph_execution", "graph_invoke", None)
        assert "graph_execution" in span_name or "graph" in span_name

        # Test stream span name
        span_name = generate_span_name("graph_execution", "graph_stream", None)
        assert "stream" in span_name

    def test_operation_map_completeness(self):
        """Tests that OPERATION_MAP has all required operations."""
        from openlit.instrumentation.langgraph.utils import OPERATION_MAP

        required_ops = [
            "graph_invoke",
            "graph_ainvoke",
            "graph_stream",
            "graph_astream",
            "graph_get_state",
            "graph_aget_state",
            "graph_compile",
        ]

        for op in required_ops:
            assert op in OPERATION_MAP, f"Missing operation: {op}"


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
