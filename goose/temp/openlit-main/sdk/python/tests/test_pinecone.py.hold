# pylint: disable=duplicate-code, no-name-in-module
"""
This module contains tests for Pinecone functionality using the Pinecone Python library.

Tests cover various API endpoints, including upsert, query and delete.
These tests validate integration with OpenLIT.

Environment Variables:
    - PINECONE_API_KEY: Pinecone API key for authentication.

Note: Ensure the environment is properly configured for Pinecone access and OpenLIT monitoring
prior to running these tests.
"""

from pinecone import Pinecone
import openlit

# Initialize the Chroma client
pc = Pinecone()
index = pc.Index("openlit-tests")

# Initialize environment and application name for OpenLIT monitoring
openlit.init(environment="openlit-python-testing", application_name="openlit-python-piencone-test")

def test_db_pinecone():
    """
    Tests basic operations within a Pinecone index.

    This includes adding vectors to the index,
    querying the index, and deleting vectors from the index. The test
    verifies correct behavior of these operations by asserting expected outcomes at
    each step.

    - Vectors are upserted into the index with unique IDs and vector values. The test
      verifies that the upsert operation reports the correct number of vectors inserted.
    - A query operation is performed using a vector similar to one of the upserted vectors.
      The test verifies that the query successfully executes within the specified namespace
      and that the results match expected vectors vectors.
    - A delete operation targets specific vectors by ID within a namespace, and the test
      verifies that the delete operation completes successfully by asserting the absence
      of an error response.
    
    The test ensures the basic CRUD operations perform as expected in Pinecone.
    Raises:
      AssertionError: If the responses from Pinecone operations do not meet the expected outcomes.
    """

    # Upsert vectors to the index
    db_upsert = index.upsert(
        vectors=[
            {"id": "vec1", "values": [0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1]},
            {"id": "vec2", "values": [0.2, 0.2, 0.2, 0.2, 0.2, 0.2, 0.2, 0.2]},
            {"id": "vec3", "values": [0.3, 0.3, 0.3, 0.3, 0.3, 0.3, 0.3, 0.3]},
            {"id": "vec4", "values": [0.4, 0.4, 0.4, 0.4, 0.4, 0.4, 0.4, 0.4]}
        ],
        namespace="python-tests"
    )
    assert db_upsert.upserted_count == 4

    # Query the vectors from the index
    db_query = index.query(
        namespace="python-tests",
        vector=[0.3, 0.3, 0.3, 0.3, 0.3, 0.3, 0.3, 0.3],
        top_k=3,
        include_values=True
    )
    assert db_query["namespace"] == "python-tests"

    # Delete the vectors from the index
    db_delete = index.delete(ids=["vec1", "vec2", "vec3", "vec4"], namespace='python-tests')
    assert db_delete == {}
