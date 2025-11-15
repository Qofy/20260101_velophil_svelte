import { describe, it, expect } from 'vitest';
import { isValidUrl } from '../src/validator';


describe('isValidUrl()', () => {
  it('returns true for a valid http URL', () => {
    expect(isValidUrl('http://example.com')).toBe(true);
  });

  it('returns true for a valid https URL', () => {
    expect(isValidUrl('https://example.com')).toBe(true);
  });

  it('returns false for a non-URL string', () => {
    expect(isValidUrl('not-a-url')).toBe(false);
  });

  it('returns false for an empty string', () => {
    expect(isValidUrl('')).toBe(false);
  });
});
