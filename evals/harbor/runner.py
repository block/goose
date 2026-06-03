"""Build the harbor config and launch a benchmark job."""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
from datetime import datetime
from pathlib import Path
from typing import Any

import yaml

from agent import PROVIDER_SECRETS


HARBOR_DIR = Path(__file__).resolve().parent
RUNS_DIR = HARBOR_DIR / "runs"
CONFIG_TEMPLATE_PATH = HARBOR_DIR / "config_template.yaml"

DEFAULT_DATASET = "terminal-bench/terminal-bench-2"
DEFAULT_MODEL = "anthropic/claude-sonnet-4-6"
DEFAULT_EXTENSIONS = ["developer", "todo"]
DEFAULT_CONCURRENCY = 4
DEFAULT_MAX_TURNS = 100


def find_dotenv() -> Path | None:
    cwd_env = Path.cwd() / ".env"
    if cwd_env.is_file():
        return cwd_env
    script_env = HARBOR_DIR / ".env"
    if script_env.is_file():
        return script_env
    return None


def load_dotenv() -> None:
    env_path = find_dotenv()
    if env_path is None:
        return
    for line in env_path.read_text().splitlines():
        line = line.strip()
        if not line or line.startswith("#") or "=" not in line:
            continue
        key, _, value = line.partition("=")
        key = key.strip()
        value = value.strip().strip('"').strip("'")
        os.environ.setdefault(key, value)


def render_goose_config(extensions: list[str]) -> tuple[str, list[dict[str, str]]]:
    """Render config.yaml from the template, enabling the given extensions.

    Returns (config_yaml_text, recipe_extension_entries).
    Raises ValueError for any extension not found in the template.
    """
    if not CONFIG_TEMPLATE_PATH.is_file():
        raise FileNotFoundError(f"Missing template: {CONFIG_TEMPLATE_PATH}")
    template = yaml.safe_load(CONFIG_TEMPLATE_PATH.read_text())
    available = template.get("extensions") or {}

    unknown = [name for name in extensions if name not in available]
    if unknown:
        raise ValueError(
            f"Unknown extensions: {', '.join(unknown)}. "
            f"Known: {', '.join(sorted(available))}"
        )

    for name, entry in available.items():
        entry["enabled"] = name in extensions

    recipe_entries = [
        {"type": available[name]["type"], "name": name} for name in extensions
    ]
    return yaml.dump(template, sort_keys=False), recipe_entries


def default_job_name(model: str, dataset: str) -> str:
    safe_model = re.sub(r"[^A-Za-z0-9._-]+", "-", model).strip("-")
    safe_dataset = re.sub(r"[^A-Za-z0-9._-]+", "-", dataset).strip("-")
    timestamp = datetime.now().strftime("%Y-%m-%d__%H-%M-%S")
    return f"goose-{safe_dataset}-{safe_model}-{timestamp}"


def validate_job_name(job_name: str) -> str:
    if not re.match(r"^[A-Za-z0-9][A-Za-z0-9._-]*$", job_name):
        raise ValueError(
            "Job name must start with a letter or number and contain only "
            "letters, numbers, dots, underscores, and hyphens"
        )
    return job_name


def parse_csv(value: str) -> list[str]:
    return [item.strip() for item in value.split(",") if item.strip()]


PACKAGE_INDEX_ENV_VARS = ("UV_DEFAULT_INDEX", "PIP_INDEX_URL", "UV_INDEX_URL")


def package_index_env() -> dict[str, str]:
    index_url = next(
        (os.environ[key] for key in PACKAGE_INDEX_ENV_VARS if os.environ.get(key)),
        None,
    )
    if index_url is None:
        return {}
    return {key: index_url for key in PACKAGE_INDEX_ENV_VARS}


