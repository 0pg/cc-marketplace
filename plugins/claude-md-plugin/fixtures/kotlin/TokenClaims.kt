package com.example.token

/**
 * JWT token claims structure.
 */
data class TokenClaims(
    val userId: String,
    val role: Role,
    val exp: Long,
    val iat: Long
)
