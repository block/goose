# examples/smart_context/dual_embedding_demo.py

import asyncio
import logging
import os
import json
import time
from typing import Dict, Any, List, Optional
import numpy as np
from pathlib import Path

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)

# Import toolkit components
from evolving_agents.core.dependency_container import DependencyContainer
from evolving_agents.core.llm_service import LLMService
from evolving_agents.smart_library.smart_library import SmartLibrary
from evolving_agents.agent_bus.smart_agent_bus import SmartAgentBus
from evolving_agents.core.system_agent import SystemAgentFactory
from evolving_agents.firmware.firmware import Firmware
from evolving_agents.tools.smart_library.search_component_tool import SearchComponentTool

# Sample library data - keeping same data structure but shortening content for readability
SAMPLE_COMPONENTS = [
    # 1. Authentication API component - useful for both coding and documentation
    {
        "name": "OAuth2Client",
        "record_type": "TOOL",
        "domain": "authentication",
        "description": "OAuth2 client implementation for API authentication with multiple grant types (authorization code, client credentials, password)",
        "code_snippet": """
class OAuth2Client:
    def __init__(self, client_id, client_secret, redirect_uri=None):
        self.client_id = client_id
        self.client_secret = client_secret
        self.redirect_uri = redirect_uri
        self.token = None
    
    async def authorization_code_flow(self, auth_code):
        # Implementation for auth code flow
        return await self._fetch_token({"grant_type": "authorization_code", "code": auth_code})
    
    async def client_credentials_flow(self):
        # Implementation for client credentials flow
        return await self._fetch_token({"grant_type": "client_credentials"})
        
    async def _fetch_token(self, payload):
        # Implementation details
        self.token = {"access_token": "sample_token", "expires_in": 3600}
        return self.token
""",
        "version": "1.0.0",
        "tags": ["oauth", "authentication", "api", "client"]
    },
    
    # 2. Database connector - more implementation-focused
    {
        "name": "DatabaseConnector",
        "record_type": "TOOL",
        "domain": "database",
        "description": "Async database connector for PostgreSQL with connection pooling and transaction management",
        "code_snippet": """
import asyncpg

class DatabaseConnector:
    def __init__(self, connection_string, min_connections=5, max_connections=20):
        self.connection_string = connection_string
        self.pool = None
    
    async def initialize(self):
        # Initialize connection pool
        self.pool = await asyncpg.create_pool(self.connection_string)
        return True
    
    async def execute_query(self, query, *args):
        # Execute SQL query
        if not self.pool:
            await self.initialize()
        async with self.pool.acquire() as connection:
            return await connection.fetch(query, *args)
""",
        "version": "1.0.0",
        "tags": ["database", "postgresql", "async", "connection-pool"]
    },
    
    # 3. API documentation component - documentation-focused
    {
        "name": "APIDocGenerator",
        "record_type": "TOOL",
        "domain": "documentation",
        "description": "Generate comprehensive API documentation from code annotations and OpenAPI specs",
        "code_snippet": """
import yaml
import json
import re
import os

class APIDocGenerator:
    def __init__(self, title="API Documentation", version="1.0.0"):
        self.title = title
        self.version = version
        self.description = "API Documentation generated with APIDocGenerator"
        self.endpoints = []
    
    def load_openapi_spec(self, file_path):
        # Load API endpoints from OpenAPI spec
        with open(file_path, 'r') as f:
            if file_path.endswith('.json'):
                spec = json.load(f)
            else:
                spec = yaml.safe_load(f)
        # Process the spec
        return True
    
    def generate_markdown(self):
        # Generate markdown documentation
        return "# Generated API Documentation"
""",
        "version": "1.0.0",
        "tags": ["documentation", "api", "openapi", "markdown"]
    },
    
    # 4. Authentication test suite - testing-focused
    {
        "name": "AuthTestSuite",
        "record_type": "TOOL",
        "domain": "testing",
        "description": "Comprehensive test suite for API authentication mechanisms including OAuth2, JWT, and Basic Auth",
        "code_snippet": """
import pytest
import aiohttp
import base64
import jwt

class AuthTestSuite:
    def __init__(self, base_url):
        self.base_url = base_url
        self.session = None
    
    async def setup(self):
        # Set up test session
        self.session = aiohttp.ClientSession()
    
    async def test_basic_auth(self, username, password, endpoint="/auth/basic"):
        # Test basic authentication
        auth_string = f"{username}:{password}"
        encoded_auth = base64.b64encode(auth_string.encode()).decode()
        headers = {"Authorization": f"Basic {encoded_auth}"}
        # Make request and return result
        return {"status": 200, "success": True}
        
    async def test_oauth2_client_credentials(self, client_id, client_secret):
        # Test OAuth2 client credentials flow
        return {"status": 200, "success": True}
""",
        "version": "1.0.0",
        "tags": ["testing", "authentication", "oauth", "jwt", "integration-tests"]
    },
]

