package com.example.token

/**
 * Exception thrown when token has expired.
 */
class TokenExpiredException(
    message: String,
    cause: Throwable? = null
) : Exception(message, cause)

/**
 * Exception thrown when token is invalid.
 */
class InvalidTokenException(
    message: String,
    cause: Throwable? = null
) : Exception(message, cause)
