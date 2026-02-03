package com.example.token

import io.jsonwebtoken.Jwts

/**
 * Contract examples for Kotlin code analyzer.
 */
class Contracts {

    /**
     * Validates a JWT token and returns the claims.
     *
     * @param token JWT token (must be non-empty)
     * @return TokenClaims or null if validation fails
     * @throws IllegalArgumentException if malformed or expired
     */
    @Throws(IllegalArgumentException::class)
    fun validateToken(token: String): TokenClaims? {
        if (token.isEmpty()) {
            throw IllegalArgumentException("Token is required")
        }

        // ... validation logic
        return TokenClaims("user123", "admin")
    }

    /**
     * Processes an order and returns a receipt.
     *
     * @param order Order object with id and items
     * @return Receipt with orderId and total
     * @throws ValidationException if order is invalid
     */
    @Throws(ValidationException::class)
    fun processOrder(order: Order): Receipt {
        if (order.id.isNullOrEmpty()) {
            throw ValidationException("Order ID required")
        }
        if (order.items.isNullOrEmpty()) {
            throw ValidationException("Items required")
        }

        // ... process order
        return Receipt(order.id, 100)
    }

    /**
     * Private helper method.
     */
    private fun mapToTokenClaims(data: Any): TokenClaims? {
        return null
    }

    /**
     * Private validation method.
     */
    private fun isTokenExpired(token: String): Boolean {
        return false
    }
}

/**
 * Token claims data class.
 */
data class TokenClaims(
    val userId: String,
    val role: String
)

/**
 * Order data class.
 */
data class Order(
    val id: String?,
    val items: List<String>?
)

/**
 * Receipt data class.
 */
data class Receipt(
    val orderId: String,
    val total: Int
)

/**
 * Exception for validation errors.
 */
class ValidationException(message: String) : Exception(message)
