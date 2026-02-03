from openai import OpenAI

from aci.common.logging_setup import get_logger
from aci.common.schemas.mcp_server import MCPServerEmbeddingFields
from aci.common.schemas.mcp_tool import MCPToolEmbeddingFields

logger = get_logger(__name__)


# TODO: abstract inference/embedding functions into a separate service
# potentially allow overriding the embedding model, dimension, inference provider
def generate_mcp_server_embedding(
    openai_client: OpenAI, mcp_server: MCPServerEmbeddingFields
) -> list[float]:
    """
    Generate embedding for mcp_server.
    TODO: what else should be included or not in the embedding?
    """
    logger.debug(f"Generating embedding for mcp_server: {mcp_server.name}...")
    text_for_embedding = mcp_server.model_dump_json()
    logger.debug(f"Text for mcp_server embedding: {text_for_embedding}")
    return generate_embedding(openai_client, text_for_embedding)


# TODO: batch generate mcp tool embeddings
# TODO: update mcp_server embedding to include mcp tool embeddings whenever mcp tools are
# added/updated?
def generate_mcp_tool_embeddings(
    openai_client: OpenAI,
    mcp_tools: list[MCPToolEmbeddingFields],
) -> list[list[float]]:
    logger.debug(f"Generating embeddings for {len(mcp_tools)} mcp tools...")
    mcp_tool_embeddings: list[list[float]] = []
    for mcp_tool in mcp_tools:
        mcp_tool_embeddings.append(generate_mcp_tool_embedding(openai_client, mcp_tool))

    return mcp_tool_embeddings


def generate_mcp_tool_embedding(
    openai_client: OpenAI,
    mcp_tool: MCPToolEmbeddingFields,
) -> list[float]:
    logger.debug(f"Generating embedding for mcp tool: {mcp_tool.name}...")
    text_for_embedding = mcp_tool.model_dump_json()
    logger.debug(
        f"Text for mcp tool embedding: {text_for_embedding[:100]}{'...' if len(text_for_embedding) > 100 else ''}"  # noqa: E501
    )
    return generate_embedding(openai_client, text_for_embedding)


# TODO: allow different inference providers
def generate_embedding(
    openai_client: OpenAI,
    text: str,
    embedding_model: str = "text-embedding-3-small",
    embedding_dimension: int = 1024,
) -> list[float]:
    """
    Generate an embedding for the given text using OpenAI's model.
    """
    logger.debug(f"Generating embedding for text: {text[:100]}{'...' if len(text) > 100 else ''}")
    try:
        response = openai_client.embeddings.create(
            input=[text],
            model=embedding_model,
            dimensions=embedding_dimension,
        )
        embedding: list[float] = response.data[0].embedding
        return embedding
    except Exception:
        logger.error("Error generating embedding", exc_info=True)
        raise
