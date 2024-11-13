"""
Langfuse Observer

This observer provides integration with Langfuse, a tool for monitoring and tracing LLM applications.

Usage:
    Include "langfuse" in your profile's list of observers to enable Langfuse integration.
    It automatically checks for Langfuse credentials in the .env.langfuse file and for a running Langfuse server.
    If these are found, it will set up the necessary client and context for tracing.

Note:
    Run setup_langfuse.sh which automates the steps for running local Langfuse.
"""

import logging
import os
import sys
from functools import cache, wraps
from io import StringIO
from typing import Callable

from langfuse.decorators import langfuse_context

from exchange.observers.base import Observer

## These are the default configurations for local Langfuse server
## Please refer to .env.langfuse.local file for local langfuse server setup configurations
DEFAULT_LOCAL_LANGFUSE_HOST = "http://localhost:3000"
DEFAULT_LOCAL_LANGFUSE_PUBLIC_KEY = "publickey-local"
DEFAULT_LOCAL_LANGFUSE_SECRET_KEY = "secretkey-local"


@cache
def auth_check() -> bool:
    # Temporarily redirect stdout and stderr to suppress print statements from Langfuse
    temp_stderr = StringIO()
    sys.stderr = temp_stderr

    # Set environment variables if not specified
    os.environ.setdefault("LANGFUSE_PUBLIC_KEY", DEFAULT_LOCAL_LANGFUSE_PUBLIC_KEY)
    os.environ.setdefault("LANGFUSE_SECRET_KEY", DEFAULT_LOCAL_LANGFUSE_SECRET_KEY)
    os.environ.setdefault("LANGFUSE_HOST", DEFAULT_LOCAL_LANGFUSE_HOST)

    auth_val = langfuse_context.auth_check()

    # Restore stderr
    sys.stderr = sys.__stderr__
    return auth_val


class LangfuseObserver(Observer):
    def initialize(self) -> None:
        langfuse_auth = auth_check()
        if langfuse_auth:
            print("Local Langfuse initialized. View your traces at http://localhost:3000")
        else:
            raise RuntimeError(
                "You passed --tracing, but a Langfuse object was not found in the current context. "
                "Please initialize the local Langfuse server and restart Goose."
            )

        langfuse_context.configure(enabled=True)
        self.tracing = True

    def initialize_with_disabled_tracing(self) -> None:
        logging.getLogger("langfuse").setLevel(logging.ERROR)
        langfuse_context.configure(enabled=False)
        self.tracing = False

    def session_id_wrapper(self, func: Callable, session_id: str) -> Callable:
        @wraps(func)  # This will preserve the metadata of 'func'
        def wrapper(*args, **kwargs) -> Callable:  # noqa: ANN002, ANN003
            langfuse_context.update_current_trace(session_id=session_id)
            return func(*args, **kwargs)

        return wrapper

    def observe_wrapper(self, *args, **kwargs) -> Callable:  # noqa: ANN002, ANN003
        def _wrapper(fn: Callable) -> Callable:
            if self.tracing and auth_check():

                @wraps(fn)
                def wrapped_fn(*fargs, **fkwargs) -> Callable:  # noqa: ANN002, ANN003
                    # group all traces under the same session
                    if "session_id" in kwargs:
                        session_id_function = kwargs.pop("session_id")
                        session_id_value = session_id_function(fargs[0])
                        modified_fn = self.session_id_wrapper(fn, session_id_value)
                        return langfuse_context.observe(*args, **kwargs)(modified_fn)(*fargs, **fkwargs)
                    else:
                        return langfuse_context.observe(*args, **kwargs)(fn)(*fargs, **fkwargs)

                return wrapped_fn
            else:
                return fn

        return _wrapper

    def finalize(self) -> None:
        langfuse_context.flush()
