import { test, expect } from '@playwright/test'

async function mockBackground(page) {
  await page.route('**/api/health**', async r => r.fulfill({ status: 200, body: '{}' }))
  await page.route('**/api/runtime_counts**', async r => r.fulfill({ status: 200, body: JSON.stringify({ orders: 0, reservations: 0, payments: 0 }) }))
  await page.route('**/api/runtime_events**', async r => r.fulfill({ status: 200, body: JSON.stringify({ items: [] }) }))
}
async function resetStorage(page) { await page.addInitScript(() => { localStorage.clear(); sessionStorage.clear(); }) }

// 2a) Non-admin: no scopes -> gated buttons hidden
test('claims gating hides buttons for non-admin without scopes', async ({ page }) => {
  await resetStorage(page)
  await mockBackground(page)

  await page.route('**/api/auth/login', async r => r.fulfill({ status: 200, body: JSON.stringify({ token: 'tokU' }) }))
  await page.route('**/api/auth/claims**', async r => r.fulfill({ status: 200, body: JSON.stringify({ roles: [], scopes: [] }) }))

  await page.goto('/')
  await page.getByTestId('login-username').fill('user@example.com')
  await page.getByTestId('login-password').fill('pw')
  await page.getByTestId('btn-login').click()

  // Settings button is now always visible by design
  await expect(page.getByTestId('btn-settings')).toBeVisible()
  await expect(page.getByTestId('btn-user-admin')).toHaveCount(0)
  await expect(page.getByTestId('btn-orders')).toBeHidden()
  await expect(page.getByTestId('btn-reserv')).toBeHidden()
  await expect(page.getByTestId('btn-kitchen')).toBeHidden()
})

// 2b) Admin: all features visible
test('claims gating shows buttons for admin', async ({ page }) => {
  await resetStorage(page)
  await mockBackground(page)

  await page.route('**/api/auth/login', async r => r.fulfill({ status: 200, body: JSON.stringify({ token: 'tokA' }) }))
  await page.route('**/api/auth/claims**', async r => r.fulfill({ status: 200, body: JSON.stringify({ roles: ['admin'], scopes: [] }) }))

  await page.goto('/')
  await page.getByTestId('login-username').fill('boss@example.com')
  await page.getByTestId('login-password').fill('pw')
  await page.getByTestId('btn-login').click()

  await expect(page.getByTestId('btn-settings')).toBeVisible()
  await expect(page.getByTestId('btn-user-admin')).toBeVisible()
  await expect(page.getByTestId('btn-orders')).toBeVisible()
  await expect(page.getByTestId('btn-reserv')).toBeVisible()
  await expect(page.getByTestId('btn-kitchen')).toBeVisible()
})
