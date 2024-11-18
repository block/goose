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


def get_lorem_picsum_base64(url):
    image_response = httpx.get(url, follow_redirects=True)
    if image_response.is_redirect:
        redirect_url = image_response.headers["Location"]
        if "fastly.picsum.photos" in redirect_url:
            # Follow redirects to fastly
            image_response = httpx.get(redirect_url)

    image_content = image_response.content
    base_64_encoded_image = (
        f'data:image/jpeg;base64,{base64.standard_b64encode(image_content).decode("utf-8")}'
    )
    return base_64_encoded_image


def test_image_comparison_url_and_data():

    base_64_image_data = get_lorem_picsum_base64("https://picsum.photos/id/1/200/200")
    base_64_image_url = ImageUrl(url=base_64_image_data)

    user_message = Message(
        role="user",
        content=[
            "Are these images the same? ",
            ImageUrl(url=base_64_image_url),
            ImageUrl(url="https://picsum.photos/id/1/200/200"),
        ],
    )
    ex = Exchange(
        provider=OpenAiProvider.from_env(),
        model="gpt-4o",
        system="Reply with yes or no.",
        tools=[],
    )
    ex.add(user_message)
    res = ex.reply()
    # The assertion below is based on comparing two identical URLs.
    assert "yes" in res.content[0].text.lower()
