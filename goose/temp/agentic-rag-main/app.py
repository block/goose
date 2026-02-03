from fastapi import FastAPI, Request
from fastapi.templating import Jinja2Templates
from fastapi.staticfiles import StaticFiles
from fastapi.responses import HTMLResponse
from pydantic import BaseModel
from singlestore_swarm import (
    rails, get_guardrails_response, is_singlestore_query,
    direct_llm_response, Swarm, Agent, search_movies
)
import singlestore_swarm

app = FastAPI()
templates = Jinja2Templates(directory="templates")
app.mount("/static", StaticFiles(directory="static"), name="static")

# Initialize Swarm client and agent
swarm_client = Swarm()
agent = Agent(
    name="MovieRecommendationAgent",
    instructions="You are a helpful movie recommendation agent.",
    functions=[search_movies]
)

class ChatMessage(BaseModel):
    message: str

@app.get("/", response_class=HTMLResponse)
async def read_root(request: Request):
    return templates.TemplateResponse("index.html", {"request": request})

@app.post("/chat")
async def chat(message: ChatMessage):
    try:
        # Get guardrails response
        guardrails_response = await get_guardrails_response(message.message)
        
        # Early return for refusal responses
        if isinstance(guardrails_response, dict):
            content = guardrails_response.get('content', '').lower()
            if "cannot provide" in content or "not able to provide" in content:
                return {"response": guardrails_response['content']}
        
        # Process response based on type
        if await is_singlestore_query(guardrails_response):
            # Set the current_query in the singlestore_swarm module
            singlestore_swarm.current_query = message.message
            response = swarm_client.run(
                agent=agent,
                messages=[{"role": "user", "content": message.message}]
            )
            return {"response": response.messages[-1]["content"]}
        else:
            response = await direct_llm_response(message.message)
            return {"response": response}
                
    except Exception as e:
        return {"response": f"An error occurred: {str(e)}"}
