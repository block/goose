from exchange.observers import ObserverManager, observe_wrapper
from exchange.observers.base import Observer


class MockObserver(Observer):
    def __init__(self):
        self.initialized = False
        self.args = None
        self.kwargs = None
        self.finalized = False

    def initialize(self):
        pass

    def observe_wrapper(self, *args, **kwargs):
        def wrapper(func):
            self.args = args
            self.kwargs = kwargs
            return func

        return wrapper

    def finalize(self):
        pass


def test_wrapper_is_invoked():
    manager = ObserverManager.get_instance()
    mock_observer = MockObserver()
    manager.initialize(True, [mock_observer])

    @observe_wrapper("arg0", arg1="arg2")
    def wrapped(x: int, y: int) -> int:
        return x + y

    # code in decorator hasn't run yet
    assert mock_observer.args is None
    assert mock_observer.kwargs is None

    ret_val = wrapped(2, 3)
    assert ret_val == 5

    # decorator has been run since `wrapped` was called
    assert mock_observer.args == ("arg0",)
    assert mock_observer.kwargs == {"arg1": "arg2"}


def test_multiple_wrappers():
    manager = ObserverManager.get_instance()
    mock_observer_1 = MockObserver()
    mock_observer_2 = MockObserver()
    manager.initialize(True, [mock_observer_1, mock_observer_2])

    @observe_wrapper("arg0")
    def wrapped(x: int, y: int) -> int:
        return x + y

    wrapped(2, 3)

    assert mock_observer_1.args == ("arg0",)
    assert mock_observer_2.args == ("arg0",)
