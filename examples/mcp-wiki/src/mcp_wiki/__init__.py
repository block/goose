import argparse
from .server import mcp

def main():
    """MCP Wiki: read Wikipedia articles and convert them to Markdown."""
    parser = argparse.ArgumentParser(
        description="Gives you the ability to read Wikipedia articles and convert them to Markdown."
    )

    _ = parser.parse_args()
    mcp.run()


if __name__ == "__main__":
    main()