from fastapi import FastAPI
from fastapi.routing import APIRoute
from pythonjsonlogger.json import JsonFormatter
from starlette.middleware.sessions import SessionMiddleware
from starlette.requests import Request
from starlette.responses import JSONResponse

from aci.common.enums import Environment
from aci.common.logging_setup import setup_logging
from aci.common.sentry import setup_sentry
from aci.virtual_mcp import config
from aci.virtual_mcp.exceptions import VirtualMCPException
from aci.virtual_mcp.middleware.interceptor import (
    InterceptorMiddleware,
    RequestContextFilter,
)
from aci.virtual_mcp.routes import (
    health,
    mcp,
)

if config.ENVIRONMENT == Environment.LOCAL:
    formatter = None
else:
    formatter = JsonFormatter(
        "{levelname} {asctime} {name} {message}",
        style="{",
        rename_fields={"asctime": "timestamp", "name": "file", "levelname": "level"},
    )

if config.ENVIRONMENT != Environment.LOCAL:
    setup_sentry(config.SENTRY_DSN, config.ENVIRONMENT)


setup_logging(
    formatter=formatter,
    aci_log_level=config.LOG_LEVEL,
    filters=[RequestContextFilter()],
)


def custom_generate_unique_id(route: APIRoute) -> str:
    return f"{route.tags[0]}-{route.name}"


app = FastAPI(
    title=config.APP_TITLE,
    root_path=config.APP_ROOT_PATH,
    docs_url=config.APP_DOCS_URL,
    redoc_url=config.APP_REDOC_URL,
    openapi_url=config.APP_OPENAPI_URL,
    generate_unique_id_function=custom_generate_unique_id,
)


@app.exception_handler(VirtualMCPException)
async def global_exception_handler(request: Request, exc: VirtualMCPException) -> JSONResponse:
    return JSONResponse(
        status_code=exc.error_code,
        content={"error": f"{exc.title}, {exc.message}" if exc.message else exc.title},
    )


# NOTE: cors middleware is not needed
app.add_middleware(
    SessionMiddleware,
    secret_key=config.SESSION_SECRET_KEY,
)
app.add_middleware(InterceptorMiddleware)


app.include_router(
    health.router,
    prefix=config.ROUTER_PREFIX_HEALTH,
    tags=[config.ROUTER_PREFIX_HEALTH.split("/")[-1]],
)

app.include_router(
    mcp.router,
    prefix=config.ROUTER_PREFIX_MCP,
    tags=[config.ROUTER_PREFIX_MCP.split("/")[-1]],
)