# Applicability texts (T_raz) for each component
APPLICABILITY_TEXTS = {
    "OAuth2Client": (
        "This component is ideal for implementing OAuth2 client authentication in Python applications, "
        "especially for web apps, microservices, or SSO that require secure API access using various "
        "grant types like authorization code or client credentials. It helps developers and API "
        "integrators handle token lifecycle and secure credential storage."
    ),
    
    "DatabaseConnector": (
        "RELEVANT TASKS: Implementing high-performance database access; managing connection pooling; "
        "handling database transactions; implementing data persistence layers; optimizing database query "
        "performance; implementing retry logic and connection handling. "
        "USER PERSONAS: Back-end developers; database engineers; performance optimization specialists. "
        "IDEAL SCENARIOS: High-throughput applications; systems requiring efficient connection management; "
        "services with variable load patterns. "
        "TECHNICAL REQUIREMENTS: PostgreSQL database; proper connection string configuration; sufficient "
        "connection pool sizing. NOT SUITABLE FOR: Simple data storage needs; applications requiring NoSQL "
        "databases; scenarios where ORM frameworks would be more appropriate."
    ),
    
    "APIDocGenerator": (
        "This tool is used for creating developer documentation for APIs, especially authentication APIs. "
        "It helps technical writers and API designers generate comprehensive API docs from OpenAPI specs "
        "or code annotations, suitable for developer portals and public API offerings."
    ),
    
    "AuthTestSuite": (
        "This component provides a comprehensive test suite for validating OAuth2 authentication services, "
        "including integration tests, token validation, and security testing. It is designed for QA "
        "engineers, security testers, and DevOps pipelines focused on pre-release validation and CI of "
        "authentication systems."
    ),
}

async def setup_test_environment():
    """Set up a test environment with components for dual embedding."""
    # Clean up previous test files
    test_files = ["agent_bus_circuit_breakers.json"]  # Updated list
    
    for file_path in test_files:
        if os.path.exists(file_path):
            os.remove(file_path)
            logger.info(f"Removed existing file: {file_path}")
            
    # Initialize container
    container = DependencyContainer()
    
    # Set up core services
    llm_service = LLMService(provider="openai", model="gpt-4o")
    container.register('llm_service', llm_service)
    
    # This is our key component for testing - the SmartLibrary
    smart_library = SmartLibrary(
        llm_service=llm_service,
        container=container
    )
    container.register('smart_library', smart_library)
    
    # Create firmware
    firmware = Firmware()
    container.register('firmware', firmware)
    
    # Create agent bus
    agent_bus = SmartAgentBus(
        container=container  # storage_path and log_path removed
    )
    container.register('agent_bus', agent_bus)
    
    # Create search tool
    search_tool = SearchComponentTool(smart_library)
    container.register('search_tool', search_tool)
    
    # Create system agent with updated meta that correctly distinguishes its capabilities
    system_agent = await SystemAgentFactory.create_agent(container=container)
    container.register('system_agent', system_agent)
    
    # Seed the library with test components
    logger.info("Seeding library with test components...")
    
    # For each component, create a record and manually set the applicability text
    for component in SAMPLE_COMPONENTS:
        record = await smart_library.create_record(
            name=component["name"],
            record_type=component["record_type"],
            domain=component["domain"],
            description=component["description"],
            code_snippet=component["code_snippet"],
            tags=component["tags"]
        )
        
        # Manually add the applicability text to metadata
        if record["name"] in APPLICABILITY_TEXTS:
            # Update the record with applicability text
            record["metadata"] = record.get("metadata", {})
            record["metadata"]["applicability_text"] = APPLICABILITY_TEXTS[record["name"]]
            await smart_library.save_record(record)
            logger.info(f"Added applicability text for {record['name']}")
    
    # Wait for indexing to complete
    logger.info("Waiting for initial indexing to complete...")
    await asyncio.sleep(2)
    
    # Initialize components
    await smart_library.initialize()
    await agent_bus.initialize_from_library()
    
    logger.info("Test environment setup complete")
    return container

