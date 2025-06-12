import argparse
import asyncio
import json
from typing import Awaitable, Callable
from pathlib import Path
from fastmcp import FastMCP, Context
from mcp import types

server = FastMCP(
    "Subrecipes 🚀",
    instructions="This extension allows you to run subrecipes.",
)

del server._mcp_server.request_handlers[
    types.ListResourcesRequest
]  # Otherwise the Goose system prompt says this supports resources


def runner(subrecipe: dict) -> tuple[str, Callable[[Context], Awaitable[str]]]:
    path = Path(subrecipe["path"])
    name = (
        "_".join(path.parts[-2:])
        .removesuffix(".yaml")
        .removesuffix(".yml")
        .removesuffix(".json")
    )[:64]  # Claude rejected a name for being longer than 64 characters
    name = name.replace(".", "_").replace(" ", "_")

    async def tool_func(ctx: Context) -> str:
        await ctx.info(f"Running subrecipe {name}")

        # shell out to the subrecipe and stream stdout/stderr
        process = await asyncio.create_subprocess_exec(
            "goose",
            "run",
            "--recipe",
            path,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
        )

        output = []

        async def read_stream(stream, name):
            while True:
                line = await stream.readline()
                if not line:
                    break
                if line := line.decode("utf-8").rstrip():
                    await ctx.log(f"{name}: {line}")
                    output.append(line)

        await asyncio.gather(
            read_stream(process.stdout, "stdout"),
            read_stream(process.stderr, "stderr"),
        )

        return_code = await process.wait()
        return f"Subrecipe {name} finished with return code {return_code}"

    return name, tool_func


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--subrecipes-json", type=str, required=True)
    args = parser.parse_args()

    subrecipes_json = json.loads(args.subrecipes_json)

    for subrecipe in subrecipes_json:
        name, tool_func = runner(subrecipe)
        server.tool(
            tool_func,
            name=name,
            description=f"Run {name}",
        )

    server.run()


if __name__ == "__main__":
    main()
