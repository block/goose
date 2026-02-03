# Movie Recommendation System with Chat Interface

A web-based movie recommendation system that uses OpenAI's GPT, Nemo Guardrails, and SingleStore to provide personalized movie recommendations and engage in general conversation about movies.

## Features

- Interactive chat interface for movie recommendations
- Natural language processing for understanding user queries
- Integration with SingleStore for efficient movie data storage and retrieval
- Real-time response with loading indicators
- Support for both movie-specific and general queries
- Caching system for improved performance

## Prerequisites

- Python 3.8 or higher
- SingleStore database instance
- OpenAI API key
- Git (for cloning the repository)

## Installation

1. Clone the repository:
```bash
git clone <repository-url>
cd singlestore-swarm-agent
```

2. Create and activate a virtual environment:
```bash
# On macOS/Linux
python -m venv venv
source venv/bin/activate

# On Windows
python -m venv venv
.\venv\Scripts\activate
```

3. Install the required packages:
```bash
pip install -r requirements.txt
```

4. Create a `.env` file in the project root with the following variables:
```
OPENAI_API_KEY=your_openai_api_key
SINGLESTORE_HOST=your_singlestore_host
SINGLESTORE_USER=your_singlestore_user
SINGLESTORE_PASSWORD=your_singlestore_password
SINGLESTORE_DATABASE=your_singlestore_database
```

## Running the Application

1. Make sure your virtual environment is activated:
```bash
source venv/bin/activate  # On macOS/Linux
.\venv\Scripts\activate   # On Windows
```

2. Start the FastAPI server:
```bash
uvicorn app:app --reload
```

3. Open your web browser and navigate to:
```
http://localhost:8000
```

## Project Structure

- `app.py`: FastAPI application and route handlers
- `singlestore_swarm.py`: Core logic for movie recommendations and database interactions
- `templates/`: HTML templates for the web interface
- `nemo-configs/`: Configuration files for Nemo Guardrails
- `requirements.txt`: Python package dependencies

## Code Flow

1. User sends a message through the web interface
2. The message is processed by FastAPI and sent to the chat endpoint
3. Nemo Guardrails analyzes the query to determine if it's movie-related
4. If movie-related:
   - Query is processed by SingleStore for movie recommendations
   - Results are formatted and returned to the user
5. If general query:
   - Query is handled by OpenAI's GPT
   - Response is cached for future use
6. Response is sent back to the web interface

## Troubleshooting

- If you encounter database connection issues, verify your SingleStore credentials in the `.env` file
- For OpenAI API errors, check if your API key is valid and has sufficient credits
- If the web interface is not loading, ensure all static files are properly served

## Contributing

Feel free to submit issues and enhancement requests!

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

Note: This project includes dependencies that may be licensed under different terms. Please refer to each dependency's documentation for their specific license terms.
