from typing import Annotated

from fastmcp import FastMCP

mcp = FastMCP("mymcp")


@mcp.tool
def divide(
    dividend: Annotated[float, "Dividend/numerator of the division."],
    divisor: Annotated[float, "Divisor/denominator of the division."],
) -> Annotated[float, "Result of the division."]:
    """Divide two numbers and return the result."""
    return dividend / divisor
