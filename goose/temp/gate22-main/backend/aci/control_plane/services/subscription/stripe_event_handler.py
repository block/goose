from datetime import UTC, datetime

from sqlalchemy.orm import Session
from stripe import Subscription, SubscriptionItem

from aci.common.db import crud
from aci.common.db.sql_models import Organization
from aci.common.exceptions import OrganizationNotFoundError
from aci.common.logging_setup import get_logger
from aci.common.schemas.subscription import (
    OrganizationSubscriptionUpsert,
    StripeWebhookEvent,
)
from aci.control_plane.exceptions import StripeOperationError
from aci.control_plane.services.subscription.stripe_client import get_stripe_client

logger = get_logger(__name__)


def handle_stripe_event(db_session: Session, event_id: str) -> None:
    """
    The entry function to handle any stripe events. This function MUST be IDEMPOTENT and can be
    called multiple times for a same event.
    """
    event_data = get_stripe_client().events.retrieve(event_id)
    event = StripeWebhookEvent.model_validate(event_data)

    # Only handle subscription events
    if not event.type.startswith("customer.subscription."):
        logger.warning(f"Unsupported Stripe event type {event.type}. Ignore.")
        return

    subscription = event.data.object
    logger.info(
        f"Stripe webhook received for event {event.type}, event id {event.id}, status {subscription.status}"  # noqa: E501
    )

    # Log Stripe event
    stripe_event_log = crud.subscriptions.get_stripe_event_log_by_stripe_event_id(
        db_session=db_session,
        stripe_event_id=event.id,
    )
    if stripe_event_log is None:
        logger.info("Stripe event log not found, inserting...")
        stripe_event_log = crud.subscriptions.insert_stripe_event_log(
            db_session=db_session,
            stripe_event=event,
        )

    if stripe_event_log.processed_at is not None:
        logger.info(f"Stripe event {event.id} already processed")
        return

    # Process the stripe event
    try:
        _process_subscription_event(db_session=db_session, subscription_id=subscription.id)
        crud.subscriptions.log_process_attempt(
            db_session=db_session,
            stripe_event_id=event.id,
            process_error=None,
            processed_at=datetime.now(UTC),
        )
    except Exception as e:
        logger.error(f"Stripe event {event.id} failed to process: {e}")
        # Log the error and do not mark the event as processed
        crud.subscriptions.log_process_attempt(
            db_session=db_session,
            stripe_event_id=event.id,
            process_error=str(e),
            processed_at=None,
        )
        raise e


def _process_subscription_event(db_session: Session, subscription_id: str) -> None:
    """
    Process a subscription event from Stripe. This function MUST be IDEMPOTENT and can be
    called multiple times for a same event.
    """
    subscription_data = get_stripe_client().subscriptions.retrieve(subscription_id)

    logger.info(f"Subscription id: {subscription_data.id}, status: {subscription_data.status}")

    # Get the organization from the customer id
    organization = crud.organizations.get_organization_by_stripe_customer_id(
        db_session=db_session,
        stripe_customer_id=subscription_data.customer
        if isinstance(subscription_data.customer, str)
        else subscription_data.customer.id,
    )
    if organization is None:
        logger.error(
            f"Failed to map organization by stripe customer id {subscription_data.customer}"
        )
        raise OrganizationNotFoundError()

    items: list[SubscriptionItem] = subscription_data.get("items", {}).get("data", [])

    # Check if the subscription has only one item
    if len(items) != 1:
        logger.error(f"Expected 1 item in subscription, got {len(items)}")
        raise StripeOperationError(f"Expected 1 item in subscription, got {len(items)}")

    subscription_item = items[0]

    match subscription_data.status:
        # "incomplete": Stripe created the subscription but failed to collect the payment yet.
        # We should not treat it as valid subscription yet.
        case "incomplete":
            pass

        # "active": Stripe successfully collected the payment.
        # "past_due": Stripe has a grace period for the payment.
        # These two are active states. We treat them as valid subscription.
        case "active" | "past_due":
            _upsert_customer_subscription(
                db_session=db_session,
                organization=organization,
                stripe_subscription=subscription_data,
                stripe_subscription_item=subscription_item,
            )

        # "canceled": Stripe canceled the subscription.
        # "incomplete_expired": Stripe failed to collect the payment for a couple times.
        #
        # These two are terminal states. We should remove the subscription from the database if it
        # exists.
        case "canceled" | "incomplete_expired":
            _remove_customer_subscription(
                db_session=db_session, organization=organization, subscription=subscription_data
            )

        #
        # "trialing": Subscription is in the trial period. We don't support trial.
        # "paused": This state when trial ends without payment method. So this is unexpected for us
        #           as we do not support trial.
        # "unpaid": Stripe tries few times and failed to charge the customer. This is unexpected
        #           for us since directly cancel the subscription in that case. Check "Manage failed
        #           payments" from Stripe Dashboard.
        case "paused" | "unpaid" | "trialing":
            logger.error(f"Unsupported subscription status {subscription_data.status}")
            raise StripeOperationError(
                f"Unsupported subscription status {subscription_data.status}"
            )


def _upsert_customer_subscription(
    db_session: Session,
    organization: Organization,
    stripe_subscription: Subscription,
    stripe_subscription_item: SubscriptionItem,
) -> None:
    plan_code = stripe_subscription.metadata.get("plan_code")
    if plan_code is None:
        logger.error("Missing plan code in subscription metadata")
        raise StripeOperationError("Missing plan code in subscription metadata")

    plan = crud.subscriptions.get_active_plan_by_plan_code(
        db_session=db_session,
        plan_code=plan_code,
    )
    if plan is None:
        logger.error(f"Failed to map plan by stripe price id {stripe_subscription_item.price.id}")
        raise StripeOperationError(
            f"Failed to map plan by stripe price id {stripe_subscription_item.price.id}"
        )

    logger.info(f"Upserting organization subscription for {organization.id}...")

    if stripe_subscription_item.quantity is None:
        # Unexpected, the quantity should be returned by Stripe
        logger.error("Missing quantity in subscription item")
        raise StripeOperationError("Missing quantity in subscription item")

    crud.subscriptions.upsert_organization_subscription(
        db_session=db_session,
        organization_id=organization.id,
        upsert_data=OrganizationSubscriptionUpsert(
            subscription_plan_id=plan.id,
            stripe_subscription_id=stripe_subscription.id,
            stripe_subscription_item_id=stripe_subscription_item.id,
            seat_count=stripe_subscription_item.quantity,
            stripe_subscription_status=stripe_subscription.status,
            current_period_start=datetime.fromtimestamp(
                stripe_subscription_item.current_period_start, tz=UTC
            ),
            current_period_end=datetime.fromtimestamp(
                stripe_subscription_item.current_period_end, tz=UTC
            ),
            cancel_at_period_end=stripe_subscription.cancel_at_period_end,
            subscription_start_date=datetime.fromtimestamp(stripe_subscription.start_date, tz=UTC),
        ),
    )


def _remove_customer_subscription(
    db_session: Session, organization: Organization, subscription: Subscription
) -> None:
    organization_subscription = (
        crud.subscriptions.get_organization_subscription_by_stripe_subscription_id(
            db_session=db_session,
            stripe_subscription_id=subscription.id,
        )
    )
    if organization_subscription is not None:
        logger.info(f"Deleting organization subscription for organization {organization.id}...")
        crud.subscriptions.delete_organization_subscription(
            db_session=db_session,
            stripe_subscription_id=subscription.id,
        )
