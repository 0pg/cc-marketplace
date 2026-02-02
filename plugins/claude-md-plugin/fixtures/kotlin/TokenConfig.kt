package com.example.token

/**
 * Token configuration settings.
 */
data class TokenConfig(
    val secret: String,
    val expiresInSeconds: Long,
    val issuer: String? = null
)
