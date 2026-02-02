"""Type definitions for authentication module."""

from dataclasses import dataclass
from typing import Optional


@dataclass
class Claims:
    """JWT token claims structure."""
    user_id: str
    role: str
    exp: Optional[int] = None
    iat: Optional[int] = None


@dataclass
class TokenConfig:
    """Configuration for token generation."""
    secret: str
    expires_in: int = 3600
    issuer: Optional[str] = None
