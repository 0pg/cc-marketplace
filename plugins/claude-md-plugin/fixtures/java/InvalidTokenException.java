package com.example.token;

/**
 * Exception thrown when token is invalid.
 */
public class InvalidTokenException extends Exception {
    public InvalidTokenException(String message) {
        super(message);
    }

    public InvalidTokenException(String message, Throwable cause) {
        super(message, cause);
    }
}
