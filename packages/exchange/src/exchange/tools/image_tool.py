from exchange import tool
from exchange.content import ImageUrl, ToolResult
from exchange.exchange import Exchange
from exchange.message import Message
from exchange.providers.openai import OpenAiProvider

@tool
def image_analysis(image: ImageUrl, instructions: str="Describe the image") -> ToolResult:
    """Image Analysis with OpenAI Vision"""
    user_message = Message(role="user", content=[f"{instructions}: ", image])                                                                                                                                                                      
    ex = Exchange(
        provider=OpenAiProvider.from_env(),
        model="gpt-4o",
        system="You are a helpful assistant.",
        tools=[],
    )
    ex.add(user_message)
    description = ex.reply()
    return ToolResult(tool_use_id="describe_image", output=description, is_error=False)
