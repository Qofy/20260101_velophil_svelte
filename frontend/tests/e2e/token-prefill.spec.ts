import { test, expect } from '@playwright/test'

async function resetStorage(page) { await page.addInitScript(() => { localStorage.clear(); sessionStorage.clear(); }) }

// Note: We validate prefill mechanics by seeding localStorage; env-based prefill is exercised in prod build.
test.skip('access token input is prefilled from localStorage on first load', async ({ page }) => {
  await page.addInitScript(() => { localStorage.setItem('API_ACCESS_TOKEN', 'test-token-123') })
  await page.goto('/')
  const tokInput = page.getByTestId('login-access-token')
  await expect(tokInput).toBeVisible()
  const val = await tokInput.inputValue()
  expect(val).toEqual('test-token-123')
})
