import json
from pathlib import Path
from uuid import UUID

import click
from deepdiff import DeepDiff
from rich.console import Console
from sqlalchemy.orm import Session

from aci.cli import config
from aci.common import utils
from aci.common.db import crud
from aci.common.db.sql_models import VirtualMCPServer
from aci.common.schemas.virtual_mcp import VirtualMCPServerUpsert

console = Console()


@click.command()
@click.option(
    "--server-file",
    "server_file",
    required=True,
    type=click.Path(exists=True, path_type=Path),
    help="Path to the virtual mcp server JSON file",
)
@click.option(
    "--skip-dry-run",
    is_flag=True,
    help="Provide this flag to run the command and apply changes to the database",
)
def upsert(server_file: Path, skip_dry_run: bool) -> UUID:
    """
    Insert or update an VirtualMCPServer in the DB from a JSON file.
    If an virtual mcp server with the given name already exists, performs an update; otherwise,
    creates a new virtual mcp server.
    """
    with utils.create_db_session(config.DB_FULL_URL) as db_session:
        return upsert_helper(db_session, server_file, skip_dry_run)


def upsert_helper(db_session: Session, vms_file: Path, skip_dry_run: bool) -> UUID:
    vms_upsert = VirtualMCPServerUpsert.model_validate(json.loads(vms_file.read_text()))
    existing_vms = crud.virtual_mcp.servers.get_server(
        db_session, vms_upsert.name, throw_error_if_not_found=False
    )
    if existing_vms is None:
        return create_helper(db_session, vms_upsert, skip_dry_run)
    else:
        return update_helper(
            db_session,
            existing_vms,
            vms_upsert,
            skip_dry_run,
        )


def create_helper(
    db_session: Session, vms_upsert: VirtualMCPServerUpsert, skip_dry_run: bool
) -> UUID:
    # Create the mcp server entry in the database
    vms = crud.virtual_mcp.servers.create_server(db_session, vms_upsert)

    if not skip_dry_run:
        console.rule(
            f"Provide [bold green]--skip-dry-run[/bold green] to create VirtualMCPServer={vms.name}"
        )
        db_session.rollback()
    else:
        db_session.commit()
        console.rule(f"Created VirtualMCPServer={vms.name}")

    return vms.id


def update_helper(
    db_session: Session,
    existing_vms: VirtualMCPServer,
    vms_upsert: VirtualMCPServerUpsert,
    skip_dry_run: bool,
) -> UUID:
    existing_vms_upsert = VirtualMCPServerUpsert.model_validate(existing_vms, from_attributes=True)
    if existing_vms_upsert == vms_upsert:
        console.rule(f"VirtualMCPServer={existing_vms.name} exists and is up to date")
        return existing_vms.id
    else:
        console.rule(f"VirtualMCPServer={existing_vms.name} exists and will be updated")

    updated_vms = crud.virtual_mcp.servers.update_server(db_session, existing_vms, vms_upsert)

    diff = DeepDiff(existing_vms_upsert.model_dump(), vms_upsert.model_dump(), ignore_order=True)

    if not skip_dry_run:
        console.rule(
            f"Provide [bold green]--skip-dry-run[/bold green] to update "
            f"VirtualMCPServer={existing_vms.name} with the following changes:"
        )
        db_session.rollback()
    else:
        db_session.commit()
        console.rule(f"Updated VirtualMCPServer={existing_vms.name}")

    console.print(diff.pretty())

    return updated_vms.id