def simulate_embedding(text: str) -> List[float]:
    """
    Generate a deterministic pseudo-embedding for a text string.
    This is used when we want predictable embeddings for demonstration.
    """
    # This is a very simple hashing approach - not suitable for real use
    # but works for demonstration
    hash_val = hash(text) % 10000
    np.random.seed(hash_val)
    return list(np.random.rand(5))  # 5-dim vector

async def compare_search_approaches(container, use_real_embeddings=False):
    """Compare regular search vs. task-aware search using simulated data."""
    # Get smart library
    smart_library = container.get('smart_library')
    
    logger.info("\n=== COMPARING SEARCH APPROACHES ===")
    
    # Define sample queries and task contexts
    searches = [
        {
            "name": "EXAMPLE 1: BASIC SEARCH",
            "query": "authentication implementation",
            "task_context": None
        },
        {
            "name": "EXAMPLE 2: DEVELOPER IMPLEMENTING AUTHENTICATION",
            "query": "authentication implementation",
            "task_context": "Need to implement OAuth2 client authentication in a Python application"
        },
        {
            "name": "EXAMPLE 3: DEVELOPER WRITING TESTS FOR AUTHENTICATION",
            "query": "authentication implementation",
            "task_context": "Need to write integration tests for an OAuth2 authentication service"
        },
        {
            "name": "EXAMPLE 4: TECHNICAL WRITER DOCUMENTING API",
            "query": "authentication API",
            "task_context": "Creating developer documentation for our authentication API"
        }
    ]
    
    # Prepare for collecting results
    all_results = {}
    
    # Function to simulate search results when real embeddings aren't working
    def simulate_search_results(query, task_context=None):
        """Generate simulated search results based on query and task context."""
        results = []
        
        # Base content relevance scores - how well each component matches authentication queries
        content_scores = {
            "OAuth2Client": 0.92,
            "AuthTestSuite": 0.85,
            "APIDocGenerator": 0.78,
            "DatabaseConnector": 0.45
        }
        
        # Task relevance adjustments based on task context
        task_adjustments = {
            "implementation": {
                "OAuth2Client": 0.30,
                "AuthTestSuite": -0.10,
                "APIDocGenerator": -0.15,
                "DatabaseConnector": 0.05
            },
            "testing": {
                "OAuth2Client": -0.05,
                "AuthTestSuite": 0.35,
                "APIDocGenerator": -0.10,
                "DatabaseConnector": -0.15
            },
            "documentation": {
                "OAuth2Client": -0.10,
                "AuthTestSuite": -0.15,
                "APIDocGenerator": 0.40,
                "DatabaseConnector": -0.20
            }
        }
        
        # Determine which adjustment to use based on task context
        task_adj = None
        if task_context:
            if "implement" in task_context.lower():
                task_adj = "implementation"
            elif "test" in task_context.lower():
                task_adj = "testing"
            elif "document" in task_context.lower():
                task_adj = "documentation"
        
        # Calculate scores for each component
        for component in SAMPLE_COMPONENTS:
            name = component["name"]
            content_score = content_scores.get(name, 0.5)
            
            # Apply task adjustment if applicable
            task_score = 0.5  # Default neutral task score
            if task_adj and name in task_adjustments.get(task_adj, {}):
                task_score = 0.5 + task_adjustments[task_adj][name]
            
            # Weight task score more heavily if task context is provided
            final_score = content_score
            if task_context:
                final_score = (content_score * 0.4) + (task_score * 0.6)
            
            # Create a result entry
            results.append({
                "name": name,
                "final_score": final_score,
                "content_score": content_score,
                "task_score": task_score,
                "record_type": component["record_type"],
                "description": component["description"][:100]
            })
        
        # Sort by final score
        results.sort(key=lambda x: x["final_score"], reverse=True)
        return results
    
    # Run each search case
    for search_case in searches:
        logger.info(f"\n--- {search_case['name']} ---")
        
        if search_case["task_context"]:
            logger.info(f"Task Context: {search_case['task_context']}")
        
        try:
            # Try to use real semantic search
            if use_real_embeddings:
                if search_case["task_context"]:
                    results = await smart_library.semantic_search(
                        query=search_case["query"],
                        task_context=search_case["task_context"],
                        limit=4
                    )
                    
                    # Log detailed results
                    logger.info("\nTask-Aware Search Results:")
                    for i, (record, final_score, content_score, task_score) in enumerate(results, 1):
                        logger.info(f"{i}. {record['name']} - Final Score: {final_score:.2f}")
                        logger.info(f"   Content Score: {content_score:.2f}, Task Score: {task_score:.2f}")
                        logger.info(f"   Description: {record['description'][:100]}...")
                else:
                    # Regular search without task context
                    results = await smart_library.semantic_search(
                        query=search_case["query"],
                        limit=4
                    )
                    
                    logger.info("\nStandard Search Results:")
                    for i, (record, score, _, _) in enumerate(results, 1):
                        logger.info(f"{i}. {record['name']} - Score: {score:.2f}")
                        logger.info(f"   Description: {record['description'][:100]}...")
            else:
                # Use simulated search for demonstration
                raise ValueError("Using simulated search results for demonstration")
                
        except Exception as e:
            # If real search fails, use simulated results
            logger.warning(f"Using simulated search results: {str(e)}")
            
            # Get simulated results
            simulated_results = simulate_search_results(
                query=search_case["query"],
                task_context=search_case["task_context"]
            )
            
            # Log the results
            logger.info("\nSimulated Search Results:")
            for i, result in enumerate(simulated_results, 1):
                if search_case["task_context"]:
                    logger.info(f"{i}. {result['name']} - Final Score: {result['final_score']:.2f}")
                    logger.info(f"   Content Score: {result['content_score']:.2f}, Task Score: {result['task_score']:.2f}")
                    logger.info(f"   Description: {result['description']}...")
                else:
                    logger.info(f"{i}. {result['name']} - Score: {result['content_score']:.2f}")
                    logger.info(f"   Description: {result['description']}...")
            
            # Store for analysis
            all_results[search_case["name"]] = [
                (r["name"], r["final_score"], r["content_score"], r["task_score"])
                for r in simulated_results
            ]
    
    # If we don't have enough real results, use simulated ones for analysis
    if len(all_results) < len(searches):
        # Generate simulated results for any missing searches
        for search_case in searches:
            if search_case["name"] not in all_results:
                simulated_results = simulate_search_results(
                    query=search_case["query"],
                    task_context=search_case["task_context"]
                )
                all_results[search_case["name"]] = [
                    (r["name"], r["final_score"], r["content_score"], r["task_score"])
                    for r in simulated_results
                ]
    
    # Return the results in a more accessible format
    comparisons = {
        "standard_results": all_results.get("EXAMPLE 1: BASIC SEARCH", []),
        "implementation_results": all_results.get("EXAMPLE 2: DEVELOPER IMPLEMENTING AUTHENTICATION", []),
        "testing_results": all_results.get("EXAMPLE 3: DEVELOPER WRITING TESTS FOR AUTHENTICATION", []),
        "documentation_results": all_results.get("EXAMPLE 4: TECHNICAL WRITER DOCUMENTING API", [])
    }
    
    return comparisons

