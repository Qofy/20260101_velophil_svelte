import { describe, it, expect, beforeEach, vi } from 'vitest'

// Use compiled TS directly
import * as api from '../app/src/lib/api'

type FetchHandler = (url: string, init?: RequestInit) => Promise<{ ok: boolean; status: number; json: () => Promise<any> }>

function setFetch(handler: FetchHandler) {
  // @ts-ignore
  global.fetch = vi.fn(handler)
}

function okJson(data: any, status = 200) {
  return { ok: true, status, json: async () => data }
}
function notOk(status = 404, data: any = { error: 'not found' }) {
  return { ok: false, status, json: async () => data }
}

beforeEach(() => {
  vi.restoreAllMocks()
  // @ts-ignore
  global.fetch = undefined as any
  localStorage.clear()
})

describe('persistence: names', () => {
  it('lists names when server returns {items}', async () => {
    setFetch(async (url) => {
      if (url.endsWith('/api/custom_names')) return okJson({ items: [{ name: 'alpha' }, { name: 'beta' }] })
      return okJson([])
    })
    const r = await api.listNames()
    expect(r.via).toBe('server')
    expect(r.names).toEqual(['alpha', 'beta'])
  })

  it('save fallback to local and list from local', async () => {
    setFetch(async (url, init) => {
      if (url.endsWith('/api/custom_names') && init?.method === 'POST') throw new Error('offline')
      if (url.endsWith('/api/custom_names')) return notOk(500)
      if (url.includes('/api/custom_names/')) return notOk(404)
      return okJson([])
    })
    await api.saveNames('cfg1', ['A', 'B'])
    const listed = await api.listNames()
    expect(listed.via).toBe('local')
    expect(listed.names).toEqual(['cfg1'])
    const loaded = await api.loadNames('cfg1')
    expect(loaded.via).toBe('local')
    expect(loaded.data).toEqual(['A', 'B'])
  })
})

describe('persistence: zones/servers/clients', () => {
  it('loadServers falls back to listing when direct endpoint missing', async () => {
    setFetch(async (url, init) => {
      if (url.endsWith('/api/custom_servers/cfgX')) return notOk(404)
      if (url.endsWith('/api/custom_servers')) return okJson([{ name: 'cfgX', data: ['s1', 's2'] }])
      return okJson([])
    })
    const loaded = await api.loadServers('cfgX')
    expect(loaded.via).toBe('server')
    expect(loaded.data).toEqual(['s1', 's2'])
  })

  it('listClients reads names from array of strings', async () => {
    setFetch(async (url) => {
      if (url.endsWith('/api/custom_clients')) return okJson(['cA', 'cB'])
      return okJson([])
    })
    const res = await api.listClients()
    expect(res.via).toBe('server')
    expect(res.names).toEqual(['cA', 'cB'])
  })
})

describe('persistence: reservations config', () => {
  it('listReservationsConfigs handles object keyed by name', async () => {
    setFetch(async (url) => {
      if (url.endsWith('/api/custom_reservations')) return okJson({ cfgA: [{ name: 'x' }], cfgB: [{ name: 'y' }] })
      return okJson([])
    })
    const res = await api.listReservationsConfigs()
    expect(res.via).toBe('server')
    expect(res.names.sort()).toEqual(['cfgA', 'cfgB'])
  })

  it('loadReservationsConfig normalizes array of strings', async () => {
    setFetch(async (url) => {
      if (url.endsWith('/api/custom_reservations/cfgZ')) return okJson({ data: ['R1', 'R2'] })
      return okJson([])
    })
    const res = await api.loadReservationsConfig('cfgZ')
    expect(res.via).toBe('server')
    expect(res.data).toEqual([{ name: 'R1' }, { name: 'R2' }])
  })
})
