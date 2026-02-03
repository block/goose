# evolving_agents/config.py

import os

# LLM settings
LLM_PROVIDER = os.environ.get("LLM_PROVIDER", "openai")
LLM_MODEL = os.environ.get("LLM_MODEL", "gpt-4o-mini")
LLM_EMBEDDING_MODEL = os.environ.get("LLM_EMBEDDING_MODEL", "text-embedding-3-small")
LLM_USE_CACHE = os.environ.get("LLM_USE_CACHE", "false").lower() == "true"
LLM_CACHE_DIR = os.environ.get("LLM_CACHE_DIR", ".llm_cache")

INTENT_REVIEW_ENABLED = os.environ.get("INTENT_REVIEW_ENABLED", "false").lower() == "true"
INTENT_REVIEW_DEFAULT_INTERACTIVE = os.environ.get("INTENT_REVIEW_INTERACTIVE", "true").lower() == "true"
INTENT_REVIEW_OUTPUT_PATH = os.environ.get("INTENT_REVIEW_OUTPUT_PATH", "intent_plans")
INTENT_REVIEW_TIMEOUT = int(os.environ.get("INTENT_REVIEW_TIMEOUT", "600"))  # 10 minutes default
INTENT_REVIEW_LEVELS = os.environ.get("INTENT_REVIEW_LEVELS", "design,components,intents").split(",")

MONGODB_URI = os.environ.get("MONGODB_URI", "mongodb://localhost:27017")
MONGODB_DATABASE_NAME = os.environ.get("MONGODB_DATABASE_NAME", "evolving_agents_db")