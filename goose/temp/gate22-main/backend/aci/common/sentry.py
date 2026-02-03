import sentry_sdk

from aci.common.enums import Environment


def setup_sentry(dsn: str, environment: Environment) -> None:
    sentry_sdk.init(
        dsn=dsn,
        environment=environment,
        # Add data like request headers and IP for users,
        # see https://docs.sentry.io/platforms/python/data-management/data-collected/ for more info
        send_default_pii=True,
        traces_sample_rate=1.0,  # TODO: adjust later
        _experiments={
            "continuous_profiling_auto_start": True,
        },
    )
