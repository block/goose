from aci.common.enums import Environment
from aci.common.utils import check_and_get_env_variable, construct_db_url

# FastAPI APP CONFIG
APP_TITLE = "ACI Virtual MCP"
APP_ROOT_PATH = "/virtual"
APP_DOCS_URL = "/docs"
APP_REDOC_URL = "/redoc"
APP_OPENAPI_URL = "/openapi.json"


ENVIRONMENT = Environment(check_and_get_env_variable("VIRTUAL_MCP_ENVIRONMENT"))
LOG_LEVEL = check_and_get_env_variable("VIRTUAL_MCP_LOG_LEVEL", default="INFO")

# ROUTERS
ROUTER_PREFIX_HEALTH = "/health"
ROUTER_PREFIX_MCP = "/mcp"


# Authentication
SESSION_SECRET_KEY = check_and_get_env_variable("VIRTUAL_MCP_SESSION_SECRET_KEY")


# DB CONFIG
DB_SCHEME = check_and_get_env_variable("VIRTUAL_MCP_DB_SCHEME")
DB_USER = check_and_get_env_variable("VIRTUAL_MCP_DB_USER")
DB_PASSWORD = check_and_get_env_variable("VIRTUAL_MCP_DB_PASSWORD")
DB_HOST = check_and_get_env_variable("VIRTUAL_MCP_DB_HOST")
DB_PORT = check_and_get_env_variable("VIRTUAL_MCP_DB_PORT")
DB_NAME = check_and_get_env_variable("VIRTUAL_MCP_DB_NAME")
DB_FULL_URL = construct_db_url(DB_SCHEME, DB_USER, DB_PASSWORD, DB_HOST, DB_PORT, DB_NAME)

# 8KB
MAX_LOG_FIELD_SIZE = 8 * 1024

# Ops
SENTRY_DSN = check_and_get_env_variable("VIRTUAL_MCP_SENTRY_DSN")
