import importlib.util
import os
import random
import shutil
import subprocess
import sys
import time
from typing import Callable

# Windows-specific import
# if sys.platform.startswith("win"):
#     import winreg

# Check and install selenium if not installed
if importlib.util.find_spec("selenium") is None:
    subprocess.check_call(["python", "-m", "pip", "install", "selenium"])
from bs4 import BeautifulSoup
from exchange import Message
from pyshadow.main import Shadow
from selenium import webdriver
from selenium.common.exceptions import InvalidSessionIdException, NoSuchElementException, TimeoutException
from selenium.webdriver.common.by import By
from selenium.webdriver.common.action_chains import ActionChains
from selenium.webdriver.common.keys import Keys
from selenium.webdriver.support import expected_conditions as ec
from selenium.webdriver.support.ui import WebDriverWait

from goose.toolkit.base import Toolkit, tool


class BrowserToolkit(Toolkit):
    """A toolkit for interacting with web browsers using Selenium."""

    def __init__(self, *args: object, **kwargs: dict[str, object]) -> None:
        super().__init__(*args, **kwargs)
        self.driver = None
        self.history = []
        self.session_dir = ".goose/browsing_session"
        os.makedirs(self.session_dir, exist_ok=True)
        self.cached_url = ""

    def _initialize_driver(self, force_restart: bool = False, mock_driver: object = None) -> None:
        """Initialize the web driver if not already initialized or if a restart is forced."""
        if self.driver is None or force_restart:
            if mock_driver:
                self.driver = mock_driver
                return
            if self.driver is not None:
                try:
                    self.driver.quit()
                    self.notifier.notify("Previous browser session closed.")
                except Exception as e:
                    self.notifier.notify(f"Error closing previous session: {str(e)}")
            self.driver = None
            subprocess.run(["pkill", "-f", "webdriver"])  # Attempt to close all previous browser instances
            self.notifier.notify("All previous browser instances terminated.")
            if self.driver is not None:
                try:
                    self.driver.quit()
                except Exception as e:
                    self.notifier.notify(f"Error closing driver: {str(e)}")

            browser_name = self._get_default_browser()

            try:
                if "chrome" in browser_name.lower():
                    options = webdriver.ChromeOptions()
                    self.driver = webdriver.Chrome(options=options)
                elif "firefox" in browser_name.lower():
                    self.driver = webdriver.Firefox()
                else:
                    self.driver = webdriver.Firefox()

                try:
                    self.driver.set_window_size(835, 1024)
                except Exception:
                    pass  # Ignore window sizing errors if they occur
            except Exception as e:
                self.notifier.notify(f"Failed to initialize browser driver: {str(e)}")
                self.notifier.notify("Falling back to Firefox.")
                self.driver = webdriver.Firefox()

    def _get_default_browser(self) -> str:
        return get_default_browser()

    def system(self) -> str:
        return Message.load("prompts/browser.jinja").text

    def safe_execute(self, func: Callable, *args: object, **kwargs: dict[str, object]) -> object:
        """Safely execute a browser action, restart the driver if needed."""
        try:
            return func(*args, **kwargs)
        except (TimeoutException, NoSuchElementException, InvalidSessionIdException, Exception) as e:
            self.notifier.notify(f"Error during browser action: {str(e)}")
            self._initialize_driver(force_restart=True)
            return func(*args, **kwargs)

    @tool
    def navigate_to(self, url: str) -> None:
        """Navigate or browse to a specified URL in the browser.

        Args:
            url (str): The URL to navigate to.
        """
        self._initialize_driver()
        self.notifier.notify(f"Navigating to {url}")
        self.safe_execute(self.driver.get, url)
        self.wait_for_page_load()
        self.history.append(url)

    @tool
    def take_browser_screenshot(self, filename: str) -> str:
        """Take a screenshot of the current browser window to assist with navigation.

        Args:
            filename (str): The file path where the screenshot will be saved.
        """
        try:
            path = os.path.join(self.session_dir, filename)
            self.driver.save_screenshot(path)
            self.notifier.notify(f"Screenshot saved in browsing session: {path}")
            return f"image:{path}"
        except Exception as e:
            self.notifier.notify(f"Error taking screenshot: {str(e)}")

    @tool
    def scroll_page(self, direction: str = "down") -> None:
        """Scroll the current page up or down.

        Args:
            direction (str): The direction to scroll the page. Either 'up' or 'down'.
        """
        actions = ActionChains(self.driver)
        if direction == "up":
            actions.send_keys(Keys.PAGE_UP).perform()
        elif direction == "down":
            actions.send_keys(Keys.PAGE_DOWN).perform()
        else:
            self.notifier.notify(f"Invalid scroll direction: {direction}")

    @tool
    def open_new_tab(self, url: str) -> None:
        """Open a new tab and navigate to the specified URL.

        Args:
            url (str): The URL to navigate to in the new tab.
        """
        if not self.driver:
            self.notifier.notify("Driver not initialized, using navigate_to instead.")
            self.navigate_to(url)
            return

        self.notifier.notify(f"Opening a new tab and navigating to {url}.")
        self.driver.execute_script(f"window.open('{url}', '_blank');")
        self.driver.switch_to.window(self.driver.window_handles[-1])
        self.wait_for_page_load()

    @tool
    def check_current_page_url(self) -> str:
        """Get the URL of the current page."""
        if not self.driver:
            self.notifier.notify("Driver is not initialized.")
            return ""

        current_url = self.driver.current_url
        self.notifier.notify(f"Current page URL: {current_url}")
        return current_url

    @tool
    def switch_to_tab(self, index: int) -> None:
        """Switch to the browser tab at the specified index.

        Args:
            index (int): The index of the tab to switch to.
        """
        try:
            self.notifier.notify(f"Switching to tab at index {index}.")
            self.driver.switch_to.window(self.driver.window_handles[index])
            self.wait_for_page_load()
        except IndexError:
            self.notifier.notify(f"Invalid tab index: {index}.")

    @tool
    def close_current_tab(self) -> None:
        """Close the current browser tab."""
        if not self.driver:
            self.notifier.notify("Cannot close the tab as the driver is not initialized.")
            return

        self.notifier.notify("Closing the current tab.")
        self.driver.close()
        if len(self.driver.window_handles) > 0:
            self.driver.switch_to.window(self.driver.window_handles[-1])

    def refresh_page(self) -> None:
        """Refresh the current browser page."""
        self.notifier.notify("Refreshing the current page.")
        self.driver.refresh()
        self.wait_for_page_load()

    @tool
    def get_html_content(self) -> str:
        """Extract the full HTML content of the current page and cache it to a file."""
        self.notifier.notify("Extracting full HTML content of the page.")
        current_url = self.driver.current_url.replace("https://", "").replace("http://", "").replace("/", "_")

        if current_url != self.cached_url:
            html_content = self.driver.page_source
            filename = os.path.join(self.session_dir, f"{current_url}_page.html")
            with open(filename, "w", encoding="utf-8") as f:
                f.write(html_content)
            self.cached_html_path = filename
            self.cached_url = current_url
            self.notifier.notify(f"HTML cached as {filename}.")

        return self.cached_html_path

    # @tool
    # def run_js(self, script: str) -> str:
    #     """Execute custom JavaScript on the page.
    #
    #     Args:
    #         script (str): JavaScript code to execute.
    #
    #     Returns:
    #         str: The result of the JavaScript execution.
    #     """
    #     self.notifier.notify("Running JavaScript in the browser.")
    #     return self.driver.execute_script(script)

    @tool
    def type_into_input(self, selector: str, text: str) -> None:
        """Type text into an input element specified by a CSS selector for the currently open page.

        Args:
            selector (str): CSS selector string to locate the input element.
            text (str): The text to type into the input element.
        """
        retries = 3
        for attempt in range(retries):
            try:
                self.notifier.notify(f"Typing '{text}' into input with selector: {selector}")
                element = WebDriverWait(self.driver, 20).until(ec.element_to_be_clickable((By.CSS_SELECTOR, selector)))
                element.clear()
                for char in text:
                    element.send_keys(char)
                    time.sleep(random.uniform(0.1, 0.3))
                break
            except TimeoutException as e:
                if attempt < retries - 1:
                    self.notifier.notify(f"Retry {attempt + 1}/{retries} due to timeout: {str(e)}")
                    time.sleep(2)
                else:
                    raise

    def wait_for_page_load(self, timeout: int = 45) -> None:
        """Wait for the page to fully load by checking the document readiness state.

        Args:
            timeout (int): Maximum time to wait for page load, in seconds.
        """
        WebDriverWait(self.driver, timeout).until(
            lambda driver: driver.execute_script("return document.readyState") == "complete"
        )
        self.notifier.notify("Page fully loaded.")

    @tool
    def click_element(self, selector: str) -> None:
        """Click a button or link specified by a CSS selector.

        Args:
            selector (str): CSS selector string to locate the element.
        """
        retries = 3
        for attempt in range(retries):
            try:
                self.notifier.notify(f"Clicking element with selector: {selector}")
                element = WebDriverWait(self.driver, 20).until(ec.element_to_be_clickable((By.CSS_SELECTOR, selector)))
                element.click()
                self.wait_for_page_load()
                break
            except TimeoutException as e:
                if attempt < retries - 1:
                    self.notifier.notify(f"Retry {attempt + 1}/{retries} due to timeout: {str(e)}")
                    time.sleep(2)
                else:
                    raise

    @tool
    def click_element_by_link_text(self, link_text: str, exact_match: bool = True) -> None:
        """Click on a page element using the text visible on the page.
        Useful when the page has multiple links or buttons, and you want to click on a specific one.

        Args:
            link_text (str): The visible text of the button or link.
            exact_match (bool): Whether to match the exact link text or any partial match.
        """
        self.notifier.notify(f"Clicking element with text: {link_text}")
        match_type = By.LINK_TEXT if exact_match else By.PARTIAL_LINK_TEXT
        element = self.driver.find_element(match_type, link_text)
        element.click()

    @tool
    def find_element_by_text_soup(self, text: str, filename: str) -> str:
        """Find an element containing the specified text using BeautifulSoup on HTML content stored in a file.
        If not found, fallback to Shadow DOM search using PyShadow.

        Args:
            text (str): The text content to find within an element.
            filename (str): The name of the file containing the HTML content.

        """
        # Search using BeautifulSoup as previously implemented
        try:
            with open(filename, "r", encoding="utf-8") as file:
                soup = BeautifulSoup(file, "html.parser")
                element = soup.find(
                    lambda tag: (tag.string and text in tag.string)
                    or (tag.get_text() and text in tag.get_text())
                    or (tag.has_attr("title") and text in tag["title"])
                    or (tag.has_attr("alt") and text in tag["alt"])
                    or (tag.has_attr("aria-label") and text in tag["aria-label"])
                )

                if element:
                    self.notifier.notify(f"Element found with text: {text}")
                    return str(element)
        except FileNotFoundError:
            self.notifier.notify(f"File not found: {filename}")
            return None

        # Fallback: search using PyShadow
        try:
            shadow = Shadow(self.driver)
            shadow_element = shadow.find_element_by_xpath(f"//*[contains(text(), '{text}')]")
            if shadow_element:
                self.notifier.notify(f"Element found in shadow DOM with text: {text}")
                return shadow_element.get_attribute("outerHTML")
        except Exception as e:
            self.notifier.notify(f"Error searching in shadow DOM: {str(e)}")

        self.notifier.notify(f"Element not found with text: {text} in either DOMs")
        return None

    @tool
    def find_elements_of_type(self, tag_type: str, filename: str) -> list[str]:
        """Find all elements of a specific tag type using BeautifulSoup on HTML content stored in a file.

        Args:
            tag_type (str): The HTML tag type to search for.
            filename (str): The name of the file containing the HTML content.
        """
        elements_as_strings = []
        try:
            with open(filename, "r", encoding="utf-8") as file:
                soup = BeautifulSoup(file, "html.parser")
                elements = soup.find_all(tag_type)
                elements_as_strings = [str(element) for element in elements]
                self.notifier.notify(f"Found {len(elements_as_strings)} elements of type: {tag_type}")
        except FileNotFoundError:
            self.notifier.notify(f"File not found: {filename}")
        return elements_as_strings

    def __del__(self) -> None:
        # Remove the entire session directory
        if os.path.exists(self.session_dir):
            try:
                shutil.rmtree(self.session_dir)
                self.notifier.notify(f"Removed browsing session directory: {self.session_dir}")
            except OSError as e:
                self.notifier.notify(f"Error removing session directory: {str(e)}")

        if self.driver:
            self.driver.quit()


