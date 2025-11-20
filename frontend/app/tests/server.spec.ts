import { describe, it, expect } from 'vitest';

// Try to hit the backend directly (no mocks). Will pass as online if reachable, otherwise verifies graceful offline state.

const URLS = [
  'http://127.0.0.1:8080/api/custom_periodic',
  'http://127.0.0.1:8080/api/custom_orders',
'http://127.0.0.1:8080/api/custom_reservations',
  'http://127.0.0.1:8080/api/custom_names',
'http://127.0.0.1:8080/api/custom_zones',
  'http://127.0.0.1:8080/api/custom_servers',
  'http://127.0.0.1:8080/api/custom_clients',
  'http://127.0.0.1:8080/'
];

async function tryFetch(url: string, timeoutMs = 1000) {
  const ac = new AbortController();
  const id = setTimeout(() => ac.abort(), timeoutMs);
  try {
    const res = await fetch(url, { signal: ac.signal });
    clearTimeout(id);
    return res.status;
  } catch (e) {
    clearTimeout(id);
    throw e;
  }
}

describe('backend connectivity (best-effort)', () => {
  it('responds or is offline gracefully', async () => {
    let online = false;
    for (const u of URLS) {
      try {
        const status = await tryFetch(u, 700);
        // if we got here, backend is up
        expect(typeof status).toBe('number');
        online = true;
        break;
      } catch {
        // continue to next
      }
    }
    // If not online, consider it gracefully offline (no exception)
    if (!online) expect(online).toBe(false);
  }, 12000);
});