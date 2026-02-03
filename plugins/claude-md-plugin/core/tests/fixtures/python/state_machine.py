"""State machine example for Python code analyzer."""

from enum import Enum
from typing import Optional, Any


class State(Enum):
    """State machine states."""
    IDLE = "idle"
    LOADING = "loading"
    LOADED = "loaded"
    ERROR = "error"


class StateContext:
    """Context for the state machine."""

    def __init__(self):
        self.state: State = State.IDLE
        self.data: Optional[Any] = None
        self.error: Optional[Exception] = None


class ResourceLoader:
    """
    Resource loader with explicit lifecycle methods.

    Lifecycle:
        1. init() - Initialize the loader
        2. start() - Start loading resources
        3. stop() - Stop the loader
        4. destroy() - Clean up resources
    """

    def __init__(self):
        self._context = StateContext()

    def init(self) -> None:
        """
        Initialize the loader.

        @lifecycle 1
        """
        self._context = StateContext()

    def start(self) -> None:
        """
        Start loading resources.

        @lifecycle 2
        """
        if self._context.state != State.IDLE:
            raise RuntimeError("Can only start from IDLE state")
        self._context.state = State.LOADING

    def stop(self) -> None:
        """
        Stop the loader.

        @lifecycle 3
        """
        self._context.state = State.IDLE
        self._context.data = None

    def destroy(self) -> None:
        """
        Clean up resources.

        @lifecycle 4
        """
        self.stop()
        # Clean up...

    def load(self) -> None:
        """State transition: IDLE -> LOADING"""
        self._context.state = State.LOADING

    def on_success(self, data: Any) -> None:
        """State transition: LOADING -> LOADED"""
        self._context.state = State.LOADED
        self._context.data = data

    def on_error(self, error: Exception) -> None:
        """State transition: LOADING -> ERROR"""
        self._context.state = State.ERROR
        self._context.error = error

    def retry(self) -> None:
        """State transition: ERROR -> IDLE (retry)"""
        if self._context.state == State.ERROR:
            self._context.state = State.IDLE
            self._context.error = None
