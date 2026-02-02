package com.example.token;

/**
 * JWT token claims structure.
 */
public class TokenClaims {
    private String userId;
    private Role role;
    private long exp;
    private long iat;

    public TokenClaims() {}

    public TokenClaims(String userId, Role role, long exp, long iat) {
        this.userId = userId;
        this.role = role;
        this.exp = exp;
        this.iat = iat;
    }

    public String getUserId() {
        return userId;
    }

    public void setUserId(String userId) {
        this.userId = userId;
    }

    public Role getRole() {
        return role;
    }

    public void setRole(Role role) {
        this.role = role;
    }

    public long getExp() {
        return exp;
    }

    public void setExp(long exp) {
        this.exp = exp;
    }

    public long getIat() {
        return iat;
    }

    public void setIat(long iat) {
        this.iat = iat;
    }
}
