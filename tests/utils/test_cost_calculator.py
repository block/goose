from datetime import datetime, timezone
from unittest.mock import MagicMock, patch

import pytest
from exchange.providers.base import Usage
from goose.utils._cost_calculator import _calculate_cost, get_total_cost_message

SESSION_NAME = "test_session"
START_TIME = datetime(2024, 10, 20, 1, 2, 3, tzinfo=timezone.utc)
END_TIME = datetime(2024, 10, 21, 2, 3, 4, tzinfo=timezone.utc)


@pytest.fixture
def start_time():
    mock_start_time = MagicMock(spec=datetime)
    mock_start_time.astimezone.return_value = START_TIME
    return mock_start_time


@pytest.fixture
def end_time():
    mock_end_time = MagicMock(spec=datetime)
    mock_end_time.astimezone.return_value = END_TIME
    return mock_end_time


@pytest.fixture
def mock_prices():
    prices = {"gpt-4o": (5.00, 15.00), "gpt-4o-mini": (0.150, 0.600)}
    with patch("goose.utils._cost_calculator.PRICES", prices) as mock_prices:
        yield mock_prices


def test_calculate_cost(mock_prices):
    cost = _calculate_cost("gpt-4o", Usage(input_tokens=10000, output_tokens=600, total_tokens=10600))
    assert cost == 0.059


def test_get_total_cost_message(mock_prices, start_time, end_time):
    message = get_total_cost_message(
        {
            "gpt-4o": Usage(input_tokens=10000, output_tokens=600, total_tokens=10600),
            "gpt-4o-mini": Usage(input_tokens=3000000, output_tokens=4000000, total_tokens=7000000),
        },
        SESSION_NAME,
        start_time,
        end_time,
    )
    expected_message = (
        "Session name: test_session | Cost for model gpt-4o Usage(input_tokens=10000, output_tokens=600,"
        " total_tokens=10600): $0.06\n"
        "Session name: test_session | Cost for model gpt-4o-mini Usage(input_tokens=3000000, output_tokens=4000000, "
        "total_tokens=7000000): $2.85\n"
        "2024-10-20T01:02:03+00:00 - 2024-10-21T02:03:04+00:00 | Session name: test_session | Total cost: $2.91"
    )
    assert message == expected_message


def test_get_total_cost_message_with_non_available_pricing(mock_prices, start_time, end_time):
    message = get_total_cost_message(
        {
            "non_pricing_model": Usage(input_tokens=10000, output_tokens=600, total_tokens=10600),
            "gpt-4o-mini": Usage(input_tokens=3000000, output_tokens=4000000, total_tokens=7000000),
        },
        SESSION_NAME,
        start_time,
        end_time,
    )
    expected_message = (
        "Session name: test_session | Cost for model non_pricing_model Usage(input_tokens=10000, output_tokens=600,"
        " total_tokens=10600): Not available\n"
        + "Session name: test_session | Cost for model gpt-4o-mini Usage(input_tokens=3000000, output_tokens=4000000,"
        " total_tokens=7000000): $2.85\n"
        + "2024-10-20T01:02:03+00:00 - 2024-10-21T02:03:04+00:00 | Session name: test_session | Total cost: $2.85"
    )
    assert message == expected_message
