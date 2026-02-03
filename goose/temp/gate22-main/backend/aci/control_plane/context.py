import contextvars

request_id_ctx_var = contextvars.ContextVar[str | None]("request_id", default="unknown")
