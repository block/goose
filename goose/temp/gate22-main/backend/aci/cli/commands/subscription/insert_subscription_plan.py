import json
from pathlib import Path
from typing import Any

import click
from rich.console import Console
from sqlalchemy.orm import Session

from aci.cli import config
from aci.common import utils
from aci.common.db import crud
from aci.common.schemas.subscription import SubscriptionPlanCreate

console = Console()


@click.command()
@click.option(
    "--plans-file",
    "-f",
    "plans_file",
    type=click.Path(exists=True, path_type=Path),
    required=True,
    help="Path to the plans file",
)
def insert_subscription_plan(plans_file: Path) -> None:
    console.print("Inserting plan")
    with utils.create_db_session(config.DB_FULL_URL) as db_session:
        with open(plans_file) as f:
            plans = json.load(f)
            insert_subscription_plans_impl(db_session, plans)


def insert_subscription_plans_impl(db_session: Session, plans_data: list[dict[str, Any]]) -> None:
    for plan_data in plans_data:
        plan = SubscriptionPlanCreate.model_validate(plan_data)
        crud.subscriptions.insert_subscription_plan(db_session, plan)
        console.print(f"[bold green]Inserted plan: {plan.plan_code}[/bold green]")
        console.print(f"{plan.model_dump_json(indent=4)}")
    db_session.commit()
