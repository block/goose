from collections.abc import Generator
from typing import cast

import pytest
from fastapi.testclient import TestClient
from sqlalchemy import Inspector, inspect
from sqlalchemy.orm import Session

from aci.common.db.sql_models import Base
from aci.common.test_utils import clear_database, create_test_db_session
from aci.mcp import config
from aci.mcp.main import app as fastapi_app


@pytest.fixture(scope="function")
def db_session() -> Generator[Session, None, None]:
    yield from create_test_db_session(config.DB_HOST, config.DB_FULL_URL)


@pytest.fixture(scope="function", autouse=True)
def database_setup_and_cleanup(db_session: Session) -> Generator[None, None, None]:
    """
    Setup and cleanup the database for each test case.
    """
    inspector = cast(Inspector, inspect(db_session.bind))

    # Check if all tables defined in models are created in the db
    for table in Base.metadata.tables.values():
        if not inspector.has_table(table.name):
            pytest.exit(f"Table {table} does not exist in the database.")

    clear_database(db_session)
    yield  # This allows the test to run
    clear_database(db_session)


@pytest.fixture(scope="function")
def test_client() -> Generator[TestClient, None, None]:
    with TestClient(fastapi_app) as client:
        yield client
