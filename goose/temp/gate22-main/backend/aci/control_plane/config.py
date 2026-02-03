from aci.common.enums import Environment
from aci.common.utils import check_and_get_env_variable, construct_db_url

# FastAPI APP CONFIG
APP_TITLE = "ACI Control Plane"
APP_ROOT_PATH = "/v1/control-plane"
APP_DOCS_URL = "/docs"
APP_REDOC_URL = "/redoc"
APP_OPENAPI_URL = "/openapi.json"


ENVIRONMENT = Environment(check_and_get_env_variable("CONTROL_PLANE_ENVIRONMENT"))
CONTROL_PLANE_BASE_URL = check_and_get_env_variable("CONTROL_PLANE_BASE_URL")
LOG_LEVEL = check_and_get_env_variable("CONTROL_PLANE_LOG_LEVEL", default="INFO")

# ROUTERS
ROUTER_PREFIX_HEALTH = "/health"
ROUTER_PREFIX_AUTH = "/auth"
ROUTER_PREFIX_ORGANIZATIONS = "/organizations"
ROUTER_PREFIX_USERS = "/users"
ROUTER_PREFIX_MCP_SERVERS = "/mcp-servers"
ROUTER_PREFIX_MCP_SERVER_CONFIGURATIONS = "/mcp-server-configurations"
ROUTER_PREFIX_CONNECTED_ACCOUNTS = "/connected-accounts"
ROUTER_PREFIX_MCP_SERVER_BUNDLES = "/mcp-server-bundles"
ROUTER_PREFIX_MCP_TOOLS = "/mcp-tools"
ROUTER_PREFIX_LOGS = "/logs"
ROUTER_PREFIX_SUBSCRIPTIONS = "/subscriptions"

# Frontend
FRONTEND_URL = check_and_get_env_variable("CONTROL_PLANE_FRONTEND_URL")

# Authentication
GOOGLE_CLIENT_ID = check_and_get_env_variable("CONTROL_PLANE_GOOGLE_CLIENT_ID")
GOOGLE_CLIENT_SECRET = check_and_get_env_variable("CONTROL_PLANE_GOOGLE_CLIENT_SECRET")
SESSION_SECRET_KEY = check_and_get_env_variable("CONTROL_PLANE_SESSION_SECRET_KEY")
REFRESH_TOKEN_KEY = check_and_get_env_variable("CONTROL_PLANE_REFRESH_TOKEN_KEY")
JWT_SIGNING_KEY = check_and_get_env_variable("CONTROL_PLANE_JWT_SIGNING_KEY")
JWT_ALGORITHM = "HS256"
JWT_ACCESS_TOKEN_EXPIRE_MINUTES = 1440
EMAIL_VERIFICATION_EXPIRE_MINUTES = 1440
ORGANIZATION_INVITATION_EXPIRE_MINUTES = 10080


# DB CONFIG
DB_SCHEME = check_and_get_env_variable("CONTROL_PLANE_DB_SCHEME")
DB_USER = check_and_get_env_variable("CONTROL_PLANE_DB_USER")
DB_PASSWORD = check_and_get_env_variable("CONTROL_PLANE_DB_PASSWORD")
DB_HOST = check_and_get_env_variable("CONTROL_PLANE_DB_HOST")
DB_PORT = check_and_get_env_variable("CONTROL_PLANE_DB_PORT")
DB_NAME = check_and_get_env_variable("CONTROL_PLANE_DB_NAME")
DB_FULL_URL = construct_db_url(DB_SCHEME, DB_USER, DB_PASSWORD, DB_HOST, DB_PORT, DB_NAME)

# LLM
OPENAI_API_KEY = check_and_get_env_variable("CONTROL_PLANE_OPENAI_API_KEY")

SENDER_EMAIL = check_and_get_env_variable("CONTROL_PLANE_SENDER_EMAIL")
SENDER_NAME = "ACI.dev Team"

DEFAULT_MCP_SERVER_LOGO = (
    "https://raw.githubusercontent.com/aipotheosis-labs/aipolabs-icons/refs/heads/main/apps/aci.png"
)

# Ops
SENTRY_DSN = check_and_get_env_variable("CONTROL_PLANE_SENTRY_DSN")

# SUBSCRIPTION
SUBSCRIPTION_ENABLED = (
    True
    if check_and_get_env_variable("CONTROL_PLANE_SUBSCRIPTION_ENABLED", default="false").lower()
    == "true"
    else False
)
SUBSCRIPTION_STRIPE_SECRET_KEY = check_and_get_env_variable(
    "CONTROL_PLANE_SUBSCRIPTION_STRIPE_SECRET_KEY", ""
)
SUBSCRIPTION_STRIPE_WEBHOOK_SECRET = check_and_get_env_variable(
    "CONTROL_PLANE_SUBSCRIPTION_STRIPE_WEBHOOK_SECRET", ""
)
SUBSCRIPTION_SUCCESS_URL = check_and_get_env_variable("CONTROL_PLANE_SUBSCRIPTION_SUCCESS_URL", "")
SUBSCRIPTION_CANCEL_URL = check_and_get_env_variable("CONTROL_PLANE_SUBSCRIPTION_CANCEL_URL", "")
