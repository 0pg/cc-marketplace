package auth

import "time"

const MaxRetries int = 3
const DefaultTimeout time.Duration = 30 * time.Second

// unexported const - should NOT be captured
const internalLimit int = 100

type UserID = string
type TokenResult = map[string]interface{}

// unexported type alias - should NOT be captured
type internalType = int

type Config struct {
	Timeout int
}

func ProcessItem(item string) bool {
	return len(item) > 0
}
