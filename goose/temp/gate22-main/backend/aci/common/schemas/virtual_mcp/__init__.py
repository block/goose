import re
from typing import Annotated, Literal

import jsonschema
from pydantic import BaseModel, ConfigDict, Field, RootModel, field_validator, model_validator

from aci.common.enums import HttpLocation, HttpMethod, VirtualMCPToolType


class RestVirtualMCPToolMetadata(BaseModel):
    type: Literal[VirtualMCPToolType.REST] = VirtualMCPToolType.REST
    method: HttpMethod
    # NOTE: unlike the tool-calling platform where we separate "path" and "server_url",
    # here we combine them into a single field "endpoint"
    endpoint: str


class ConnectorVirtualMCPToolMetadata(BaseModel):
    model_config = ConfigDict(extra="forbid")

    # NOTE: for now we don't allow any fields for connector type
    type: Literal[VirtualMCPToolType.CONNECTOR] = VirtualMCPToolType.CONNECTOR


class VirtualMCPToolMetadata(
    RootModel[
        Annotated[
            RestVirtualMCPToolMetadata | ConnectorVirtualMCPToolMetadata,
            Field(discriminator="type"),
        ]
    ]
):
    pass


class VirtualMCPAuthTokenData(BaseModel):
    """
    example:
    {
        "location": "header",
        "name": "Authorization",
        "prefix": "Bearer",
        "token": "1234567890"
    }
    """

    location: HttpLocation
    name: str
    prefix: str | None = None
    token: str


class VirtualMCPServerUpsert(BaseModel):
    """
    Schema for upserting a virtual MCP server in the database.
    Matches the server.json under the "virtual_mcp_servers" directory.
    """

    model_config = ConfigDict(extra="forbid")

    name: str
    description: str


class VirtualMCPToolUpsert(BaseModel):
    """
    Schema for upserting a virtual MCP tool in the database.
    Matches the tools.json under the "virtual_mcp_servers" directory.
    """

    model_config = ConfigDict(extra="forbid")

    name: str
    description: str
    input_schema: dict
    tool_metadata: VirtualMCPToolMetadata

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
    def validate_parameters(self) -> "VirtualMCPToolUpsert":
        # Validate that parameters schema itself is a valid JSON Schema
        jsonschema.validate(
            instance=self.input_schema, schema=jsonschema.Draft7Validator.META_SCHEMA
        )

        return self
