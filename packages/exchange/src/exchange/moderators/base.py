from abc import ABC, abstractmethod
from goose.notifier import Notifier

class Moderator(ABC):
    def __init__(self, notifier: Notifier | None = None) -> None:
        self.notifier = notifier

    @abstractmethod
    def rewrite(self, exchange: type["exchange.exchange.Exchange"]) -> None:  # noqa: F821
        pass