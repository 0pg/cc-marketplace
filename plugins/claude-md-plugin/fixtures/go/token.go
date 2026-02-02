// Package token provides JWT token handling utilities.
package token

import (
	"errors"
	"time"

	"github.com/golang-jwt/jwt/v5"
)

// Claims represents the JWT claims structure.
type Claims struct {
	UserID string `json:"user_id"`
	Role   string `json:"role"`
	jwt.RegisteredClaims
}

// Config holds token configuration.
type Config struct {
	Secret    string
	ExpiresIn time.Duration
	Issuer    string
}

// ErrExpiredToken is returned when token is expired.
var ErrExpiredToken = errors.New("token has expired")

// ErrInvalidToken is returned when token is invalid.
var ErrInvalidToken = errors.New("invalid token")

// ValidateToken validates a JWT token and returns claims.
func ValidateToken(tokenString string, secret string) (*Claims, error) {
	token, err := jwt.ParseWithClaims(tokenString, &Claims{}, func(t *jwt.Token) (interface{}, error) {
		return []byte(secret), nil
	})

	if err != nil {
		if errors.Is(err, jwt.ErrTokenExpired) {
			return nil, ErrExpiredToken
		}
		return nil, ErrInvalidToken
	}

	claims, ok := token.Claims.(*Claims)
	if !ok || !token.Valid {
		return nil, ErrInvalidToken
	}

	return claims, nil
}

// GenerateToken creates a new JWT token.
func GenerateToken(userID, role string, config Config) (string, error) {
	claims := Claims{
		UserID: userID,
		Role:   role,
		RegisteredClaims: jwt.RegisteredClaims{
			ExpiresAt: jwt.NewNumericDate(time.Now().Add(config.ExpiresIn)),
			IssuedAt:  jwt.NewNumericDate(time.Now()),
			Issuer:    config.Issuer,
		},
	}

	token := jwt.NewWithClaims(jwt.SigningMethodHS256, claims)
	return token.SignedString([]byte(config.Secret))
}

// internalHelper is a private helper function.
func internalHelper(data map[string]interface{}) bool {
	return len(data) > 0
}
