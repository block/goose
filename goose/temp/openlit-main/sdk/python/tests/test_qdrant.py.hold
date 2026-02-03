# pylint: disable=duplicate-code, no-name-in-module
"""
This module contains tests for Qdrant functionality using the Qdrant Python library.

Tests cover various API endpoints, including create_collection, add, query,
upsert, update, get and delete.
These tests validate integration with OpenLIT.

Environment Variables:
    - QDRANT_URL: Qdrant Cloud Instance URL.
    - QDRANT_API_TOKEN: Qdrant API api_key for authentication.

Note: Ensure the environment is properly configured for Qdrant access and OpenLIT monitoring
prior to running these tests.
"""

import os
from qdrant_client import QdrantClient, models
from qdrant_client.models import PointStruct
import openlit

# Initialize Qdrant client
client = QdrantClient(
    url=os.getenv("QDRANT_URL"),
    api_key=os.getenv("QDRANT_API_TOKEN"),
)

# Initialize environment and application name for OpenLIT monitoring
openlit.init(
    environment="openlit-python-testing", application_name="openlit-python-qdrant-test"
)
COLLECTION_NAME = "openlit"


def test_db_qdrant():
    """
    Tests basic operations within a Qdrant collection.

    This includes creating a new collection, adding documents to the collection,
    querying the collection, and deleting documents from the collection. The test
    verifies correct behavior of these operations by asserting expected outcomes at
    each step.

    - A new collection is created and verified for correct naming.
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

    # # Create a new collection
    # collection = client.create_collection(
    #   collection_name=COLLECTION_NAME,
    #   vectors_config=VectorParams(size=4, distance=Distance.DOT),
    # )

    # assert collection is True

    # Upsert to the collection
    upsert = client.upsert(
        collection_name=COLLECTION_NAME,
        wait=True,
        points=[
            PointStruct(
                id=1, vector=[0.05, 0.61, 0.76, 0.74], payload={"city": "Berlin"}
            ),
            PointStruct(
                id=2, vector=[0.19, 0.81, 0.75, 0.11], payload={"city": "London"}
            ),
            PointStruct(
                id=3, vector=[0.36, 0.55, 0.47, 0.94], payload={"city": "Moscow"}
            ),
            PointStruct(
                id=4, vector=[0.18, 0.01, 0.85, 0.80], payload={"city": "New York"}
            ),
            PointStruct(
                id=5, vector=[0.24, 0.18, 0.22, 0.44], payload={"city": "Beijing"}
            ),
            PointStruct(
                id=6, vector=[0.35, 0.08, 0.11, 0.44], payload={"city": "Mumbai"}
            ),
        ],
    )

    assert upsert.status == "completed"

    # Set Payload to the collection
    set_payload = client.set_payload(
        collection_name=COLLECTION_NAME,
        payload={
            "city": "Vienna",
        },
        points=[1],
    )
    assert set_payload.status == "completed"

    # Overwrite Payload to the collection
    overwrite_payload = client.overwrite_payload(
        collection_name=COLLECTION_NAME,
        payload={
            "city": "Toronto",
        },
        points=[1],
    )
    assert overwrite_payload.status == "completed"

    # Clear Payload to the collection
    clear_payload = client.clear_payload(
        collection_name=COLLECTION_NAME,
        points_selector=[1],
    )
    assert clear_payload.status == "completed"

    # Delete Payload to the collection
    delete_payload = client.delete_payload(
        collection_name=COLLECTION_NAME,
        keys=["city"],
        points=[2],
    )
    assert delete_payload.status == "completed"

    # Upload Points to the collection
    upload_points = client.upload_points(
        collection_name=COLLECTION_NAME,
        points=[
            models.PointStruct(
                id=1,
                payload={
                    "city": "Toronto",
                },
                vector=[0.9, 0.1, 0.1, 0.2],
            )
        ],
        max_retries=3,
    )
    assert upload_points is None

    # Update Vectors in the collection
    update_vectors = client.update_vectors(
        collection_name=COLLECTION_NAME,
        points=[
            models.PointVectors(
                id=1,
                vector=[0.9, 0.1, 0.1, 0.2],
            )
        ],
    )
    assert update_vectors.status == "completed"

    # Delete Points in the collection
    delt = client.delete(
        collection_name=COLLECTION_NAME,
        points_selector=models.PointIdsList(
            points=[2],
        ),
    )
    assert delt.status == "completed"

    # Retrieve vectors from the collection
    retrieve = client.retrieve(
        collection_name=COLLECTION_NAME,
        ids=[4],
    )
    assert isinstance(retrieve, list)

    client.create_payload_index(
        collection_name=COLLECTION_NAME,
        field_name="city",
        field_schema="keyword",
    )

    # Scroll vectors from the collection
    scroll = client.scroll(
        collection_name=COLLECTION_NAME,
        scroll_filter=models.Filter(
            must=[
                models.FieldCondition(
                    key="city", match=models.MatchValue(value="Toronto")
                ),
            ]
        ),
        limit=1,
        with_payload=True,
        with_vectors=False,
    )
    assert isinstance(scroll, tuple)

    # # Search vectors from the collection
    # search = client.search(
    #     collection_name=COLLECTION_NAME,
    #     query_filter=models.Filter(
    #         must=[
    #             models.FieldCondition(
    #                 key="city",
    #                 match=models.MatchValue(
    #                     value="London",
    #                 ),
    #             )
    #         ]
    #     ),
    #     search_params=models.SearchParams(hnsw_ef=128, exact=False),
    #     query_vector=[0.2, 0.1, 0.9, 0.7],
    #     limit=3,
    # )
    # assert isinstance(search, list)

    # Search groups from the collection
    search_groups = client.search_groups(
        collection_name=COLLECTION_NAME,
        query_vector=[1.1, 1.2, 1.3, 1.4],
        group_by="city",
        limit=4,
        group_size=2,
    )
    assert isinstance(search_groups.groups, list)

    # Get Recommened vectors from the collection
    recommend = client.recommend(
        collection_name=COLLECTION_NAME,
        positive=[1, 3],
        negative=[4, [0.2, 0.3, 0.4, 0.5]],
        strategy=models.RecommendStrategy.AVERAGE_VECTOR,
        query_filter=models.Filter(
            must=[
                models.FieldCondition(
                    key="city",
                    match=models.MatchValue(
                        value="London",
                    ),
                )
            ]
        ),
        limit=3,
    )
    assert isinstance(recommend, list)

    # # Delete collection
    # del_col = client.delete_collection(
    #     collection_name=COLLECTION_NAME,
    # )
    # assert del_col is True
