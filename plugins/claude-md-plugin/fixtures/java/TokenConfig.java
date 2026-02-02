package com.example.token;

/**
 * Token configuration settings.
 */
public class TokenConfig {
    private String secret;
    private long expiresInSeconds;
    private String issuer;

    public TokenConfig(String secret, long expiresInSeconds) {
        this.secret = secret;
        this.expiresInSeconds = expiresInSeconds;
    }

    public TokenConfig(String secret, long expiresInSeconds, String issuer) {
        this.secret = secret;
        this.expiresInSeconds = expiresInSeconds;
        this.issuer = issuer;
    }

    public String getSecret() {
        return secret;
    }

    public long getExpiresInSeconds() {
        return expiresInSeconds;
    }

    public String getIssuer() {
        return issuer;
    }
}
