import pytest
from unittest.mock import MagicMock
from goose.toolkit.web_browser import BrowserToolkit


# Mock the webdriver
@pytest.fixture
def mock_driver(mocker):
    mocker.patch("selenium.webdriver.Chrome")
    mocker.patch("selenium.webdriver.Firefox")

    driver_mock = MagicMock()

    mocker.patch.object(BrowserToolkit, "_initialize_driver", return_value=None)

    return driver_mock


def test_html_content_extraction(mock_driver):
    mock_notifier = MagicMock()
    toolkit = BrowserToolkit(notifier=mock_notifier)
    toolkit.driver = mock_driver
    mock_driver.current_url = "http://example.com"
    mock_driver.page_source = "<html><head></head><body>TestPage</body></html>"

    cached_html_path = toolkit.get_html_content()

    # Read from the cached HTML file and assert its content
    with open(cached_html_path, "r", encoding="utf-8") as file:
        html_content = file.read()

    assert html_content == "<html><head></head><body>TestPage</body></html>"
