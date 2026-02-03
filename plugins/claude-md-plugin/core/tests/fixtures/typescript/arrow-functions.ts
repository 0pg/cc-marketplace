import { Claims } from './types';

/**
 * Validates a JWT token using arrow function syntax
 * @param token - The JWT token to validate
 */
export const validateToken = (token: string): Promise<Claims> => {
  // implementation
  return Promise.resolve({ userId: '', role: '' } as Claims);
};

/**
 * Generates a unique ID with prefix
 */
export const generateId = async (prefix: string): Promise<string> => {
  return `${prefix}-${Date.now()}`;
};

// Sync arrow function
export const syncHelper = (value: number): number => {
  return value * 2;
};

// Private function (should not be exported)
const privateHelper = () => {
  return 'internal';
};
