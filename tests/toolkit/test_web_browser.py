import pytest
from unittest.mock import MagicMock, patch
from selenium.webdriver.common.by import By
from selenium.webdriver.support.ui import WebDriverWait
from selenium.webdriver.support import expected_conditions as EC
from src.goose.toolkit.base import Toolkit
from src.goose.toolkit.web_browser import BrowserToolkit


# Mock the webdriver
@pytest.fixture
def mock_driver(mocker):
    mocker.patch('selenium.webdriver.Chrome')
    mocker.patch('selenium.webdriver.Firefox')

    driver_mock = MagicMock()
    
    mocker.patch.object(BrowserToolkit, '_initialize_driver', return_value=None)
    
    return driver_mock


def test_html_content_extraction(mock_driver):
    mock_notifier = MagicMock()
    toolkit = BrowserToolkit(notifier=mock_notifier)
    toolkit.driver = mock_driver
    mock_driver.page_source = '<html><head></head><body>Test Page</body></html>'

    html_content = toolkit.get_html_content()
    assert html_content == '<html><head></head><body>Test Page</body></html>'


def test_cookie_management(mock_driver):
    mock_notifier = MagicMock()
    toolkit = BrowserToolkit(notifier=mock_notifier)
    toolkit.driver = mock_driver

    # Test adding a cookie
    toolkit.manage_cookies('add', {'name': 'test_cookie', 'value': '123'})
    mock_driver.add_cookie.assert_called_once_with({'name': 'test_cookie', 'value': '123'})

    # Test getting cookies
    mock_driver.get_cookies.return_value = [{'name': 'test_cookie', 'value': '123'}]
    cookies = toolkit.manage_cookies('get')
    assert cookies == [{'name': 'test_cookie', 'value': '123'}]

    # Test deleting a cookie
    toolkit.manage_cookies('delete', {'name': 'test_cookie'})
    mock_driver.delete_cookie.assert_called_once_with('test_cookie')
