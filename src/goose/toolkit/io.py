import os
import uuid
import shutil

from goose.toolkit.base import Toolkit, tool
from exchange import Message
from PIL import Image
import pyautogui

class IO(Toolkit):
    """Provides tools to control mouse and keyboard inputs."""

    def __init__(self, *args: object, **kwargs: dict[str, object]) -> None:
        super().__init__(*args, **kwargs)
        self.pyautogui = pyautogui
        self.screen_width, self.screen_height = self.get_screen_info().values()
        self.session_dir = os.path.expanduser(".goose/screenshots")
        if not os.path.exists(self.session_dir):
            os.makedirs(self.session_dir)

    def __del__(self):
        # Remove the entire screenshot directory
        if os.path.exists(self.session_dir):
            try:
                shutil.rmtree(self.session_dir)
                self.notifier.log(f"Removed browsing session directory: {self.session_dir}")
            except OSError as e:
                self.notifier.log(f"Error removing session directory: {str(e)}")

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
            (str) A message indicating the mouse has been moved.
        """
        self.pyautogui.moveTo(x, y)
        return f"Mouse moved to ({x}, {y})"

    @tool
    def click_mouse(self) -> str:
        """
        Perform a mouse click at the current cursor position.

        Return:
            (str) A message indicating the mouse has been clicked.
        """
        self.pyautogui.click()
        return "Mouse clicked"

    @tool
    def right_click_mouse(self) -> str:
        """
        Perform a right mouse click at the current cursor position.

        Return:
            (str) A message indicating the mouse has been right-clicked.
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
            (str) A message indicating the text has been typed.
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
            (str) A message indicating the key has been pressed.
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
            (str) A message indicating the key has been pressed while holding another key.
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
            (str) A message indicating the scroll action.
        """
        self.pyautogui.scroll(clicks, x, y)
        return f"Scrolled {clicks} clicks at ({x}, {y})"

    @tool
    def scale_to_resolution(self, x: int, y: int, resolution: tuple[int, int]) -> tuple[int, int]:
        """
        Map coordinates from original resolution to the current screen resolution.

        Args:
            x (int): The x-coordinate to scale.
            y (int): The y-coordinate to scale.
            resolution (tuple[int, int]): The original resolution to scale from.

        Return:
            (tuple[int, int]) The scaled coordinates.
        """
        scale_x = self.screen_width / resolution[0]
        scale_y = self.screen_height / resolution[1]
        new_x = int(x * scale_x)
        new_y = int(y * scale_y)
        return new_x, new_y

    @tool
    def view_image(self, image_path: str) -> str:
        """
        Allows to view any image

        Args:
            image_path (str): The file path to the image to open.

        Return:
            (str) A message indicating the image has been opened.
        """
        return f"image:{image_path}"

    @tool
    def take_and_resize_screenshot(self, max_size_mb: int = 5) -> str:
        """
        Take a screenshot and resize it to ensure it's under 5MB,
        maintaining aspect ratio.

        Args:
            max_size_mb (int): Maximum size of the screenshot in MB. Default is 5MB.
        """
        
        filename = os.path.join(self.session_dir, f"goose_screenshot_{uuid.uuid4().hex}.jpg")

        # Take a screenshot and convert to RGB
        screenshot = self.pyautogui.screenshot()
        screenshot = screenshot.convert('RGB')
        screenshot.save(filename)

        # Open the image using Pillow
        img = Image.open(filename)

        # Calculate the maximum acceptable file size in bytes
        max_size_bytes = max_size_mb * 1024 * 1024

        # If the image size is greater than max_size_bytes, compress the image
        quality = 90
        while os.path.getsize(filename) > max_size_bytes and quality > 10:
            img.save(filename, 'JPEG', quality=quality)
            quality -= 10

        # Confirm the resulting image is smaller than the max size
        if os.path.getsize(filename) > max_size_bytes:
            raise Exception("Unable to reduce image size below the specified maximum.")

        print(f"Final screenshot saved: {filename}")
        return f"image:{filename}"

    @tool
    def take_screenshot_and_crop(self, area_of_interest, save_name, max_size_mb=5, image_path=None):
        """
        Take a screenshot (or use an existing image), crop a specified area, 
        and return it along with the pixel coordinates of the cropped area 
        in the original screen size.

        Args:
            area_of_interest (tuple): A tuple (left, upper, right, lower) indicating the area to crop.
            save_name (str): The name of the file to save the cropped image.
            max_size_mb (int): The maximum acceptable size of the cropped image in megabytes.
            image_path (str, optional): Path to an existing image file to be cropped.

        Returns:
            (tuple): Returns the cropped image path and a tuple with pixel coordinates of the cropped area (left, upper, right, lower) relative to the full screenshot or provided image.

        Raises:
            Exception: If the cropped image exceeds the specified maximum size.
        """
        if image_path:
            # Use an existing image
            full_screenshot = Image.open(image_path)
        else:
            # Take a new screenshot of the entire screen
            full_screenshot = self.pyautogui.screenshot()

        save_path = os.path.join(self.session_dir, save_name)

        # Crop the specified area of interest and convert to RGB
        cropped_img = full_screenshot.crop(area_of_interest)
        cropped_img = cropped_img.convert('RGB')
        cropped_img.save(save_path)

        # Check the size of the cropped image
        max_size_bytes = max_size_mb * 1024 * 1024
        if os.path.getsize(save_path) > max_size_bytes:
            raise Exception(f"Cropped image exceeds the maximum size of {max_size_mb}MB")

        # Return the cropped image and cropping offsets
        return save_path, area_of_interest

    # Provide any system instructions for the model
    # This can be generated dynamically, and is run at startup time
    def system(self) -> str:
        return Message.load("prompts/io.jinja").text
