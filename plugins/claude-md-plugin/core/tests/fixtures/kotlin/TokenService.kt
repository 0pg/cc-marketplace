package com.example.token

import io.jsonwebtoken.Claims
import io.jsonwebtoken.ExpiredJwtException
import io.jsonwebtoken.Jwts
import io.jsonwebtoken.SignatureAlgorithm
import java.util.Date

/**
 * Service for JWT token operations.
 */
class TokenService(private val config: TokenConfig) {

    /**
     * Validates a JWT token and returns the claims.
     *
     * @param token The JWT token to validate
     * @return Result containing the decoded claims or error
     */
    fun validateToken(token: String): Result<TokenClaims> = runCatching {
        val claims = Jwts.parser()
            .setSigningKey(config.secret)
            .parseClaimsJws(token)
            .body

        mapToTokenClaims(claims)
    }.recoverCatching { e ->
        when (e) {
            is ExpiredJwtException -> throw TokenExpiredException("Token has expired", e)
            else -> throw InvalidTokenException("Invalid token", e)
        }
    }

    /**
     * Generates a new JWT token.
     *
     * @param userId The user ID
     * @param role The user role
     * @return Result containing the generated token or error
     */
    fun generateToken(userId: String, role: Role): Result<String> = runCatching {
        val now = System.currentTimeMillis()
        val expiration = now + (config.expiresInSeconds * 1000)

        Jwts.builder()
            .setSubject(userId)
            .claim("role", role.name)
            .setIssuedAt(Date(now))
            .setExpiration(Date(expiration))
            .signWith(SignatureAlgorithm.HS256, config.secret)
            .compact()
    }

    private fun mapToTokenClaims(claims: Claims): TokenClaims {
        return TokenClaims(
            userId = claims.subject,
            role = Role.valueOf(claims["role"] as String),
            exp = claims.expiration.time / 1000,
            iat = claims.issuedAt.time / 1000
        )
    }

    private fun isTokenExpired(claims: Claims): Boolean {
        return claims.expiration.before(Date())
    }
}
