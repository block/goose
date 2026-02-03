from collections.abc import Generator

from fastapi import HTTPException, status
from sqlalchemy.exc import TimeoutError as SQLAlchemyTimeoutError
from sqlalchemy.orm import Session

from aci.common import utils
from aci.common.logging_setup import get_logger
from aci.mcp import config

logger = get_logger(__name__)


def yield_db_session() -> Generator[Session, None, None]:
    try:
        db_session = utils.create_db_session(config.DB_FULL_URL)
    except SQLAlchemyTimeoutError as e:
        logger.error(f"Timeout creating database session, likely pool exhausted, error={e}")
        raise HTTPException(
            status_code=status.HTTP_503_SERVICE_UNAVAILABLE,
            detail="Service temporarily unavailable",
        ) from e
    except Exception as e:
        logger.error(f"Failed to create database session, error={e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail="Internal server error",
        ) from e

    try:
        yield db_session
        db_session.commit()
    except Exception:
        db_session.rollback()
        raise
    finally:
        db_session.close()