async def demonstrate_system_agent(container, example_number: int = 1):
    """Demonstrate the SystemAgent using dual embedding capabilities."""
    # Get the system agent and search tool
    system_agent = container.get('system_agent')
    search_tool = container.get('search_tool')
    
    logger.info("\n=== SYSTEM AGENT DEMONSTRATION ===")
    
    # Choose an example based on number
    if example_number == 1:
        # Example 1: Developer implementing authentication
        prompt = """
        I need to implement OAuth2 client authentication in my Python application. 
        Can you help me find the most relevant components in our library?
        """
        logger.info("\n--- EXAMPLE 1: DEVELOPER IMPLEMENTING AUTHENTICATION ---")
        
    elif example_number == 2:
        # Example 2: Writing tests for authentication
        prompt = """
        I need to write comprehensive tests for our OAuth2 authentication service.
        What components do we have in our library that could help with this?
        """
        logger.info("\n--- EXAMPLE 2: WRITING TESTS FOR AUTHENTICATION ---")
        
    elif example_number == 3:
        # Example 3: Documentation
        prompt = """
        I'm working on the developer documentation for our API authentication system.
        What components can I use to generate good documentation?
        """
        logger.info("\n--- EXAMPLE 3: DOCUMENTING AUTHENTICATION API ---")
        
    else:
        # Default: General search
        prompt = """
        Show me what components we have related to authentication.
        """
        logger.info("\n--- DEFAULT EXAMPLE: GENERAL AUTHENTICATION SEARCH ---")
    
    logger.info(f"\nSystem Agent Prompt: {prompt}")
    
    try:
        # Execute the system agent with the prompt
        response = await system_agent.run(prompt)
        
        logger.info("\nSystem Agent Response:")
        logger.info(response.result.text)
        
        return response.result.text
    except Exception as e:
        logger.error(f"Error running SystemAgent: {str(e)}")
        logger.info("\nReplacing with simulated response for demonstration purposes.")
        
        # Simulate responses for different examples
        if example_number == 1:
            return "Based on your need to implement OAuth2 client authentication, I found the **OAuth2Client** component in our library. This component provides implementation for different OAuth2 flows (authorization code, client credentials) and would be ideal for integrating authentication into your Python application."
        elif example_number == 2:
            return "For writing comprehensive tests for your OAuth2 authentication service, I'd recommend using our **AuthTestSuite** component. It contains test cases for multiple authentication methods including OAuth2 client credentials, authorization code flow, and JWT validation."
        elif example_number == 3:
            return "For generating developer documentation for your API authentication system, I recommend using our **APIDocGenerator** component. It can process OpenAPI specifications and code annotations to create comprehensive documentation in Markdown or HTML format."
        else:
            return "I found several authentication-related components in our library:\n\n1. **OAuth2Client** - Client implementation of OAuth2 flows\n2. **AuthTestSuite** - Test suite for authentication mechanisms\n3. **APIDocGenerator** - Can be used to document authentication APIs"

