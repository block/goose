import json
from json import JSONDecodeError
from pathlib import Path
from typing import Any

import click


@click.command()
@click.option(
    "--directory",
    "directory",
    required=True,
    type=click.Path(exists=True, file_okay=False, path_type=Path),
    help="Path to the directory containing app.json and functions.json",
)
def convert(directory: Path) -> None:
    try:
        _convert_directory(directory)
    except ValueError as exc:
        raise click.ClickException(str(exc)) from exc


def _convert_directory(directory: Path) -> None:
    app_path = directory / "app.json"
    functions_path = directory / "functions.json"
    server_path = directory / "server.json"
    tools_path = directory / "tools.json"

    if not app_path.exists():
        raise ValueError(f"Missing app.json in {directory}")
    if not functions_path.exists():
        raise ValueError(f"Missing functions.json in {directory}")
    if server_path.exists():
        raise ValueError(f"server.json already exists in {directory}")
    if tools_path.exists():
        raise ValueError(f"tools.json already exists in {directory}")

    app_data = _load_json(app_path)
    functions_data = _load_json(functions_path)

    server_data = _transform_app(app_data)
    tools_data = _transform_functions(functions_data)

    _write_json(server_path, server_data)
    _write_json(tools_path, tools_data)

    app_path.unlink()
    functions_path.unlink()

    click.echo(f"Converted files written to {server_path.name} and {tools_path.name}")


def _load_json(path: Path) -> Any:
    try:
        with path.open() as file:
            return json.load(file)
    except JSONDecodeError as exc:
        raise ValueError(f"Invalid JSON in {path}: {exc}") from exc


def _transform_app(app_data: Any) -> dict[str, Any]:
    if not isinstance(app_data, dict):
        raise ValueError("app.json must contain a JSON object")

    missing_fields = [field for field in ("name", "description") if field not in app_data]
    if missing_fields:
        raise ValueError(f"app.json missing required fields: {', '.join(missing_fields)}")

    return {"name": app_data["name"], "description": app_data["description"]}


def _transform_functions(functions_data: Any) -> list[dict[str, Any]]:
    if not isinstance(functions_data, list):
        raise ValueError("functions.json must contain a JSON array")

    transformed_tools: list[dict[str, Any]] = []
    for item in functions_data:
        if not isinstance(item, dict):
            raise ValueError("Each entry in functions.json must be a JSON object")
        transformed_tools.append(_transform_function(item))

    return transformed_tools


def _transform_function(function_data: dict[str, Any]) -> dict[str, Any]:
    name = function_data.get("name")
    description = function_data.get("description")
    if not name or not description:
        raise ValueError("Each function must include 'name' and 'description'")

    protocol = function_data.get("protocol")
    protocol_data = function_data.get("protocol_data") or {}
    tool_metadata = _build_tool_metadata(protocol, protocol_data)

    parameters = function_data.get("parameters")
    if parameters is None:
        raise ValueError(f"Function '{name}' is missing 'parameters'")

    return {
        "name": name,
        "description": description,
        "tool_metadata": tool_metadata,
        "input_schema": parameters,
    }


def _build_tool_metadata(protocol: Any, protocol_data: Any) -> dict[str, Any]:
    if protocol not in {"connector", "rest"}:
        raise ValueError(f"Unsupported protocol '{protocol}'")

    metadata: dict[str, Any] = {"type": protocol}

    if protocol == "rest":
        if not isinstance(protocol_data, dict):
            raise ValueError("protocol_data must be an object for rest protocol")
        method = protocol_data.get("method")
        path = protocol_data.get("path")
        server_url = protocol_data.get("server_url")
        missing = [
            field for field in ("method", "path", "server_url") if not protocol_data.get(field)
        ]
        if missing:
            raise ValueError("protocol_data for rest protocol must include: " + ", ".join(missing))
        endpoint = _merge_url(server_url, path)
        metadata.update({"method": method, "endpoint": endpoint})

    return metadata


def _merge_url(server_url: Any, path: Any) -> str:
    if not isinstance(server_url, str) or not isinstance(path, str):
        raise ValueError("server_url and path must be strings")

    return f"{server_url}{path}"


def _write_json(path: Path, payload: Any) -> None:
    with path.open("w") as file:
        json.dump(payload, file, indent=2)
        file.write("\n")
