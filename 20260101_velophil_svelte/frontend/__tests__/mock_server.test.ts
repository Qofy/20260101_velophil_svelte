import { describe, it, expect } from 'vitest';
import { mockCheckUrlExists } from '../src/mock_server';

describe('mockCheckUrlExists', () => {
  it('returns a predictable result for a given URL', async () => {
    const url = 'https://example.com';
    const result1 = await mockCheckUrlExists(url);
    const result2 = await mockCheckUrlExists(url);
    expect(result1).toEqual(result2); // deterministic
    expect(['file', 'folder']).toContain(result1.type);
    expect(typeof result1.exists).toBe('boolean');
  });

  it('handles different URLs deterministically', async () => {
    const resultA = await mockCheckUrlExists('https://a.com');
    const resultB = await mockCheckUrlExists('https://b.com');
    expect(resultA).not.toEqual(resultB); // likely different
  });
});
