from goose.toolkit.base import Toolkit, tool
from exchange import Message


class IO(Toolkit):
    """Provides tools to control mouse and keyboard inputs."""

    def __init__(self, *args: object, **kwargs: dict[str, object]) -> None:
        super().__init__(*args, **kwargs)
        import pyautogui
        self.pyautogui = pyautogui
        self.screen_width, self.screen_height = self.get_screen_info().values()

    @tool
    def get_screen_info(self):
        """Return the current screen's width and height."""
        width, height = self.pyautogui.size()
        return {'width': width, 'height': height}

    @tool
    def move_mouse(self, x: int, y: int) -> str:
        """
        Move the mouse cursor to the specified (x, y) coordinates.

        Args:
            x (int): The x-coordinate to move the mouse to.
            y (int): The y-coordinate to move the mouse to.

        Return:
            (str) a message indicating the mouse has been moved.
        """

        self.pyautogui.moveTo(x, y)
        return f"Mouse moved to ({x}, {y})"

    @tool
    def click_mouse(self) -> str:
        """
        Perform a mouse click at the current cursor position.

        Return:
            (str) a message indicating the mouse has been clicked.
        """

        self.pyautogui.click()
        return "Mouse clicked"

    @tool
    def right_click_mouse(self) -> str:
        """
        Perform a right mouse click at the current cursor position.

        Return:
            (str) a message indicating the mouse has been right-clicked.
        """

        self.pyautogui.click(button="right")
        return "Mouse right-clicked"

    @tool
    def type_text(self, text: str) -> str:
        """
        Type the given text using the keyboard.

        Args:
            text (str): The text to type.

        Return:
            (str) a message indicating the text has been typed.
        """

        self.pyautogui.write(text)
        return f"Typed text: {text}"

    @tool
    def press(self, key: str) -> str:
        """
        Press a key on the keyboard.

        Args:
            key (str): The key to press.

        Return:
            (str) a message indicating the key has been pressed.
        """

        self.pyautogui.press(key)
        return f"Key {key} pressed"

    @tool
    def press_while_holding(self, keys: [str], hold_key: str) -> str:
        """
        Press a key while holding another key.

        Args:
            keys ([str]): The key to press.
            hold_key (str): The key to hold while pressing the key.

        Return:
            (str) a message indicating the key has been pressed while holding another key.
        """

        with self.pyautogui.hold(hold_key):
            self.pyautogui.press(keys)
        return f"Key {keys} pressed while holding {hold_key}"

    @tool
    def scroll(self, clicks: int, x: int = None, y: int = None) -> str:
        """
        Scroll the mouse wheel.

        Args:
            clicks (int): The number of clicks to scroll.
            x (int, optional): The x-coordinate to scroll at.
            y (int, optional): The y-coordinate to scroll at.

        Return:
            (str) a message indicating the scroll action.
        """

        self.pyautogui.scroll(clicks, x, y)
        return f"Scrolled {clicks} clicks at ({x}, {y})"

    @tool
    def locate_on_screen(self, image: str) -> str:
        """
        Locate an image on the screen.

        Args:
            image (str): The file path to the image to locate.

        Return:
            (str) a message indicating whether the image was found and its position.
        """

        location = pyautogui.locateOnScreen(image)
        if location:
            return f"Image found at {location}"
        else:
            return "Image not found on screen"

    @tool
    def locate_all_on_screen(self, image: str) -> str:
        """
        Locate all instances of an image on the screen.

        Args:
            image (str): The file path to the image to locate.

        Return:
            (str) a message indicating the positions of all instances found.
        """

        locations = self.pyautogui.locateAllOnScreen(image)
        locations_list = list(locations)
        if locations_list:
            return f"Image found at {locations_list}"
        else:
            return "No instances of the image found on screen"

    @tool
    def scale_to_resolution(self, x: int, y: int, resolution: tuple[int, int]) -> tuple[int, int]:
        """Map coordinates from original resolution to the current screen resolution.

        Args:
            x (int): The x-coordinate to scale.
            y (int): The y-coordinate to scale.
            resolution (tuple[int, int]): The original resolution to scale from.

        Return:
            (tuple[int, int]) the scaled coordinates.
        """
        scale_x = self.screen_width / resolution[0]
        scale_y = self.screen_height / resolution[1]
        new_x = int(x * scale_x)
        new_y = int(y * scale_y)
        return new_x, new_y

      # Provide any system instructions for the model
    # This can be generated dynamically, and is run at startup time
    def system(self) -> str:
        return Message.load("prompts/io.jinja").text
