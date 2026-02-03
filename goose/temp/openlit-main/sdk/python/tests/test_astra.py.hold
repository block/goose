# pylint: disable=duplicate-code, no-name-in-module, assignment-from-no-return
"""
This module contains tests for AstraDB functionality using the AstraDB Python library.

Tests cover various API endpoints, including create_collection, add, query,
upsert, update, get, peek and delete.
These tests validate integration with OpenLIT.

Note: Ensure the environment is properly configured for AstraDB access and OpenLIT monitoring
prior to running these tests.
"""

import os
from astrapy import DataAPIClient
import openlit

# Initialize the client
client = DataAPIClient(os.getenv("ASTRA_DB_APPLICATION_TOKEN"))
db = client.get_database_by_api_endpoint(os.getenv("ASTRA_DB_API_ENDPOINT"))

# Initialize environment and application name for OpenLIT monitoring
openlit.init(
    environment="openlit-python-testing", application_name="openlit-python-astra-test"
)


def test_db_astra():
    """
    Tests basic operations within a AstraDB collection.

    This includes creating a new collection, adding documents to the collection,
    querying the collection, and deleting documents from the collection. The test
    verifies correct behavior of these operations by asserting expected outcomes at
    """

    # collection = db.create_collection(
    #   "openlit",
    #   dimension=3,
    #   metric=astrapy.constants.VectorMetric.COSINE,
    # )
    # assert collection.name == "openlit"
    collection = db.get_collection("openlit")

    response = collection.insert_one(
        {"summary": "I was flying", "$vector": [-0.4, 0.7, 0]}
    )
    assert isinstance(response.inserted_id, str)

    response = collection.insert_many(
        [
            {
                "tags": ["food"],
                "summary": "A dinner on the Moon",
                "$vector": [0.2, -0.3, -0.5],
            },
            {
                "summary": "Riding the waves",
                "tags": ["sport"],
                "$vector": [0, 0.2, 1],
            },
            {
                "summary": "Friendly aliens in town",
                "tags": ["scifi"],
                "$vector": [-0.3, 0, 0.8],
            },
            {
                "summary": "Meeting Beethoven at the dentist",
                "$vector": [0.2, 0.6, 0],
            },
        ],
    )
    assert isinstance(response.inserted_ids, list)

    response = collection.update_one(
        {"tags": "sport"},
        {"$set": {"summary": "Surfers' paradise"}},
    )
    assert isinstance(response.update_info["n"], int)

    response = collection.update_many(
        {"tags": {"$exists": False}},
        {"$set": {"name": "unknown"}},
    )
    assert isinstance(response.update_info["n"], int)

    response = collection.find_one_and_update(
        {"tags": {"$exists": True}},
        {"$set": {"name": "Mr."}},
    )
    assert isinstance(response["tags"], list)

    response = collection.find_one({"tag": {"$exists": True}})
    assert response is None

    cursor = collection.find(
        {},
        sort={"$vector": [0, 0.2, 0.4]},
        limit=2,
        include_similarity=True,
    )
    for response in cursor:
        assert isinstance(response["summary"], str)

    response = collection.replace_one(
        {"tag": {"$exists": True}},  # filter
        {"summary": "Pest"},  # replacement
    )
    assert isinstance(response.update_info["n"], int)

    response = collection.delete_one({"tag": {"$exists": True}})
    assert isinstance(response.deleted_count, int)

    response = collection.delete_many({"tag": {"$exists": True}})
    assert isinstance(response.deleted_count, int)

    response = collection.find_one_and_delete({"status": "stale_entry"})
    assert response is None

    # response = db.drop_collection(name_or_collection="openlit")
    # assert isinstance(response["ok"], int)