# def get_default_browser_windows() -> str:
#     try:
#         with winreg.OpenKey(
#             winreg.HKEY_CURRENT_USER, r"Software\Microsoft\Windows\Shell\Associations\UrlAssociations\http\UserChoice"
#         ) as key:
#             prog_id, _ = winreg.QueryValueEx(key, "ProgId")
#
#         with winreg.OpenKey(winreg.HKEY_CLASSES_ROOT, f"{prog_id}\\shell\\open\\command") as cmd_key:
#             command, _ = winreg.QueryValueEx(cmd_key, None)
#
#         if command.startswith('"'):
#             executable = command.split('"')[1]
#         else:
#             executable = command.split(" ")[0]
#
#         return os.path.basename(executable)
#
#     except Exception as e:
#         print(f"Error retrieving default browser on Windows: {e}")
#         return None


def get_default_browser_macos() -> str:
    try:
        import os
        import plistlib

        plist_path = os.path.expanduser(
            "~/Library/Preferences/com.apple.LaunchServices/com.apple.launchservices.secure.plist"
        )

        if not os.path.exists(plist_path):
            print(f"Launch services plist not found at: {plist_path}")
            return None

        with open(plist_path, "rb") as fp:
            plist = plistlib.load(fp)
            handlers = plist.get("LSHandlers", [])

            for handler in handlers:
                scheme = handler.get("LSHandlerURLScheme")
                if scheme and scheme.lower() == "http":
                    return handler.get("LSHandlerRoleAll")

        return None
    except Exception as e:
        print(f"Error retrieving default browser on macOS: {e}")
        return None


