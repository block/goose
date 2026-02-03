from aci.common.enums import Environment
from aci.common.utils import check_and_get_env_variable, construct_db_url

# FastAPI APP CONFIG
APP_TITLE = "ACI Gateway MCP"
APP_ROOT_PATH = "/gateway"
APP_DOCS_URL = "/docs"
APP_REDOC_URL = "/redoc"
APP_OPENAPI_URL = "/openapi.json"


ENVIRONMENT = Environment(check_and_get_env_variable("MCP_ENVIRONMENT"))
LOG_LEVEL = check_and_get_env_variable("MCP_LOG_LEVEL", default="INFO")
LOG_STRUCTURED = (
    True
    if check_and_get_env_variable("MCP_LOG_STRUCTURED", default="true").lower() == "true"
    else False
)
# ROUTERS
ROUTER_PREFIX_HEALTH = "/health"
ROUTER_PREFIX_MCP = "/mcp"


# Authentication
SESSION_SECRET_KEY = check_and_get_env_variable("MCP_SESSION_SECRET_KEY")


# DB CONFIG
DB_SCHEME = check_and_get_env_variable("MCP_DB_SCHEME")
DB_USER = check_and_get_env_variable("MCP_DB_USER")
DB_PASSWORD = check_and_get_env_variable("MCP_DB_PASSWORD")
DB_HOST = check_and_get_env_variable("MCP_DB_HOST")
DB_PORT = check_and_get_env_variable("MCP_DB_PORT")
DB_NAME = check_and_get_env_variable("MCP_DB_NAME")
DB_FULL_URL = construct_db_url(DB_SCHEME, DB_USER, DB_PASSWORD, DB_HOST, DB_PORT, DB_NAME)

# LLM
OPENAI_API_KEY = check_and_get_env_variable("MCP_OPENAI_API_KEY")

# 8KB
MAX_LOG_FIELD_SIZE = 8 * 1024

# Ops
SENTRY_DSN = check_and_get_env_variable("MCP_SENTRY_DSN")

# mcp
MCP_SESSION_ID_HEADER = "mcp-session-id"

# OpenTelemetry
OTEL_ENABLED = (
    True
    if check_and_get_env_variable("MCP_OTEL_ENABLED", default="true").lower() == "true"
    else False
)
# Single endpoint for all signals (traces, metrics, logs) - gRPC will route automatically
OTEL_EXPORTER_OTLP_ENDPOINT = check_and_get_env_variable("MCP_OTEL_EXPORTER_OTLP_ENDPOINT")

# SUBSCRIPTION
SUBSCRIPTION_ENABLED = (
    True
    if check_and_get_env_variable("MCP_SUBSCRIPTION_ENABLED", default="false").lower() == "true"
    else False
)
