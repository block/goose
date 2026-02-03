import os
import pytest

from zenguard.zenguard import Credentials, ZenGuard, ZenGuardConfig


@pytest.fixture(scope="session")
def zen_api_key():
    api_key = os.environ.get("ZEN_API_KEY")
    assert api_key, "ZEN_API_KEY is not set"
    return api_key
    

@pytest.fixture(scope="module")
def zenguard(zen_api_key):
    config = ZenGuardConfig(credentials=Credentials(api_key=zen_api_key))
    return ZenGuard(config=config)
