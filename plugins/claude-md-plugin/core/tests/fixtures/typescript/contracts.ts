import { Claims, Order, Receipt } from './types';

/**
 * Validates a JWT token and returns the claims.
 * @precondition token must be non-empty
 * @postcondition returns Claims with valid userId
 * @throws InvalidTokenError if token is malformed or expired
 */
export function validateToken(token: string): Claims {
  if (!token) {
    throw new InvalidTokenError('Token is required');
  }
  // ... validation logic
  return { userId: 'user123', role: 'admin' } as Claims;
}

/**
 * Processes an order and returns a receipt.
 * This function has validation logic that we can infer contracts from.
 */
export function processOrder(order: Order): Receipt {
  if (!order.id) {
    throw new ValidationError('Order ID required');
  }
  if (order.items.length === 0) {
    throw new ValidationError('Items required');
  }
  // ... process order
  return { orderId: order.id, total: 100 } as Receipt;
}

/**
 * Helper function with invariant.
 * @invariant balance must never be negative
 */
export function withdraw(amount: number, balance: number): number {
  if (amount > balance) {
    throw new InsufficientFundsError('Not enough balance');
  }
  return balance - amount;
}

export class InvalidTokenError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'InvalidTokenError';
  }
}

export class ValidationError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'ValidationError';
  }
}

export class InsufficientFundsError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'InsufficientFundsError';
  }
}
