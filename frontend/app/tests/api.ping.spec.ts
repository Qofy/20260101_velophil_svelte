import { describe, it, expect, vi, afterEach } from 'vitest';
import { pingServer } from '../src/lib/api';

const HEALTH_URL = 'http://127.0.0.1:8080/health';

afterEach(() => {
  vi.restoreAllMocks();
});

describe('api.pingServer', () => {
  it('returns online=true on 200 response', async () => {
    const fakeRes = new Response('', { status: 200 });
    const fetchMock = vi.spyOn(globalThis, 'fetch' as any).mockResolvedValue(fakeRes as any);
    const res = await pingServer(200);
    expect(res.online).toBe(true);
    expect(typeof res.status).toBe('number');
    expect(res.status).toBe(200);
    expect(fetchMock).toHaveBeenCalled();
  });

  it('returns online=false on fetch error/timeout', async () => {
    vi.spyOn(globalThis, 'fetch' as any).mockRejectedValue(new Error('network'));
    const res = await pingServer(50);
    expect(res.online).toBe(false);
    expect(typeof res.error).toBe('string');
  });
});
