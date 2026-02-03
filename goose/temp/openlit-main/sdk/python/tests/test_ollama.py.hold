import ollama
import openlit

openlit.init()

ollama.pull('gemma2:2b')

stream = ollama.chat(
    model='gemma2:2b',
    messages=[{'role': 'user', 'content': 'Hello sync non-streaming, What is LLM observability?'}],
    stream=False,
)

stream = ollama.chat(
    model='gemma2:2b',
    messages=[{'role': 'user', 'content': 'Hi sync streaming, What is LLM observability?'}],
    stream=True,
)

for chunk in stream:
  pass

ollama.embeddings(model='gemma2:2b', prompt='Hi sync embedding, What is LLM observability?')


import asyncio
from ollama import AsyncClient

async def chat():
  message = {'role': 'user', 'content': 'Hi async non-streaming, What is LLM observability?'}
  response = await AsyncClient().chat(model='gemma2:2b', messages=[message])

  message = {'role': 'user', 'content': 'Hi async streaming, What is LLM observability?'}
  async for part in await AsyncClient().chat(model='gemma2:2b', messages=[message], stream=True):
    pass

  await AsyncClient().embeddings(model='gemma2:2b', prompt='Hi async embedding, What is LLM observability?')
  

asyncio.run(chat())