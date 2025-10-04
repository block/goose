"""Optional extension system for advanced users"""

from abc import ABC, abstractmethod
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from ..client import GooseClient


class Extension(ABC):
    """Base class for extensions"""

    def __init__(self, client: "GooseClient"):
        """Initialize the extension with a client instance.

        Args:
            client: The GooseClient instance to extend
        """
        self.client = client

    @abstractmethod
    def install(self):
        """Install the extension.

        This method should modify the client instance to add
        the extension's functionality.
        """
        pass

    def uninstall(self):
        """Uninstall the extension.

        Optional method to remove the extension's modifications.
        Default implementation does nothing.
        """
        pass
