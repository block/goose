from stripe import StripeClient

from aci.control_plane import config

_stripe_client_instance = None


def get_stripe_client() -> StripeClient:
    global _stripe_client_instance
    if _stripe_client_instance is None:
        if config.SUBSCRIPTION_STRIPE_SECRET_KEY is None:
            raise ValueError("SUBSCRIPTION_STRIPE_SECRET_KEY is not set")
        _stripe_client_instance = StripeClient(config.SUBSCRIPTION_STRIPE_SECRET_KEY)
    return _stripe_client_instance
