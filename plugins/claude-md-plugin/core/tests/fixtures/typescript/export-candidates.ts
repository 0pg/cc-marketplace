// TypeScript export candidates fixture
// Tests: enum exports, const variable exports, let variable exports

export enum Direction {
  Up = 'up',
  Down = 'down',
  Left = 'left',
  Right = 'right',
}

export enum Status {
  Active,
  Inactive,
  Pending,
}

export const MAX_RETRIES = 3;
export const DEFAULT_TIMEOUT: number = 30000;
export const API_BASE_URL = 'https://api.example.com';

export let currentUser: string;
export let connectionCount: number;

// This should be captured as a function, not a variable
export const processItem = (item: string): boolean => {
  return item.length > 0;
};

export function regularFunction(x: number): number {
  return x * 2;
}
