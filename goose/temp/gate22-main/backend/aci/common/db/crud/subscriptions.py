from datetime import UTC, datetime
from typing import Literal, overload
from uuid import UUID

from sqlalchemy import delete, select, update
from sqlalchemy.orm import Session

from aci.common.db.sql_models import (
    OrganizationSubscription,
    SubscriptionPlan,
    SubscriptionStripeEventLogs,
)
from aci.common.schemas.subscription import (
    DEFAULT_FREE_PLAN_CODE,
    OrganizationSubscriptionUpsert,
    StripeWebhookEvent,
    SubscriptionPlanCreate,
)

"""
CRUD methods for subscription database.
Since there are limited operations for subscription database, we combine all the CRUD methods here
instead of creating a separate file for each model.
"""


########################################
# Subscription Plan
########################################
@overload
def get_free_plan(
    db_session: Session,
    throw_error_if_not_found: Literal[True],
) -> SubscriptionPlan: ...


@overload
def get_free_plan(
    db_session: Session,
    throw_error_if_not_found: Literal[False],
) -> SubscriptionPlan | None: ...


def get_free_plan(
    db_session: Session,
    throw_error_if_not_found: bool,
) -> SubscriptionPlan | None:
    statement = select(SubscriptionPlan).where(
        SubscriptionPlan.plan_code == DEFAULT_FREE_PLAN_CODE,
        SubscriptionPlan.archived_at.is_(None),
    )
    plan: SubscriptionPlan | None = db_session.execute(statement).scalar_one_or_none()
    if plan is None:
        if throw_error_if_not_found:
            raise Exception("Free plan not found")
    return plan


def get_all_public_plans(
    db_session: Session,
) -> list[SubscriptionPlan]:
    statement = select(SubscriptionPlan).where(
        SubscriptionPlan.archived_at.is_(None), SubscriptionPlan.is_public.is_(True)
    )
    return list(db_session.execute(statement).scalars().all())


def get_active_plan_by_plan_code(
    db_session: Session,
    plan_code: str,
) -> SubscriptionPlan | None:
    statement = select(SubscriptionPlan).where(
        SubscriptionPlan.plan_code == plan_code, SubscriptionPlan.archived_at.is_(None)
    )
    return db_session.execute(statement).scalar_one_or_none()


def get_plan_by_id(
    db_session: Session,
    plan_id: UUID,
) -> SubscriptionPlan | None:
    statement = select(SubscriptionPlan).where(SubscriptionPlan.id == plan_id)
    return db_session.execute(statement).scalar_one_or_none()


def insert_subscription_plan(
    db_session: Session, plan_data: SubscriptionPlanCreate
) -> SubscriptionPlan:
    plan = SubscriptionPlan(**plan_data.model_dump())
    db_session.add(plan)
    db_session.flush()
    db_session.refresh(plan)
    return plan


########################################
# Organization Subscription
########################################
def upsert_organization_subscription(
    db_session: Session, organization_id: UUID, upsert_data: OrganizationSubscriptionUpsert
) -> OrganizationSubscription:
    statement = select(OrganizationSubscription).where(
        OrganizationSubscription.organization_id == organization_id
    )
    organization_subscription = db_session.execute(statement).scalar_one_or_none()
    if organization_subscription is None:
        organization_subscription = OrganizationSubscription(
            organization_id=organization_id, **upsert_data.model_dump()
        )
        db_session.add(organization_subscription)
    else:
        for key, value in upsert_data.model_dump(exclude_unset=True).items():
            setattr(organization_subscription, key, value)
    db_session.flush()
    db_session.refresh(organization_subscription)
    return organization_subscription


def get_organization_subscription(
    db_session: Session, organization_id: UUID
) -> OrganizationSubscription | None:
    statement = select(OrganizationSubscription).where(
        OrganizationSubscription.organization_id == organization_id
    )
    return db_session.execute(statement).scalar_one_or_none()


def get_organization_subscription_by_stripe_subscription_id(
    db_session: Session, stripe_subscription_id: str
) -> OrganizationSubscription | None:
    statement = select(OrganizationSubscription).where(
        OrganizationSubscription.stripe_subscription_id == stripe_subscription_id
    )
    return db_session.execute(statement).scalar_one_or_none()


def delete_organization_subscription(db_session: Session, stripe_subscription_id: str) -> None:
    statement = delete(OrganizationSubscription).where(
        OrganizationSubscription.stripe_subscription_id == stripe_subscription_id
    )
    db_session.execute(statement)
    db_session.flush()


def insert_stripe_event_log(
    db_session: Session,
    stripe_event: StripeWebhookEvent,
) -> SubscriptionStripeEventLogs:
    stripe_event_log = SubscriptionStripeEventLogs(
        stripe_event_id=stripe_event.id,
        type=stripe_event.type,
        payload=stripe_event.model_dump(),
        received_at=datetime.now(UTC),
        process_attempts=0,
        processed_at=None,
        process_error=None,
    )
    db_session.add(stripe_event_log)
    db_session.flush()
    db_session.refresh(stripe_event_log)
    return stripe_event_log


def get_stripe_event_log_by_stripe_event_id(
    db_session: Session, stripe_event_id: str
) -> SubscriptionStripeEventLogs | None:
    statement = select(SubscriptionStripeEventLogs).where(
        SubscriptionStripeEventLogs.stripe_event_id == stripe_event_id
    )
    return db_session.execute(statement).scalar_one_or_none()


def log_process_attempt(
    db_session: Session,
    stripe_event_id: str,
    process_error: str | None,
    processed_at: datetime | None,
) -> None:
    statement = (
        update(SubscriptionStripeEventLogs)
        .where(SubscriptionStripeEventLogs.stripe_event_id == stripe_event_id)
        .values(
            process_attempts=SubscriptionStripeEventLogs.process_attempts + 1,
            process_error=process_error,
            processed_at=processed_at,
        )
    )
    db_session.execute(statement)
    db_session.flush()
