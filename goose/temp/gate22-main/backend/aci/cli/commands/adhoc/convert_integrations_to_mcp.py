import json
from json import JSONDecodeError
from pathlib import Path
from typing import Any

import click

DEFAULT_OAUTH2_LOCATION = "header"
DEFAULT_OAUTH2_NAME = "Authorization"
DEFAULT_OAUTH2_PREFIX = "Bearer"


@click.command()
@click.option(
    "--directory",
    "directory",
    required=True,
    type=click.Path(exists=True, file_okay=False, path_type=Path),
    help="Path to the directory containing app.json",
)
def convert(directory: Path) -> None:
    """Convert app.json to server.json in the provided directory."""
    try:
        _convert(directory)
    except ValueError as exc:
        raise click.ClickException(str(exc)) from exc


def _convert(directory: Path) -> None:
    app_path = directory / "app.json"
    server_path = directory / "server.json"
    functions_path = directory / "functions.json"

    if not app_path.exists():
        raise ValueError(f"Missing app.json in {directory}")
    if server_path.exists():
        raise ValueError(f"server.json already exists in {directory}")

    app_data = _load_json(app_path)
    server_data = _transform_app(app_data)

    _write_json(app_path, server_data)
    app_path.rename(server_path)
    if functions_path.exists():
        click.echo(f"Removing functions.json in {directory}")
        functions_path.unlink()

    click.echo(f"Converted app.json to server.json at {server_path}")


def _load_json(path: Path) -> Any:
    try:
        with path.open() as file:
            return json.load(file)
    except JSONDecodeError as exc:
        raise ValueError(f"Invalid JSON in {path}: {exc}") from exc


def _transform_app(app_data: Any) -> dict[str, Any]:
    if not isinstance(app_data, dict):
        raise ValueError("app.json must contain a JSON object")

    name = _require_non_empty_string(app_data, "name")
    description = _require_non_empty_string(app_data, "description")
    logo = _optional_string(app_data.get("logo"))
    categories = _coerce_categories(app_data.get("categories"))
    auth_configs = _transform_security_schemes(app_data.get("security_schemes"))

    server_data: dict[str, Any] = {
        "name": name,
        "url": f"https://mcp.aci.dev/virtual/mcp?server_name={name}",
        "transport_type": "streamable_http",
        "description": description,
        "logo": logo if logo is not None else "",
        "categories": categories,
        "auth_configs": auth_configs,
        "server_metadata": {"is_virtual_mcp_server": True},
    }

    return server_data


def _optional_string(value: Any) -> str | None:
    if value is None:
        return None
    if not isinstance(value, str):
        raise ValueError("Optional string fields must be strings when provided")
    return value


def _coerce_categories(value: Any) -> list[str]:
    if value is None:
        return []
    if not isinstance(value, list):
        raise ValueError("Field 'categories' must be an array of strings")
    categories: list[str] = []
    for item in value:
        if not isinstance(item, str) or not item.strip():
            raise ValueError("Each category must be a non-empty string")
        categories.append(item)
    return categories


def _transform_security_schemes(value: Any) -> list[dict[str, Any]]:
    if value is None:
        return []
    if not isinstance(value, dict):
        raise ValueError("Field 'security_schemes' must be an object")

    auth_configs: list[dict[str, Any]] = []
    for scheme_name, scheme_data in value.items():
        if scheme_name == "oauth2":
            auth_configs.append(_transform_oauth2(scheme_data))
        elif scheme_name == "api_key":
            auth_configs.append(_transform_api_key(scheme_data))
        elif scheme_name == "no_auth":
            auth_configs.append(_transform_no_auth(scheme_data))
        else:
            raise ValueError(f"Unsupported security scheme '{scheme_name}'")

    return auth_configs


def _transform_oauth2(data: Any) -> dict[str, Any]:
    if not isinstance(data, dict):
        raise ValueError("OAuth2 configuration must be an object")

    result: dict[str, Any] = {
        "type": "oauth2",
        "client_id": _require_non_empty_string(data, "client_id"),
        "client_secret": _require_non_empty_string(data, "client_secret"),
        "scope": _require_string(data, "scope"),
        "authorize_url": _require_non_empty_string(data, "authorize_url"),
        "access_token_url": _require_non_empty_string(data, "access_token_url"),
        "refresh_token_url": _require_non_empty_string(data, "refresh_token_url"),
    }

    location = data.get("location")
    name = data.get("name")
    prefix = data.get("prefix")

    if not isinstance(location, str):
        raise ValueError("OAuth2 location must be a string")
    if location != DEFAULT_OAUTH2_LOCATION:
        result["location"] = location

    if not isinstance(name, str):
        raise ValueError("OAuth2 name must be a string")
    if name != DEFAULT_OAUTH2_NAME:
        result["name"] = name

    if not isinstance(prefix, str):
        raise ValueError("OAuth2 prefix must be a string")
    if prefix != DEFAULT_OAUTH2_PREFIX:
        result["prefix"] = prefix

    token_endpoint_auth_method = data.get("token_endpoint_auth_method")
    if token_endpoint_auth_method is not None:
        if not isinstance(token_endpoint_auth_method, str):
            raise ValueError("OAuth2 token_endpoint_auth_method must be a string")
        result["token_endpoint_auth_method"] = token_endpoint_auth_method

    return result


def _transform_api_key(data: Any) -> dict[str, Any]:
    if not isinstance(data, dict):
        raise ValueError("API key configuration must be an object")

    location = _require_non_empty_string(data, "location")
    name = _require_non_empty_string(data, "name")

    result: dict[str, Any] = {
        "type": "api_key",
        "location": location,
        "name": name,
    }

    prefix = data.get("prefix")
    if prefix is not None:
        if not isinstance(prefix, str):
            raise ValueError("API key prefix must be a string")
        result["prefix"] = prefix

    return result


def _transform_no_auth(data: Any) -> dict[str, Any]:
    if data not in (None, {}):
        raise ValueError("no_auth configuration must be empty")
    return {"type": "no_auth"}


def _require_non_empty_string(data: dict[str, Any], key: str) -> str:
    value = data.get(key)
    if not isinstance(value, str) or not value.strip():
        raise ValueError(f"Field '{key}' must be a non-empty string")
    return value


def _require_string(data: dict[str, Any], key: str) -> str:
    value = data.get(key)
    if not isinstance(value, str):
        raise ValueError(f"Field '{key}' must be a string")
    return value


def _write_json(path: Path, payload: Any) -> None:
    with path.open("w") as file:
        json.dump(payload, file, indent=2)
        file.write("\n")
