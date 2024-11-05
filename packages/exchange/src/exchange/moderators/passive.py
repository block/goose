from exchange.moderators.base import Moderator


class PassiveModerator(Moderator):
    """A no-op moderator that makes no modifications to the exchange.

    This moderator is useful as a base case when no moderation is needed,
    or when temporarily disabling other moderators for testing purposes.
    """

    def rewrite(self, _: type["exchange.exchange.Exchange"]) -> None:  # noqa: F821
        pass