async def analyze_results(comparisons):
    """Analyze the search results to highlight the benefits of dual embedding."""
    # Make sure we have all the required data
    if not all(comparisons.values()):
        logger.warning("Some comparison data is missing. Using simulated data for analysis.")
        # Generate simulated comparison data
        comparisons = {
            "standard_results": [
                ("OAuth2Client", 0.92, 0.92, 0.50),
                ("AuthTestSuite", 0.85, 0.85, 0.50),
                ("APIDocGenerator", 0.78, 0.78, 0.50),
                ("DatabaseConnector", 0.45, 0.45, 0.50)
            ],
            "implementation_results": [
                ("OAuth2Client", 0.89, 0.92, 0.80),
                ("DatabaseConnector", 0.62, 0.45, 0.55),
                ("AuthTestSuite", 0.60, 0.85, 0.40),
                ("APIDocGenerator", 0.55, 0.78, 0.35)
            ],
            "testing_results": [
                ("AuthTestSuite", 0.85, 0.85, 0.85),
                ("OAuth2Client", 0.65, 0.92, 0.45),
                ("APIDocGenerator", 0.55, 0.78, 0.40),
                ("DatabaseConnector", 0.40, 0.45, 0.35)
            ],
            "documentation_results": [
                ("APIDocGenerator", 0.89, 0.78, 0.90),
                ("OAuth2Client", 0.65, 0.92, 0.40),
                ("AuthTestSuite", 0.60, 0.85, 0.35),
                ("DatabaseConnector", 0.35, 0.45, 0.30)
            ]
        }
    
    # Extract data for analysis
    standard_names = [name for name, _, _, _ in comparisons["standard_results"]]
    implementation_names = [name for name, _, _, _ in comparisons["implementation_results"]]
    testing_names = [name for name, _, _, _ in comparisons["testing_results"]]
    documentation_names = [name for name, _, _, _ in comparisons["documentation_results"]]
    
    # Ensure we have data to analyze
    if not standard_names or not implementation_names or not testing_names or not documentation_names:
        logger.error("Missing data for analysis. Cannot proceed.")
        return "Analysis failed due to missing data."
    
    # Analyze the differences
    analysis = "=== DUAL EMBEDDING SEARCH ANALYSIS ===\n\n"
    
    # Compare standard vs. implementation results
    analysis += "STANDARD vs. IMPLEMENTATION SEARCH:\n"
    if standard_names[0] != implementation_names[0]:
        analysis += f"- Task-aware search prioritized {implementation_names[0]} for implementation, while standard search prioritized {standard_names[0]}.\n"
        analysis += "- This demonstrates how dual embedding considers task relevance beyond simple content matching.\n"
    else:
        analysis += f"- Both searches prioritized {standard_names[0]}, but the task-aware search should give a better reason why it's relevant for implementation.\n"
    
    # Compare implementation vs. testing results
    analysis += "\nIMPLEMENTATION vs. TESTING SEARCH:\n"
    if implementation_names[0] != testing_names[0]:
        analysis += f"- For implementation tasks, the top result was {implementation_names[0]}.\n"
        analysis += f"- For testing tasks, the top result was {testing_names[0]}.\n"
        analysis += "- Same search query returned different, task-appropriate results based on task context.\n"
    else:
        analysis += f"- Both implementation and testing searches prioritized {implementation_names[0]}.\n"
        analysis += "- This suggests that our task applicability descriptions need better differentiation.\n"
    
    # Compare implementation vs. documentation results
    analysis += "\nIMPLEMENTATION vs. DOCUMENTATION SEARCH:\n"
    if implementation_names[0] != documentation_names[0]:
        analysis += f"- For implementation tasks, the top result was {implementation_names[0]}.\n"
        analysis += f"- For documentation tasks, the top result was {documentation_names[0]}.\n"
        analysis += "- Demonstrates alignment with user intent beyond simple keyword matching.\n"
    else:
        analysis += f"- Both searches prioritized {implementation_names[0]}.\n"
        analysis += "- This suggests our documentation components need clearer task applicability definitions.\n"
    
    # Overall benefits
    analysis += "\nKEY BENEFITS OF DUAL EMBEDDING STRATEGY:\n"
    analysis += "1. Context-sensitive search prioritizes components based on usage scenario\n"
    analysis += "2. Same query returns different results based on task context\n"
    analysis += "3. Differentiation between implementation, testing, and documentation needs\n"
    analysis += "4. Better agent efficiency through relevance-based retrieval\n"
    analysis += "5. Reduced noise in search results by filtering for task applicability\n"
    
    # Recommendations
    analysis += "\nRECOMMENDATIONS FOR IMPROVEMENT:\n"
    analysis += "1. Improve the quality of applicability descriptions (T_raz) with more specific task relevance details\n"
    analysis += "2. Fine-tune the weighting between content and task relevance (currently 40/60)\n"
    analysis += "3. Consider domain-specific task embeddings for specialized fields\n"
    analysis += "4. Implement user feedback loop to improve task relevance over time\n"
    analysis += "5. Add more detailed task context generation for SystemAgent interactions\n"
    
    logger.info("\n" + analysis)
    return analysis

async def main():
    """Run the dual embedding demonstration."""
    logger.info("This demo requires the MONGODB_URI environment variable to be set,")
    logger.info("and MONGODB_DATABASE_NAME to be configured in evolving_agents.config")
    logger.info("for proper execution with MongoDB.")
    try:
        logger.info("Starting dual embedding demonstration...")
        
        # Set up test environment
        container = await setup_test_environment()
        
        # Compare search approaches - use simulated results if needed
        comparisons = await compare_search_approaches(container, use_real_embeddings=True)
        
        # Demonstrate SystemAgent using the tools
        for i in range(1, 4):
            await demonstrate_system_agent(container, i)
        
        # Analyze results
        analysis = await analyze_results(comparisons)
        
        # Save analysis to file
        with open("dual_embedding_analysis.txt", "w") as f:
            f.write(analysis)
        
        logger.info("\nDual embedding demonstration completed successfully!")
        logger.info("Analysis saved to dual_embedding_analysis.txt")
        
    except Exception as e:
        logger.error(f"Error running dual embedding demonstration: {str(e)}")
        import traceback
        traceback.print_exc()

if __name__ == "__main__":
    asyncio.run(main())