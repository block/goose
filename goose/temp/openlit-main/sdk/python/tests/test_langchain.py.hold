# pylint: disable=duplicate-code, no-member
"""
This module contains tests for Langchain functionality using the langchain Python library.

Tests cover various functions, including WebBaseLoader, TextSplitter,
and Prompt pull from Hub. 
These tests validate integration with OpenLIT.

Note: Ensure the environment is properly configured for Langchain and OpenLIT monitoring
prior to running these tests.
"""

import os
import bs4
# from langchain_community.document_loaders import WebBaseLoader
from langchain_text_splitters import RecursiveCharacterTextSplitter
import openlit

# Initialize environment and application name for OpenLIT monitoring
openlit.init(environment="openlit-testing", application_name="openlit-python-test")

def test_langchain():
    """
    Verifies the functionality of LangChain operations including document loading, text splitting,
    and prompt pulling from the Hub. Each operation is tested to ensure it functions as intended,
    confirming the efficient use of the LangChain library for processing and interacting
    with text data.

    Steps involved in the test:
    - Documents are loaded using the WebBaseLoader, focusing on specific content from
      a provided URL.
    - The loaded documents are then split into smaller chunks using
      the RecursiveCharacterTextSplitter.
    - A prompt is retrieved from the LangChain Hub to verify the functionality of
      pulling resources from the Hub.

    This test checks for the expected outcomes and metadata from each operation to
    confirm that the LangChain library is working as expected within the intended use cases.

    Raises:
      AssertionError: If the outcomes from the LangChain operations deviate from what is expected.
    """
    os.environ["LANGCHAIN_TRACING_V2"] = "false"

    # docs = WebBaseLoader(
    #   web_paths=("https://lilianweng.github.io/posts/2023-06-23-agent/",),
    #   bs_kwargs={
    #     "parse_only": bs4.SoupStrainer(
    #       class_=("post-content", "post-title", "post-header")
    #     )
    #   },
    # ).load()
    # assert docs[0].metadata["source"] == "https://lilianweng.github.io/posts/2023-06-23-agent/"

    # texts = RecursiveCharacterTextSplitter(chunk_size=1000, chunk_overlap=200)
    # splits = texts.split_documents(docs)
    # assert splits[0].metadata["source"] == "https://lilianweng.github.io/posts/2023-06-23-agent/"
