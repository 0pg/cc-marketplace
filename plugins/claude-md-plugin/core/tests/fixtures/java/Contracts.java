package com.example.token;

import io.jsonwebtoken.Jwts;

/**
 * Contract examples for Java code analyzer.
 */
public class Contracts {

    /**
     * Validates a JWT token and returns the claims.
     *
     * @param token JWT token (must be non-empty)
     * @return TokenClaims object with valid userId
     * @throws InvalidTokenException if malformed or expired
     */
    public TokenClaims validateToken(String token) throws InvalidTokenException {
        if (token == null || token.isEmpty()) {
            throw new InvalidTokenException("Token is required");
        }

        // ... validation logic using Jwts
        return new TokenClaims("user123", "admin");
    }

    /**
     * Processes an order and returns a receipt.
     *
     * @param order Order object with id and items
     * @return Receipt with orderId and total
     * @throws ValidationException if order is invalid
     */
    public Receipt processOrder(Order order) throws ValidationException {
        if (order.getId() == null || order.getId().isEmpty()) {
            throw new ValidationException("Order ID required");
        }
        if (order.getItems() == null || order.getItems().isEmpty()) {
            throw new ValidationException("Items required");
        }

        // ... process order
        return new Receipt(order.getId(), 100);
    }

    /**
     * Private helper method.
     */
    private TokenClaims mapToTokenClaims(Object data) {
        return null;
    }

    /**
     * Private validation method.
     */
    private boolean isTokenExpired(String token) {
        return false;
    }
}

/**
 * Token claims data class.
 */
class TokenClaims {
    private final String userId;
    private final String role;

    public TokenClaims(String userId, String role) {
        this.userId = userId;
        this.role = role;
    }

    public String getUserId() {
        return userId;
    }

    public String getRole() {
        return role;
    }
}

/**
 * Order data class.
 */
class Order {
    private String id;
    private java.util.List<String> items;

    public String getId() {
        return id;
    }

    public java.util.List<String> getItems() {
        return items;
    }
}

/**
 * Receipt data class.
 */
class Receipt {
    private final String orderId;
    private final int total;

    public Receipt(String orderId, int total) {
        this.orderId = orderId;
        this.total = total;
    }
}

/**
 * Exception for invalid tokens.
 */
class InvalidTokenException extends Exception {
    public InvalidTokenException(String message) {
        super(message);
    }
}

/**
 * Exception for validation errors.
 */
class ValidationException extends Exception {
    public ValidationException(String message) {
        super(message);
    }
}
