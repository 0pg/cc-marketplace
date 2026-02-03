package token

import (
	"errors"

	"github.com/golang-jwt/jwt/v5"
)

// Errors for token validation
var (
	ErrInvalidToken = errors.New("invalid token")
	ErrExpiredToken = errors.New("token expired")
)

// Claims represents the JWT claims.
type Claims struct {
	UserID string
	Role   string
}

// ValidateToken validates a JWT token.
// Precondition: token must be non-empty
// Postcondition: returns Claims or error
// Errors: ErrInvalidToken if token format is invalid
func ValidateToken(token string) (*Claims, error) {
	if token == "" {
		return nil, ErrInvalidToken
	}

	// ... validation logic using jwt package
	_ = jwt.New(jwt.SigningMethodHS256)

	return &Claims{UserID: "user123", Role: "admin"}, nil
}

// ProcessOrder processes an order.
// Precondition: order.ID must not be empty
// Precondition: order.Items must not be empty
// Postcondition: returns Receipt or error
func ProcessOrder(order Order) (*Receipt, error) {
	if order.ID == "" {
		return nil, errors.New("order ID required")
	}
	if len(order.Items) == 0 {
		return nil, errors.New("items required")
	}
	// ... process order
	return &Receipt{OrderID: order.ID, Total: 100}, nil
}

// Order represents an order.
type Order struct {
	ID    string
	Items []string
}

// Receipt represents a receipt.
type Receipt struct {
	OrderID string
	Total   int
}

// internalHelper is a private function.
func internalHelper(data string) string {
	return data
}
