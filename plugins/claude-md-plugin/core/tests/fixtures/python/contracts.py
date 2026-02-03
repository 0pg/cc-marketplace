"""Contract examples for Python code analyzer."""

from typing import Dict, Any
from .types import Claims


class InvalidTokenError(Exception):
    """Raised when a token is invalid."""
    pass


class ValidationError(Exception):
    """Raised when validation fails."""
    pass


def validate_token(token: str) -> Claims:
    """
    Validates a JWT token and returns the claims.

    Args:
        token: JWT token string (must be non-empty)

    Returns:
        Claims object with valid userId

    Raises:
        InvalidTokenError: if token format is invalid or expired
    """
    if not token:
        raise InvalidTokenError("Token is required")
    # ... validation logic
    return Claims(user_id="user123", role="admin")


def process_order(order: Dict[str, Any]) -> Dict[str, Any]:
    """
    Processes an order and returns a receipt.

    Args:
        order: Order dictionary with id and items

    Returns:
        Receipt dictionary with orderId and total

    Raises:
        ValidationError: if order is invalid
    """
    if not order.get("id"):
        raise ValidationError("Order ID required")
    if not order.get("items") or len(order["items"]) == 0:
        raise ValidationError("Items required")
    # ... process order
    return {"orderId": order["id"], "total": 100}


def _internal_helper(data: str) -> str:
    """Private helper function - should not be exported."""
    return data.strip()
