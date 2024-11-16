import os
import platform

from exchange.content import ImageUrl, ToolResult
from exchange.exchange import Exchange
from exchange.message import Message
from exchange.providers.openai import OpenAiProvider
from goose.toolkit.base import Toolkit, tool

class VisionToolkit(Toolkit):
    """A toolkit for image analysis using AI capabilities."""

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)

    @tool
    def describe_image(self, image: str, instructions: str="Describe the image") -> str:
        """Analyze an image and return a description or other analysis based on the instructions.

        Args:
            image (ImageUrl): The URL of the image to analyze.
            instructions (str): Instructions for the AI on what kind of analysis to perform.
        """
        if image.startswith('data:'):
            print("Using base64 image")


        image = ImageUrl(url=image)
        user_message = Message(role="user", content=[f"{instructions}: ", image])
        ex = Exchange(
            provider=OpenAiProvider.from_env(),
            model="gpt-4o-mini",
            system="You are a helpful assistant.",
            messages=[user_message],
            tools=[],
        )
        message_response = ex.reply()
        print(message_response.content[0].text)
        return str(message_response.content[0].text)

    def system(self) -> str:
        return """This toolkit allows you to visually analyze images using AI capabilities."""
