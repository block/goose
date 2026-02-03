import contextvars

request_id_ctx_var = contextvars.ContextVar[str]("request_id", default="unknown")
