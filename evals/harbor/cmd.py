#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.12"
# dependencies = ["harbor==0.8.0", "PyYAML>=6.0"]
# ///
"""Harbor benchmark runner and reporter for Goose.

Subcommands:
    run        run a benchmark job
    list       list all runs in the runs/ directory
    show       per-task results for one run
    task       full detail for one task in one run
    compare    compare two runs task-by-task
    rm         remove one or more runs
"""

from __future__ import annotations

import argparse
from pathlib import Path

from reporter import cmd_compare, cmd_list, cmd_pull, cmd_rm, cmd_show, cmd_task
from runner import (
    DEFAULT_CONCURRENCY,
    DEFAULT_DATASET,
    DEFAULT_EXTENSIONS,
    DEFAULT_MAX_TURNS,
    DEFAULT_MODEL,
    cmd_run,
    parse_csv,
)


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    sub = parser.add_subparsers(dest="cmd", required=True)

    p_run = sub.add_parser("run", help="run a benchmark job")
    p_run.add_argument("goose_binary", type=Path, help="path to the goose binary to test")
    p_run.add_argument("--dataset", default=DEFAULT_DATASET)
    p_run.add_argument("--model", default=DEFAULT_MODEL)
    p_run.add_argument(
        "--tasks",
        type=parse_csv,
        default=[],
        help="comma-separated task names (default: all tasks in dataset)",
    )
    p_run.add_argument(
        "--extensions",
        type=parse_csv,
        default=DEFAULT_EXTENSIONS,
        help=f"comma-separated extension names (default: {','.join(DEFAULT_EXTENSIONS)})",
    )
    p_run.add_argument("--trials", type=int, default=1)
    p_run.add_argument("--concurrency", type=int, default=DEFAULT_CONCURRENCY)
    p_run.add_argument("--max-turns", type=int, default=DEFAULT_MAX_TURNS)
    p_run.add_argument("--timeout-multiplier", type=float, default=1.0)
    p_run.add_argument("--job-name")
    p_run.add_argument(
        "--no-install-goose-runtime-deps",
        dest="install_goose_runtime_deps",
        action="store_false",
        default=True,
        help="skip apt-get install libgomp1 inside the task container",
    )
    p_run.add_argument("--dry-run", action="store_true")

    sub.add_parser("list", help="list all runs with summary stats")

    p_show = sub.add_parser("show", help="per-task results for one run")
    p_show.add_argument("job_name")
    p_show.add_argument(
        "--status",
        choices=["pass", "partial", "fail", "timeout", "error", "no-reward"],
    )

    p_task = sub.add_parser("task", help="full detail for one task in one run")
    p_task.add_argument("job_name")
    p_task.add_argument("task_name")
    p_task.add_argument("--tail", type=int, default=0, help="tail N lines of the agent log")

    p_cmp = sub.add_parser("compare", help="compare two runs task-by-task")
    p_cmp.add_argument("job_a")
    p_cmp.add_argument("job_b")
    p_cmp.add_argument("-v", "--verbose", action="store_true")

    p_rm = sub.add_parser("rm", help="remove one or more runs")
    p_rm.add_argument("job_names", nargs="+", help="job names under runs/")
    p_rm.add_argument("-y", "--yes", action="store_true", help="skip confirmation prompt")

    p_pull = sub.add_parser("pull", help="rsync runs from a remote machine")
    p_pull.add_argument(
        "remote",
        help="user@host:/path/to/goose (we append evals/harbor/runs/)",
    )
    p_pull.add_argument(
        "--jobs",
        nargs="*",
        help="restrict to specific job names (default: all runs)",
    )
    p_pull.add_argument(
        "--delete",
        action="store_true",
        help="remove local runs that no longer exist on the remote",
    )

    return parser


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    if args.cmd == "run":
        return cmd_run(args)
    if args.cmd == "list":
        return cmd_list(args)
    if args.cmd == "show":
        return cmd_show(args)
    if args.cmd == "task":
        return cmd_task(args)
    if args.cmd == "compare":
        return cmd_compare(args)
    if args.cmd == "rm":
        return cmd_rm(args)
    if args.cmd == "pull":
        return cmd_pull(args)
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
