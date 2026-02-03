from datetime import datetime
from typing import Literal
from uuid import UUID

from pydantic import BaseModel, ConfigDict, Field, ValidationInfo, field_validator

DEFAULT_FREE_PLAN_CODE = "GATE22_FREE_PLAN"


class SubscriptionPlanPublic(BaseModel):
    plan_code: str
    display_name: str
    max_seats_for_subscription: int | None
    max_custom_mcp_servers: int | None
    log_retention_days: int | None


class OrganizationUsage(BaseModel):
    seat_count: int
    custom_mcp_servers_count: int


class SubscriptionPlanCreate(BaseModel):
    plan_code: str
    display_name: str
    is_public: bool
    stripe_price_id: str | None
    max_seats_for_subscription: int | None
    max_custom_mcp_servers: int | None
    log_retention_days: int | None

    @field_validator(
        "max_seats_for_subscription",
        "max_custom_mcp_servers",
        "log_retention_days",
    )
    @classmethod
    def validate_positive_integers(cls, v: int | None, info: ValidationInfo) -> int | None:
        if v is not None and v < 1:
            raise ValueError(f"{info.field_name} must be greater than 0")
        return v


class Entitlement(BaseModel):
    seat_count: int | None
    max_custom_mcp_servers: int | None
    log_retention_days: int | None


class SubscriptionPublic(BaseModel):
    plan_code: str
    seat_count: int
    current_period_start: datetime
    current_period_end: datetime
    cancel_at_period_end: bool


class SubscriptionStatusPublic(BaseModel):
    subscription: SubscriptionPublic | None
    entitlement: Entitlement
    usage: OrganizationUsage
    is_usage_exceeded: bool


class SubscriptionSeatChangeRequest(BaseModel):
    seat_count: int = Field(..., gt=0)


class SubscriptionPlanChangeRequest(BaseModel):
    plan_code: str
    seat_count: int = Field(..., gt=0)


class SubscriptionCheckout(BaseModel):
    url: str


class SubscriptionResult(BaseModel):
    subscription_id: str


class SubscriptionCancellation(BaseModel):
    subscription_id: str


class OrganizationSubscriptionUpsert(BaseModel):
    subscription_plan_id: UUID
    seat_count: int
    # See https://docs.stripe.com/billing/subscriptions/overview?locale=en-GB
    stripe_subscription_status: Literal[
        "active",
        "trialing",
        "past_due",
        "canceled",
        "incomplete_expired",
        "incomplete",
        "paused",
        "unpaid",
    ]
    stripe_subscription_id: str
    stripe_subscription_item_id: str
    current_period_start: datetime
    current_period_end: datetime
    cancel_at_period_end: bool
    subscription_start_date: datetime


##############################
# Stripe Event Data
#############################
class StripeEventData(BaseModel):
    model_config = ConfigDict(extra="allow")

    id: str  # subscription id
    status: str
    metadata: dict[str, str]


class StripeEventDataObject(BaseModel):
    model_config = ConfigDict(extra="allow")
    object: StripeEventData


class StripeWebhookEvent(BaseModel):
    model_config = ConfigDict(extra="allow")

    id: str  # event id
    type: str
    data: StripeEventDataObject


class StripeVerifySessionRequest(BaseModel):
    session_id: str