def dataset_config(dataset_ref: str, tasks: list[str]) -> dict[str, Any]:
    name, sep, ref = dataset_ref.rpartition("@")
    dataset_name = name if sep else dataset_ref
    dataset: dict[str, Any] = {"name": dataset_name}
    if sep:
        dataset["ref" if "/" in name else "version"] = ref
    if tasks:
        dataset["task_names"] = tasks
    return dataset


def build_harbor_config(args: argparse.Namespace) -> dict[str, Any]:
    if "/" not in args.model:
        raise ValueError("--model must be in provider/model form, e.g. anthropic/claude-sonnet-4-6")
    if args.trials < 1:
        raise ValueError("--trials must be at least 1")
    if args.concurrency < 1:
        raise ValueError("--concurrency must be at least 1")
    if args.timeout_multiplier <= 0:
        raise ValueError("--timeout-multiplier must be positive")

    goose_binary = args.goose_binary.expanduser().resolve()
    if not goose_binary.is_file():
        raise ValueError(f"--goose-binary does not exist or is not a file: {args.goose_binary}")

    config_yaml, extension_entries = render_goose_config(args.extensions)

    provider = args.model.split("/", 1)[0]
    missing_secrets = [
        key for key in PROVIDER_SECRETS.get(provider, []) if not os.environ.get(key)
    ]
    if missing_secrets:
        raise ValueError(
            f"Missing env vars for provider '{provider}': {', '.join(missing_secrets)}. "
            f"Set them in a .env file (cwd or {HARBOR_DIR}) or your shell."
        )

    agent_kwargs: dict[str, Any] = {
        "goose_binary": str(goose_binary),
        "config_yaml": config_yaml,
        "extension_entries": extension_entries,
        "install_goose_runtime_deps": args.install_goose_runtime_deps,
    }
    if args.max_turns is not None:
        agent_kwargs["max_turns"] = args.max_turns

    job_name = (
        validate_job_name(args.job_name)
        if args.job_name
        else default_job_name(args.model, args.dataset)
    )

    index_env = package_index_env()
    container_env_passthrough = [
        f"{key}=${{{key}}}"
        for key in PROVIDER_SECRETS.get(provider, [])
        if os.environ.get(key)
    ] + [f"{key}={value}" for key, value in index_env.items()]

    config: dict[str, Any] = {
        "job_name": job_name,
        "jobs_dir": str(RUNS_DIR),
        "n_attempts": args.trials,
        "n_concurrent_trials": args.concurrency,
        "environment": {
            "type": "docker",
            "force_build": False,
            "delete": True,
            "env": container_env_passthrough,
        },
        "agents": [
            {
                "import_path": "agent:GooseBinaryAgent",
                "model_name": args.model,
                "kwargs": agent_kwargs,
            }
        ],
        "datasets": [dataset_config(args.dataset, args.tasks)],
    }
    if index_env:
        config["verifier"] = {"env": index_env}
    if args.timeout_multiplier != 1.0:
        config["timeout_multiplier"] = args.timeout_multiplier
    return config


def cmd_run(args: argparse.Namespace) -> int:
    load_dotenv()
    try:
        config = build_harbor_config(args)
    except Exception as error:
        print(f"error: {error}", file=sys.stderr)
        return 2

    RUNS_DIR.mkdir(parents=True, exist_ok=True)
    job_dir = RUNS_DIR / config["job_name"]
    job_dir.mkdir(parents=True, exist_ok=True)
    config_path = job_dir / "_generated_config.json"
    config_path.write_text(json.dumps(config, indent=2) + "\n")

    command = ["harbor", "run", "-c", str(config_path)]
    print(f"Job:    {config['job_name']}")
    print(f"Config: {config_path}")
    print(f"Runs:   {RUNS_DIR}")
    if args.dry_run:
        return 0

    env = os.environ.copy()
    env["PYTHONPATH"] = f"{HARBOR_DIR}{os.pathsep}{env.get('PYTHONPATH', '')}".rstrip(os.pathsep)
    completed = subprocess.run(command, env=env, check=False)
    return completed.returncode
