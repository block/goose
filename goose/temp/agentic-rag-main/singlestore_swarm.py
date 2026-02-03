from swarm import Swarm, Agent
import singlestoredb as s2
import os
from nemoguardrails import LLMRails, RailsConfig
from openai import OpenAI
from typing import Dict, Any, List, Callable, Tuple
import numpy as np
from functools import lru_cache

# Initialize OpenAI
client = OpenAI(
    api_key=os.getenv("OPENAI_API_KEY"),
    timeout=10.0,  # Reduce default timeout
    max_retries=2  # Limit retries
)

# Initialize NeMo Guardrails
config = RailsConfig.from_path("nemo-configs/")
rails = LLMRails(config)

# Cache for OpenAI responses
@lru_cache(maxsize=100)
def cached_llm_response_sync(query: str) -> str:
    """Cached version of LLM responses for repeated queries"""
    try:
        response = client.chat.completions.create(
            model="gpt-3.5-turbo",
            messages=[
                {"role": "user", "content": query}
            ]
        )
        return response.choices[0].message.content or "No response generated"
    except Exception as e:
        return f"Error getting LLM response: {e}"

async def cached_llm_response(query: str) -> str:
    """Async wrapper for cached LLM responses"""
    return cached_llm_response_sync(query)

async def direct_llm_response(query: str) -> str:
    """Get response directly from LLM for non-SingleStore queries"""
    return await cached_llm_response(query)

# Cache for guardrails responses
response_cache = {}

async def get_guardrails_response(query: str) -> dict:
    """Cached version of guardrails responses"""
    if query not in response_cache:
        response_cache[query] = await rails.generate_async(messages=[{"role": "user", "content": query}])
    return response_cache[query]

# Global connection and query
singlestore_pool = None
current_query = ""

async def get_db_connection():
    """Get a connection from the pool"""
    global singlestore_pool
    try:
        if singlestore_pool is None:
            # Get connection parameters from environment
            host = os.getenv("SINGLESTORE_HOST")
            user = os.getenv("SINGLESTORE_USER")
            password = os.getenv("SINGLESTORE_PASSWORD")
            database = os.getenv("SINGLESTORE_DATABASE")

            # Create connection pool
            singlestore_pool = s2.connect(
                host=host,
                port=3306,
                user=user,
                password=password,
                database=database,
                pool_size=5,  # Adjust based on your needs
                pool_name='singlestore_pool'
            )
        return singlestore_pool
    except Exception as e:
        print(f"Error connecting to SingleStore: {e}")
        return None

async def search_movies() -> str:
    """Core function to search movies in SingleStore"""
    global current_query
    
    try:
        conn = await get_db_connection()
        if not conn:
            return "Failed to connect to database"

        sql_query = '''
           SELECT title, MATCH(title) AGAINST (?) as relevance
           FROM movies
           WHERE MATCH(title) AGAINST (?)
           ORDER BY relevance DESC
           LIMIT 10
        '''

        cursor = conn.cursor()
        cursor.execute(sql_query, (current_query, current_query))
        
        # Convert cursor results to numpy array for easier handling
        results = np.array(cursor.fetchall())
        cursor.close()  # Close cursor but keep connection in pool

        if len(results) == 0:
            return "No movie recommendations found for your query."
            
        response = "Here are some movie recommendations:\n"
        for title, relevance in results:
            response += f"- {title} (Relevance: {float(relevance):.2f})\n"
            
        return response
            
    except Exception as e:
        return f"Error getting recommendations: {e}"

async def is_singlestore_query(response) -> bool:
    """Check if the guardrails response indicates need for SingleStore"""
    try:
        # Print the full response for debugging
        print("\nNemo Guardrails Full Response:", response)
        
        # Get the last message from the response
        if isinstance(response, dict):
            response_text = response.get('content', '').lower()
        else:
            response_text = response.last_message.content.lower()
            
        print("\nNemo Guardrails Response Text:", response_text)
        is_singlestore = "inform using singlestore" in response_text or "delegate to agent" in response_text
        print("Is SingleStore Query?:", is_singlestore)
        
        return is_singlestore
    except Exception as e:
        # If we can't access the response content as expected,
        # print more debug information
        print("\nError accessing Nemo Guardrails response:", str(e))
        print("Response type:", type(response))
        return False

async def main():
    global current_query
    
    # Initialize Swarm client
    swarm_client = Swarm()
    
    # Initialize agent with the movie recommendation function
    agent = Agent(
        name="MovieRecommendationAgent",
        instructions="You are a helpful movie recommendation agent.",
        functions=[search_movies]
    )

    print("Welcome! You can ask me anything. Type 'exit' to quit.")
    
    while True:
        # Get user input
        user_query = input("\nYou: ").strip()
        
        if user_query.lower() == 'exit':
            print("Goodbye!")
            if singlestore_pool:
                singlestore_pool.close()
            break
        
        try:
            print("\nSending query to Nemo Guardrails...")
            # First pass through guardrails
            guardrails_response = await get_guardrails_response(user_query)
            print("\n=== Nemo Guardrails Response Details ===")
            print("Full Response Object:", guardrails_response)
            print("Response Type:", type(guardrails_response))
            if isinstance(guardrails_response, dict):
                print("Response Keys:", guardrails_response.keys())
                print("Response Content:", guardrails_response.get('content'))
            print("=====================================\n")
            
            # Extract the actual response content
            if isinstance(guardrails_response, dict):
                response_content = guardrails_response
                # Use the guardrails response directly if it's a refusal
                if "cannot provide" in response_content.get('content', '').lower() or \
                   "not able to provide" in response_content.get('content', '').lower():
                    print("Bot:", response_content['content'])
                    continue
            else:
                response_content = guardrails_response.last_message
            
            # Check if query needs SingleStore
            if await is_singlestore_query(response_content):
                # Update current query for the search function
                current_query = user_query
                
                # Use SingleStore agent for movie recommendations
                messages = [{"role": "user", "content": user_query}]
                response = await swarm_client.run(agent=agent, messages=messages)
                print("Bot:", response.messages[-1]["content"])
            else:
                # Use direct LLM response for general queries
                response = await direct_llm_response(user_query)
                print("Bot:", response)
                
        except Exception as e:
            print(f"An error occurred: {e}")

if __name__ == "__main__":
    import asyncio
    asyncio.run(main())
