"""
ZenGuard is a class that represents the ZenGuard object. It is used to connect to ZenGuard AI API and its services.
"""

from dataclasses import dataclass
from enum import Enum
from typing import Optional

import httpx

BASE_TIER_ENDPOINT = "https://api.zenguard.ai"
DEDICATED_TIER_ENDPOINT = "https://dedicated-tier.zenguard.ai"
ZEN_IN_PATH = "/v2/zen/in"


@dataclass
class Credentials:
    """Authentication credentials for ZenGuard API."""

    api_key: str


class Tier(str, Enum):
    BASE = "base"
    DEDICATED = "dedicated"


@dataclass
class ZenGuardConfig:
    """Configuration for ZenGuard client.

    Args:
        credentials: Authentication credentials
        tier: API tier to use (base or dedicated)
    """

    credentials: Credentials
    tier: Optional[Tier] = Tier.BASE


class Detector(str, Enum):
    ALLOWED_TOPICS = "/v1/detect/topics/allowed"
    BANNED_TOPICS = "/v1/detect/topics/banned"
    PROMPT_INJECTION = "/v1/detect/prompt_injection"
    KEYWORDS = "/v1/detect/keywords"
    PII = "/v1/detect/pii"
    SECRETS = "/v1/detect/secrets"


class ZenGuard:
    """
    ZenGuard is a class that represents the ZenGuard object.
    It is used to connect to ZenGuard AI API and its services.
    """

    def __init__(self, config: ZenGuardConfig):
        """
        Initialize ZenGuard client.

        Args:
            config: Client configuration

        Raises:
            ValueError: If the API key is invalid
        """
        api_key = config.credentials.api_key
        if not isinstance(api_key, str) or not api_key:
            raise ValueError("The API key must be a string type and not empty.")

        self._api_key = api_key
        self._backend = (
            DEDICATED_TIER_ENDPOINT
            if config.tier == Tier.DEDICATED
            else BASE_TIER_ENDPOINT
        )

        self._client = httpx.Client(headers={"x-api-key": self._api_key}, timeout=10)

    def detect(self, detectors: list[Detector], prompt: str) -> dict:
        """
        Uses detectors to evaluate the prompt and return the results.

        Args:
            detector: The detector to use for evaluation
            prompt: The text to analyze

        Returns:
            dict: The API response containing detection results

        Raises:
            ValueError: If the prompt is empty or whitespace
            RuntimeError: If the API returns an error response
        """
        if not prompt or prompt.isspace():
            raise ValueError("Prompt can't be empty.")

        if detectors is None or len(detectors) == 0:
            raise ValueError("At least one detector must be provided.")

        if len(detectors) > 1 and self._backend == BASE_TIER_ENDPOINT:
            raise ValueError(
                "Multiple detectors are only supported for dedicated tier."
            )

        if len(detectors) == 1:
            url = f"{self._backend}{detectors[0].value}"
        else:
            url = f"{self._backend}{ZEN_IN_PATH}"

        try:
            response = self._client.post(
                url,
                json={"messages": [prompt]},
            )
            if response.status_code != 200:
                raise RuntimeError(
                    f"ZenGuard: Received an unexpected status code: {response.status_code}\nResponse content: {response.json()}"
                )
            return response.json()
        except httpx.RequestError as e:
            raise RuntimeError(
                f"ZenGuard: An error occurred while making the request: {str(e)}"
            ) from e
