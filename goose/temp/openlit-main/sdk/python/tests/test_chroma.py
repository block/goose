# pylint: disable=duplicate-code, no-name-in-module, assignment-from-no-return
"""
This module contains tests for ChromaDB functionality using the ChromaDB Python library.

Tests cover various API endpoints, including create_collection, add, query,
upsert, update, get, peek and delete.
These tests validate integration with OpenLIT.

Note: Ensure the environment is properly configured for ChromaDB access and OpenLIT monitoring
prior to running these tests.
"""

import chromadb
import openlit

# Initialize ChromaDB client
chroma_client = chromadb.Client()

# Initialize environment and application name for OpenLIT monitoring
openlit.init(environment="openlit-testing", application_name="openlit-python-test")


def test_db_chroma():
    """
    Tests basic operations within a ChromaDB collection.

    This includes creating a new collection, adding documents to the collection,
    querying the collection, and deleting documents from the collection. The test
    verifies correct behavior of these operations by asserting expected outcomes at
    each step.

    - A new collection named "openlit" is created and verified for correct naming.
    - Documents are added to the collection with specific metadata and ids,
      and the absence of an error response is verified.
    - A query operation is performed, and its results are verified to match the
      expected document ids.
    - A delete operation targets a specific document by id, and successful deletion
      is implicitly verified by the absence of an error response.

    The test ensures the basic CRUD operations perform as expected in ChromaDB.
    Raises:
      AssertionError: If the responses from ChromaDB operations do not meet the expected outcomes.
    """

    # Create a new collection named "openlit"
    collection = chroma_client.create_collection(name="openlit")
    assert collection.name == "openlit"

    # Add documents to the collection
    db_add = collection.add(
        documents=["This is a document", "This is another document"],
        metadatas=[{"source": "my_source"}, {"source": "my_source"}],
        ids=["id1", "id2"],
    )
    assert db_add is None

    # Query the documents in the collection
    db_query = collection.query(query_texts=["This is a query document"], n_results=2)
    assert db_query["ids"] == [["id1", "id2"]]

    # Delete a document from the collection
    db_delete = collection.delete(ids=["id2"], where={"source": "my_source"})
    assert db_delete is None
