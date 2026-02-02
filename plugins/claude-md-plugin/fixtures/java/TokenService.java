package com.example.token;

import io.jsonwebtoken.Claims;
import io.jsonwebtoken.ExpiredJwtException;
import io.jsonwebtoken.Jwts;
import io.jsonwebtoken.SignatureAlgorithm;
import java.util.Date;

/**
 * Service for JWT token operations.
 */
public class TokenService {
    private final TokenConfig config;

    public TokenService(TokenConfig config) {
        this.config = config;
    }

    /**
     * Validates a JWT token and returns the claims.
     *
     * @param token The JWT token to validate
     * @return The decoded token claims
     * @throws TokenExpiredException if the token has expired
     * @throws InvalidTokenException if the token is invalid
     */
    public TokenClaims validateToken(String token) throws TokenExpiredException, InvalidTokenException {
        try {
            Claims claims = Jwts.parser()
                .setSigningKey(config.getSecret())
                .parseClaimsJws(token)
                .getBody();

            return mapToTokenClaims(claims);
        } catch (ExpiredJwtException e) {
            throw new TokenExpiredException("Token has expired", e);
        } catch (Exception e) {
            throw new InvalidTokenException("Invalid token", e);
        }
    }

    /**
     * Generates a new JWT token.
     *
     * @param userId The user ID
     * @param role The user role
     * @return The generated JWT token
     */
    public String generateToken(String userId, Role role) {
        long now = System.currentTimeMillis();
        long expiration = now + (config.getExpiresInSeconds() * 1000);

        return Jwts.builder()
            .setSubject(userId)
            .claim("role", role.name())
            .setIssuedAt(new Date(now))
            .setExpiration(new Date(expiration))
            .signWith(SignatureAlgorithm.HS256, config.getSecret())
            .compact();
    }

    private TokenClaims mapToTokenClaims(Claims claims) {
        TokenClaims tokenClaims = new TokenClaims();
        tokenClaims.setUserId(claims.getSubject());
        tokenClaims.setRole(Role.valueOf(claims.get("role", String.class)));
        tokenClaims.setExp(claims.getExpiration().getTime() / 1000);
        tokenClaims.setIat(claims.getIssuedAt().getTime() / 1000);
        return tokenClaims;
    }

    private boolean isTokenExpired(Claims claims) {
        return claims.getExpiration().before(new Date());
    }
}
