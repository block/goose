"""first version of complete schemas

Revision ID: 94b423b00cf5
Revises:
Create Date: 2025-08-25 10:03:04.823304+00:00

"""
from typing import Sequence, Union

from alembic import op


# revision identifiers, used by Alembic.
revision: str = '94b423b00cf5'
down_revision: Union[str, None] = None
branch_labels: Union[str, Sequence[str], None] = None
depends_on: Union[str, Sequence[str], None] = None


def upgrade() -> None:
    # create extension if not exists vector;
    op.execute("CREATE EXTENSION IF NOT EXISTS vector;")


def downgrade() -> None:
    # drop extentions
    op.execute("DROP EXTENSION IF EXISTS vector;")
