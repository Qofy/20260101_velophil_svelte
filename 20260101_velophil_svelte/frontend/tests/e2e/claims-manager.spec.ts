import { test, expect } from '@playwright/test'

async function mockBackground(page) {
  await page.route('**/api/health**', async r => r.fulfill({ status: 200, body: '{}' }))
  await page.route('**/api/runtime_counts**', async r => r.fulfill({ status: 200, body: JSON.stringify({ orders: 0, reservations: 0, payments: 0 }) }))
  await page.route('**/api/runtime_events**', async r => r.fulfill({ status: 200, body: JSON.stringify({ items: [] }) }))
}
async function resetStorage(page) { await page.addInitScript(() => { localStorage.clear(); sessionStorage.clear(); }) }

// 3) Admin edits own claims and UI updates without re-login
test('claims manager edits current user and gating updates', async ({ page }) => {
  await resetStorage(page)
  await mockBackground(page)

  const adminEmail = 'admin@example.com'

  // Login as admin
  await page.route('**/api/auth/login', async r => r.fulfill({ status: 200, body: JSON.stringify({ token: 'tokAdmin' }) }))
  let selfLimited = false
  await page.route('**/api/auth/claims**', async r => {
    const url = new URL(r.request().url())
    const email = (url.searchParams.get('email') || '').toLowerCase()
    if (email === adminEmail) {
      if (selfLimited) {
        return r.fulfill({ status: 200, body: JSON.stringify({ roles: [], scopes: ['settings:write','runtime:read'] }) })
      }
      // initial claims (admin full)
      return r.fulfill({ status: 200, body: JSON.stringify({ roles: ['admin'], scopes: ['orders:write','reservations:write','settings:write'] }) })
    }
    return r.fulfill({ status: 200, body: JSON.stringify({ roles: [], scopes: [] }) })
  })

  await page.route('**/api/auth/users**', async r => {
    await r.fulfill({ status: 200, body: JSON.stringify([{ email: adminEmail, name: 'Boss' }, { email: 'user2@example.com', name: 'User Two' }]) })
  })

  // Update claims PUT
  let putReceived = false
  await page.route('**/api/auth/claims', async r => {
    if (r.request().method() === 'PUT') {
      putReceived = true
      selfLimited = true
      return r.fulfill({ status: 200, body: '{}' })
    }
    return r.continue()
  })

  await page.goto('/')
  await page.getByTestId('login-username').fill(adminEmail)
  await page.getByTestId('login-password').fill('pw')
  await page.getByTestId('btn-login').click()

  // Open user admin
  await page.getByTestId('btn-user-admin').click()
  // Click self row
  await page.locator('[data-testid="user-row"][data-email="'+adminEmail+'"]').click()
  // Change scopes to drop orders:write (for visibility change), keep settings:write
  await page.getByTestId('claims-scopes').fill('runtime:read\nsettings:write')
  await page.getByTestId('btn-save-claims').click()
  await expect.poll(() => putReceived ? 'ok' : 'wait').toBe('ok')

  // After save, App refreshes current user claims; mock handler above returns limited scopes (no orders:write)
  // Expect ORDERS to hide, but SETTINGS remains visible
  await expect(page.getByTestId('btn-orders')).toBeHidden()
  await expect(page.getByTestId('btn-settings')).toBeVisible()
})
