# import pytest
import base64
from exchange.exchange import Exchange
from exchange.message import Message
from exchange.content import ImageUrl
from exchange.providers.openai import OpenAiProvider
import httpx


def test_describe_image_url():
    image_url = ImageUrl(url="https://picsum.photos/id/1/200/200")  
    user_message = Message(role="user", content=["Describe the image: ", image_url])                                                                                                                                                                      
    ex = Exchange(
        provider=OpenAiProvider.from_env(),
        model="gpt-4o",
        system="You are a helpful assistant.",
        tools=[],
    )
    ex.add(user_message)
    res = ex.reply()
    assert "laptop" in res.content[0].text

def test_image_comparison_url_and_data():
    image_url_base64 = base64.standard_b64encode(httpx.get("https://picsum.photos/id/1/200/200").content).decode("utf-8")
    image_url = ImageUrl(url="https://picsum.photos/id/1/200/200")  
    user_message = Message(role="user", content=["Are these images the same? ", image_url_base64, image_url ])                                                                                                                                                                      
    ex = Exchange(
        provider=OpenAiProvider.from_env(),
        model="gpt-4o",
        system="Reply with yes or no.",
        tools=[],
    )
    ex.add(user_message)
    res = ex.reply()
    assert "yes" in res.content[0].text.lower()
