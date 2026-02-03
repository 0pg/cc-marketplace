"""Union-based state machine for Python code analyzer."""

from dataclasses import dataclass
from typing import Union


@dataclass
class Idle:
    """Initial idle state."""
    pass


@dataclass
class Loading:
    """Loading state with progress."""
    progress: int


@dataclass
class Loaded:
    """Loaded state with data."""
    data: str


@dataclass
class Error:
    """Error state with message."""
    message: str


# State type alias using Union
State = Union[Idle, Loading, Loaded, Error]

# Alternative: using | syntax (Python 3.10+)
# State = Idle | Loading | Loaded | Error
