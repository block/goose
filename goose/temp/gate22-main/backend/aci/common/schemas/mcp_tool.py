import re
from datetime import datetime
from uuid import UUID

import jsonschema
from pydantic import BaseModel, Field, field_validator, model_validator


class MCPToolMetadata(BaseModel):
    canonical_tool_name: str = Field(
        ...,
        description="The canonical name of the tool of the mcp server",
    )
    canonical_tool_description_hash: str = Field(
        ...,
        description="The description of the tool of the mcp server in html format",
    )
    canonical_tool_input_schema_hash: str = Field(
        ...,
        description="The input schema of the tool of the mcp server in json schema format",
    )


class MCPToolUpsert(BaseModel, extra="forbid"):
    """
    Schema for upserting a mcp tool in the database.
    """

    name: str
    description: str
    input_schema: dict
    tags: list[str]
    tool_metadata: MCPToolMetadata

    @field_validator("name")
    def validate_name(cls, v: str) -> str:
        # Check valid characters
        if not re.match(r"^[A-Z0-9_]+$", v):
            raise ValueError(
                "Name must be uppercase, contain only letters, numbers, and underscores"
            )
        # Check exactly one occurrence of '__'
        if v.count("__") != 1:
            raise ValueError("Name must have exactly one occurrence of '__'")
        # Check no triple or more underscores
        if "___" in v:
            raise ValueError("Name must not have triple or more underscores")
        return v

    @model_validator(mode="after")
    def validate_parameters(self) -> "MCPToolUpsert":
        # Validate that parameters schema itself is a valid JSON Schema
        jsonschema.validate(
            instance=self.input_schema, schema=jsonschema.Draft7Validator.META_SCHEMA
        )

        return self


class MCPToolEmbeddingFields(BaseModel):
    """
    Fields used for generating the embeddings for semantic search.
    """

    name: str
    description: str
    input_schema: dict


# Only used in /mcp-tools/{id} endpoint
class MCPToolPublic(BaseModel):
    id: UUID
    name: str
    description: str
    input_schema: dict
    tags: list[str]

    created_at: datetime
    updated_at: datetime


class MCPToolPublicWithoutSchema(BaseModel):
    id: UUID
    name: str
    description: str
    tags: list[str]

    created_at: datetime
    updated_at: datetime