# def get_default_browser_linux() -> str:
#     try:
#         result = subprocess.run(
#             ["xdg-settings", "get", "default-web-browser"], stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True
#         )
#
#         if result.returncode != 0:
#             print(f"Error: {result.stderr.strip()}")
#             return None
#
#         desktop_file = result.stdout.strip()
#         desktop_paths = [
#             os.path.expanduser("~/.local/share/applications/"),
#             "/usr/share/applications/",
#             "/usr/local/share/applications/",
#         ]
#
#         for path in desktop_paths:
#             desktop_file_path = os.path.join(path, desktop_file)
#             if os.path.exists(desktop_file_path):
#                 with open(desktop_file_path, "r") as f:
#                     for line in f:
#                         if line.startswith("Name="):
#                             name = line.split("=", 1)[1].strip()
#                             return name
#         return desktop_file.replace(".desktop", "")
#
#     except Exception as e:
#         print(f"Error retrieving default browser on Linux: {e}")
#         return None


def get_default_browser() -> str:
    if sys.platform.startswith("darwin"):
        return get_default_browser_macos()
    # other platforms are not enabled yet.
    # elif sys.platform.startswith("win"):
    #     return get_default_browser_windows()
    # elif sys.platform.startswith("linux"):
    #     return get_default_browser_linux()
    else:
        print(f"Unsupported platform {sys.platform}")
        return None
        return None
