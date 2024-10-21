from exchange import Message

from goose.toolkit.base import Toolkit


class Focus(Toolkit):
    """Provides a prompt on how to change the Goose context"""

    def system(self) -> str:
        result = Message.load("prompts/focus.jinja").text
        print(result)
        return result
