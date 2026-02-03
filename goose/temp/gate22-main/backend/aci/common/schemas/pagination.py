from typing import TypeVar

from pydantic import BaseModel, Field

T = TypeVar("T")


class PaginationParams(BaseModel):
    offset: int = Field(default=0, ge=0, description="Offset for pagination")
    limit: int = Field(
        default=30, ge=1, le=100, description="Limit for pagination, max 100, default 30"
    )


class PaginationResponse[T](BaseModel):
    data: list[T]
    offset: int


class CursorPaginationParams(BaseModel):
    """
    Cursor-based pagination parameters
    """

    cursor: str | None = Field(
        default=None,
        description="Opaque cursor for pagination. Use the next_cursor from previous response.",
    )
    limit: int = Field(
        default=50, ge=1, le=100, description="Number of items to return, max 100, default 50"
    )


class CursorPaginationResponse[T](BaseModel):
    """
    Cursor-based pagination response with next_cursor for efficient pagination.
    """

    data: list[T]
    next_cursor: str | None = Field(
        default=None, description="Cursor for the next page. None if no more data."
    )
