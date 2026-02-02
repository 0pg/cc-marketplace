"""Authentication module for user validation."""

from typing import Optional
from dataclasses import dataclass
import jwt
from .types import Claims, TokenConfig


@dataclass
class AuthResult:
    """Result of authentication attempt."""
    success: bool
    user_id: Optional[str]
    error: Optional[str] = None


def validate_token(token: str, secret: str) -> Claims:
    """
    Validate a JWT token and return claims.

    Args:
        token: The JWT token to validate
        secret: The secret key for verification

    Returns:
        Claims object with user information

    Raises:
        jwt.ExpiredSignatureError: If token is expired
        jwt.InvalidTokenError: If token is invalid
    """
    try:
        payload = jwt.decode(token, secret, algorithms=['HS256'])
        return Claims(**payload)
    except jwt.ExpiredSignatureError:
        raise
    except jwt.InvalidTokenError:
        raise


def generate_token(user_id: str, role: str, config: TokenConfig) -> str:
    """Generate a new JWT token for a user."""
    payload = {
        'user_id': user_id,
        'role': role,
    }
    return jwt.encode(payload, config.secret, algorithm='HS256')


def _internal_helper(data: dict) -> bool:
    """Internal helper function - not exported."""
    return bool(data)


class TokenManager:
    """Manages token lifecycle."""

    def __init__(self, config: TokenConfig):
        self.config = config

    def create(self, user_id: str, role: str) -> str:
        """Create a new token."""
        return generate_token(user_id, role, self.config)

    def verify(self, token: str) -> Claims:
        """Verify and decode a token."""
        return validate_token(token, self.config.secret)
