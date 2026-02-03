from uuid import uuid4

from sqlalchemy.orm import Session

from aci.common.db import crud
from aci.common.db.sql_models import (
    Organization,
    OrganizationSubscription,
    SubscriptionPlan,
)
from aci.common.logging_setup import get_logger
from aci.common.schemas.subscription import (
    SubscriptionCancellation,
    SubscriptionCheckout,
    SubscriptionResult,
)
from aci.control_plane import config
from aci.control_plane.exceptions import (
    StripeOperationError,
)
from aci.control_plane.services.subscription.stripe_client import get_stripe_client

logger = get_logger(__name__)


def create_stripe_subscription(
    db_session: Session,
    organization: Organization,
    plan: SubscriptionPlan,
    seat_count: int,
) -> SubscriptionCheckout:
    """
    Create a stripe subscription. It will create and return a stripe checkout session.
    It will also create a stripe customer id if it is not set.
    Note: This function will NOT update any data in the database. The updated data will be sent
    asynchronously from Stripe webhook and handled by the stripe_event_handler.

    Returns:
        SubscriptionCheckout Object with the stripe checkout session url.
    """

    # Create stripe customer id if it is not set (first time stripe subscription)
    if organization.stripe_customer_id is None:
        stripe_customer = get_stripe_client().customers.create({"name": organization.name})
        logger.info(f"Stripe customer created: {stripe_customer.id}")
        # TODO: put email / org name as the customer metadata for easier retrieval
        crud.organizations.update_organization_stripe_customer_id(
            db_session=db_session,
            organization=organization,
            stripe_customer_id=stripe_customer.id,
        )
        stripe_customer_id = stripe_customer.id
    else:
        stripe_customer_id = organization.stripe_customer_id

    # Should not happen, we should have checked this in caller
    if plan.stripe_price_id is None:
        logger.error(f"Subscription plan {plan.plan_code} has no stripe price id")
        raise StripeOperationError(f"Subscription plan {plan.plan_code} has no stripe price id")

    # Checkout the subscription
    idempotency_key = f"{organization.id}-{plan.plan_code}-{uuid4()!s}"

    # Checkout session will created the subscription with `collection_method=automatic_collection`
    # by default.
    stripe_checkout_session = get_stripe_client().checkout.sessions.create(
        {
            "customer": stripe_customer_id,
            "mode": "subscription",
            "ui_mode": "hosted",
            "line_items": [
                {
                    "price": plan.stripe_price_id,
                    "quantity": seat_count,
                }
            ],
            "subscription_data": {
                "metadata": {
                    "organization_id": str(organization.id),
                    "plan_code": plan.plan_code,
                }
            },
            # Stripe will replace the placeholder {CHECKOUT_SESSION_ID} with the actual session id
            "success_url": f"{config.SUBSCRIPTION_SUCCESS_URL}?session_id={{CHECKOUT_SESSION_ID}}",
            "cancel_url": config.SUBSCRIPTION_CANCEL_URL,
        },
        {
            "idempotency_key": idempotency_key,
        },
    )

    logger.info(f"Stripe checkout session created: {stripe_checkout_session.id}")

    if stripe_checkout_session.url is None:
        logger.error(f"Stripe checkout session has no url: {stripe_checkout_session.id}")
        raise StripeOperationError(
            f"Stripe checkout session has no url: {stripe_checkout_session.id}"
        )

    return SubscriptionCheckout(url=stripe_checkout_session.url)


def update_stripe_subscription(
    db_session: Session,
    organization: Organization,
    plan: SubscriptionPlan,
    seat_count: int,
    existing_subscription: OrganizationSubscription,
) -> SubscriptionResult:
    """
    Update the stripe subscription. It will NOT return a stripe checkout session, the update will
    be effective immediately, and stripe will charge / refund the price difference pro-rated
    immediately.

    Note: This function will NOT update any data in the database. The updated data will be sent
    asynchronously from Stripe webhook and handled by the stripe_event_handler.

    Returns:
        SubscriptionResult Object with the stripe subscription id.
    """
    if organization.stripe_customer_id is None:
        logger.error(f"Organization {organization.id} has no stripe customer id")
        raise StripeOperationError(f"Organization {organization.id} has no stripe customer id")

    # Should not happen, we should have checked this in caller
    if plan.stripe_price_id is None:
        logger.error(f"Subscription plan {plan.plan_code} has no stripe price id")
        raise StripeOperationError(f"Subscription plan {plan.plan_code} has no stripe price id")

    # If the price difference is positive, stripe will create a proration and charge the price
    # difference.
    # If the price difference is negative, stripe will issue credit to the customer and the price
    # difference will be deducted from the next period.
    # See https://docs.stripe.com/billing/subscriptions/prorations for details
    subscription = get_stripe_client().subscriptions.update(
        existing_subscription.stripe_subscription_id,
        {
            "items": [
                {
                    "id": existing_subscription.stripe_subscription_item_id,
                    "price": plan.stripe_price_id,
                    "quantity": seat_count,
                }
            ],
            "proration_behavior": "always_invoice",
            "metadata": {
                "organization_id": str(organization.id),
                "plan_code": plan.plan_code,
            },
        },
    )
    logger.info(f"Stripe subscription updated: {existing_subscription.stripe_subscription_id}")

    return SubscriptionResult(subscription_id=subscription.id)


def cancel_stripe_subscription(
    subscription: OrganizationSubscription,
) -> SubscriptionCancellation:
    """
    Cancel the stripe subscription. It will NOT return a stripe checkout session, the cancellation
    will be effective at the end of the current period.

    Note: This function will NOT update any data in the database. The updated data will be sent
    asynchronously from Stripe webhook and handled by the stripe_event_handler.
    """
    # stripe_client.subscriptions.cancel(subscription.stripe_subscription_id)

    # Set the cancellation at the end of the current period. Stripe will emit an event about
    # cancellation scheduled, and then another cancellation event during the period end.
    get_stripe_client().subscriptions.update(
        subscription.stripe_subscription_id,
        {
            "cancel_at_period_end": True,
        },
    )

    # Do not update the subscription in the database, it will be updated by the stripe webhook
    return SubscriptionCancellation(subscription_id=subscription.stripe_subscription_id)
