# pylint: disable=duplicate-code, no-name-in-module
"""
This module contains tests for Milvus functionality using the Milvus Python library.

Tests cover various API endpoints, including create_collection, add, query,
upsert, update, get and delete. 
These tests validate integration with OpenLIT.

Environment Variables:
    - MILVUS_URL: Milvus Cloud Instance URL.
    - MILVUS_API_TOKEN: Milvus API api_key for authentication.

Note: Ensure the environment is properly configured for Milvus access and OpenLIT monitoring
prior to running these tests.
"""

import os
from pymilvus import MilvusClient
import openlit

# Initialize Milvus client
client = MilvusClient(
    uri=os.getenv("MILVUS_URL"),
    token = os.getenv("MILVUS_API_TOKEN")
)

# Initialize environment and application name for OpenLIT monitoring
openlit.init(environment="openlit-testing", application_name="openlit-python-test")

COLLECTION_NAME = "openlit"

def test_db_milvus():
    """
    Tests basic operations within a Milvus collection.

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
    #   dimension=5
    # )

    # assert collection is None

    data=[
      # pylint: disable=line-too-long
      {"id": 0, "vector": [0.3580376395471989, -0.6023495712049978, 0.18414012509913835, -0.26286205330961354, 0.9029438446296592], "color": "pink_8682"},
      {"id": 1, "vector": [0.19886812562848388, 0.06023560599112088, 0.6976963061752597, 0.2614474506242501, 0.838729485096104], "color": "red_7025"},
      {"id": 2, "vector": [0.43742130801983836, -0.5597502546264526, 0.6457887650909682, 0.7894058910881185, 0.20785793220625592], "color": "orange_6781"},
      {"id": 3, "vector": [0.3172005263489739, 0.9719044792798428, -0.36981146090600725, -0.4860894583077995, 0.95791889146345], "color": "pink_9298"},
      {"id": 4, "vector": [0.4452349528804562, -0.8757026943054742, 0.8220779437047674, 0.46406290649483184, 0.30337481143159106], "color": "red_4794"},
      {"id": 5, "vector": [0.985825131989184, -0.8144651566660419, 0.6299267002202009, 0.1206906911183383, -0.1446277761879955], "color": "yellow_4222"},
      {"id": 6, "vector": [0.8371977790571115, -0.015764369584852833, -0.31062937026679327, -0.562666951622192, -0.8984947637863987], "color": "red_9392"},
      {"id": 7, "vector": [-0.33445148015177995, -0.2567135004164067, 0.8987539745369246, 0.9402995886420709, 0.5378064918413052], "color": "grey_8510"},
      {"id": 8, "vector": [0.39524717779832685, 0.4000257286739164, -0.5890507376891594, -0.8650502298996872, -0.6140360785406336], "color": "white_9381"},
      {"id": 9, "vector": [0.5718280481994695, 0.24070317428066512, -0.3737913482606834, -0.06726932177492717, -0.6980531615588608], "color": "purple_4976"}
    ]

    insert = client.insert(
      collection_name=COLLECTION_NAME,
      data=data
    )

    assert insert["insert_count"] == 10

    upsert = client.upsert(
      collection_name=COLLECTION_NAME,
      data=data
    )

    assert upsert["upsert_count"] == 10

    query_vectors = [
      [0.041732933, 0.013779674, -0.027564144, -0.013061441, 0.009748648]
    ]

    search = client.search(
      collection_name=COLLECTION_NAME,
      data=query_vectors,
      limit=3,
    )

    assert isinstance(search[0], list)

    query = client.query(
      collection_name=COLLECTION_NAME,
      output_fields=["color"],
      limit=3,
    )

    assert isinstance(query[0], dict)

    getqry = client.get(
        collection_name=COLLECTION_NAME,
        ids=[1,2,3],
        output_fields=["color", "vector"]
    )

    assert isinstance(getqry[0], dict)

    delt = client.delete(
      collection_name=COLLECTION_NAME,
      filter="id in [5,6,7,8,9]"
    )

    assert isinstance(delt, dict)

    # # Delete collection
    # client.drop_collection(
    #   collection_name=COLLECTION_NAME,
    # )
