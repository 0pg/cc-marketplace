# Python export candidates fixture
# Tests: UPPER_CASE constants, type aliases

from typing import Union, Optional, List

MAX_RETRIES = 3
DEFAULT_TIMEOUT = 30
API_BASE_URL = "https://api.example.com"

UserId = Union[str, int]
TokenResult = Optional[dict]
ItemList = List[str]

# Should NOT be captured (not UPPER_CASE, not PascalCase type alias)
_internal_counter = 0
simple_var = "hello"


class UserConfig:
    pass


def process_item(item: str) -> bool:
    return len(item) > 0
