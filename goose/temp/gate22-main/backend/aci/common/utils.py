import datetime
import os
import re
import secrets
import string
from functools import cache
from uuid import UUID

import bcrypt
import humanize
import jwt
from sqlalchemy import Engine, create_engine
from sqlalchemy.orm import Session, sessionmaker

from aci.common.db.sql_models import User
from aci.common.schemas.auth import (
    ActAsInfo,
    JWTPayload,
)


def check_and_get_env_variable(name: str, default: str | None = None) -> str:
    value = os.getenv(name)
    if value is None:
        if default is None:
            raise ValueError(f"Environment variable '{name}' is not set")
        return default
    if value == "":
        if default is None:
            raise ValueError(f"Environment variable '{name}' is empty string")
        return default
    return value


# TODO: from coderabbit review: construct_db_url doesn not escape special chars in user/password
# (e.g., @:/). This will break connections.
# from urllib.parse import quote_plus
# return f"{scheme}://{quote_plus(user)}:{quote_plus(password)}@{host}:{port}/{db_name}"
def construct_db_url(
    scheme: str, user: str, password: str, host: str, port: str, db_name: str
) -> str:
    return f"{scheme}://{user}:{password}@{host}:{port}/{db_name}"


def format_to_screaming_snake_case(name: str) -> str:
    """
    Convert a string with spaces, hyphens, slashes, camel case etc. to screaming snake case.
    e.g., "GitHub Create Repository" -> "GITHUB_CREATE_REPOSITORY"
    e.g., "GitHub/Create Repository" -> "GITHUB_CREATE_REPOSITORY"
    e.g., "github-create-repository" -> "GITHUB_CREATE_REPOSITORY"
    """
    name = re.sub(r"[\W]+", "_", name)  # Replace non-alphanumeric characters with underscore
    s1 = re.sub("(.)([A-Z][a-z]+)", r"\1_\2", name)
    s2 = re.sub("([a-z0-9])([A-Z])", r"\1_\2", s1)
    s3 = s2.replace("-", "_").replace("/", "_").replace(" ", "_")
    s3 = re.sub("_+", "_", s3)  # Replace multiple underscores with single underscore
    s4 = s3.upper().strip("_")

    return s4


# NOTE: it's important that you don't create a new engine for each session, which takes
# up db resources and will lead up to errors pretty fast
# TODO: fine tune the pool settings
# TODO: use async
@cache
def get_db_engine(db_url: str) -> Engine:
    return create_engine(
        db_url,
        pool_size=10,
        max_overflow=20,
        pool_timeout=1,  # Fail fast if pool exhausted (1s timeout)
        pool_recycle=3600,  # recycle connections after 1 hour
        pool_pre_ping=True,
    )


# NOTE: cache this because only one sessionmaker is needed for all db sessions
@cache
def get_sessionmaker(db_url: str) -> sessionmaker:
    engine = get_db_engine(db_url)
    return sessionmaker(autocommit=False, autoflush=False, bind=engine)


def create_db_session(db_url: str) -> Session:
    SessionMaker = get_sessionmaker(db_url)
    session: Session = SessionMaker()
    return session


def parse_mcp_server_name_from_mcp_tool_name(mcp_tool_name: str) -> str:
    """
    Parse the mcp server name from a mcp tool name.
    e.g., "ACI_TEST__HELLO_WORLD" -> "ACI_TEST"
    """
    return mcp_tool_name.split("__")[0]


def is_uuid(value: str | UUID) -> bool:
    if isinstance(value, UUID):
        return True
    try:
        UUID(value)
        return True
    except ValueError:
        return False


def hash_user_password(password: str) -> str:
    salt = bcrypt.gensalt()
    hashed = bcrypt.hashpw(password.encode(), salt)
    return hashed.decode()


def sign_token(
    user: User,
    act_as: ActAsInfo | None,
    jwt_signing_key: str,
    jwt_algorithm: str,
    jwt_access_token_expire_minutes: int,
) -> str:
    """
    Sign a JWT token for the user. It should include act_as information.
    """
    now = datetime.datetime.now(datetime.UTC)
    expired_at = now + datetime.timedelta(minutes=jwt_access_token_expire_minutes)
    jwt_payload = JWTPayload(
        sub=str(user.id),
        exp=int(expired_at.timestamp()),
        iat=int(now.timestamp()),
        user_id=user.id,
        name=user.name,
        email=user.email,
        act_as=act_as,
    )
    # Sign JWT, with the user's acted as organization and role
    token = jwt.encode(
        jwt_payload.model_dump(mode="json"), jwt_signing_key, algorithm=jwt_algorithm
    )
    return token


def format_duration_from_minutes(minutes: int) -> str:
    """
    Convert a duration in minutes into a friendly human-readable label.
    Uses the humanize library for consistent and localized formatting.
    """
    delta = datetime.timedelta(minutes=minutes)
    return humanize.naturaldelta(delta)


def generate_alphanumeric_string(
    length: int, character_pool: str = string.ascii_letters + string.digits
) -> str:
    """
    Generate a random alphanumeric string of a given length.
    Convenient to call with customized `character_pool`, examples:
        "ABCDE" / string.digits + string.ascii_uppercase / string.ascii_letters
    """
    return "".join(secrets.choice(character_pool) for _ in range(length))
