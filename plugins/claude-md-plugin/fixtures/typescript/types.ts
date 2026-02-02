/**
 * JWT token claims structure
 */
export interface Claims {
  userId: string;
  role: Role;
  exp: number;
  iat: number;
}

export type Role = 'admin' | 'user' | 'guest';

export interface TokenConfig {
  secret: string;
  expiresIn: string;
  issuer?: string;
}

export type TokenPayload = Omit<Claims, 'exp' | 'iat'>;
