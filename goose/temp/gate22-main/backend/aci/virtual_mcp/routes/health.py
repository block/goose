from fastapi import APIRouter

from aci.common.logging_setup import get_logger

logger = get_logger(__name__)
router = APIRouter()


# TODO: add more checks?
@router.get("", include_in_schema=False)
async def health() -> bool:
    return True
