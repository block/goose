from abc import ABC, abstractmethod
from typing import Callable, Type


class Observer(ABC):
    @abstractmethod
    def initialize(self) -> None:
        pass

    @abstractmethod
    def observe_wrapper(*args, **kwargs) -> Callable:  # noqa: ANN002, ANN003
        pass

    @abstractmethod
    def finalize(self) -> None:
        pass


class ObserverManager:
    _instance = None
    _observers: list[Observer] = []

    @classmethod
    def get_instance(cls: Type["ObserverManager"]) -> "ObserverManager":
        if cls._instance is None:
            cls._instance = cls()
        return cls._instance

    def initialize(self, tracing: bool, observers: list[Observer]) -> None:
        from exchange.observers.langfuse import LangfuseObserver

        self._observers = observers
        for observer in self._observers:
            # LangfuseObserver has special behavior when tracing is _dis_abled.
            # Consider refactoring to make this less special-casey if that's common.
            if isinstance(observer, LangfuseObserver) and not tracing:
                observer.initialize_with_disabled_tracing()
            elif tracing:
                observer.initialize()

    def finalize(self) -> None:
        for observer in self._observers:
            observer.finalize()
