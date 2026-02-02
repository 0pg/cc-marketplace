import { Claims, TokenConfig } from './types';
import jwt from 'jsonwebtoken';
import { hashPassword } from '../utils/crypto';

/**
 * Validates a JWT token and returns the claims
 * @param token - The JWT token to validate
 * @returns The decoded claims
 * @throws TokenExpiredError if the token is expired
 */
export async function validateToken(token: string): Promise<Claims> {
  try {
    const claims = jwt.verify(token, process.env.JWT_SECRET!) as Claims;
    return claims;
  } catch (e) {
    if (e instanceof jwt.TokenExpiredError) {
      throw new TokenExpiredError('Token has expired');
    }
    throw new InvalidTokenError('Invalid token');
  }
}

export function generateToken(userId: string, role: string): string {
  return jwt.sign({ userId, role }, process.env.JWT_SECRET!, {
    expiresIn: '1h'
  });
}

export class TokenExpiredError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'TokenExpiredError';
  }
}

export class InvalidTokenError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'InvalidTokenError';
  }
}

export default validateToken;
