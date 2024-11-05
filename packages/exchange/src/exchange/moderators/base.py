from abc import ABC, abstractmethod


class Moderator(ABC):
    """Base class for all moderators that can modify or rewrite exchange messages.

    Moderators are used to process and potentially modify the conversation history
    in an Exchange. Common use cases include truncation, summarization, and other
    message transformations.
    """

    @abstractmethod
    def rewrite(self, exchange: type["exchange.exchange.Exchange"]) -> None:  # noqa: F821
        pass
