import { test, expect } from '@playwright/test';

test.describe('Authentication Flow', () => {
  test.beforeEach(async ({ page }) => {
    // Navigate to the home page before each test
    await page.goto('/');
  });

  test('should display login form when Login button is clicked', async ({ page }) => {
    // Wait for page to load
    await page.waitForLoadState('networkidle');

    // Click the Login button
    await page.click('button:has-text("Login")');

    // Verify login overlay is displayed
    await expect(page.locator('text=Login to Continue')).toBeVisible();
    await expect(page.locator('[data-testid="login-username"]')).toBeVisible();
    await expect(page.locator('[data-testid="login-password"]')).toBeVisible();
    await expect(page.locator('[data-testid="btn-login"]')).toBeVisible();
  });

  test('should successfully login with valid credentials', async ({ page }) => {
    // Wait for page to load
    await page.waitForLoadState('networkidle');

    // Click the Login button
    await page.click('button:has-text("Login")');

    // Wait for login form
    await expect(page.locator('[data-testid="login-username"]')).toBeVisible();

    // Fill in credentials
    await page.fill('[data-testid="login-username"]', 'admin@example.com');
    await page.fill('[data-testid="login-password"]', 'AdminPass123');

    // Click login button
    await page.click('[data-testid="btn-login"]');

    // Wait for login to complete and overlay to disappear
    await expect(page.locator('text=Login to Continue')).not.toBeVisible({ timeout: 10000 });

    // Verify user is logged in - check for user email or admin badge
    await expect(page.locator('text=admin@example.com')).toBeVisible({ timeout: 10000 });
    await expect(page.locator('text=Admin').or(page.locator('.badge:has-text("Admin")'))).toBeVisible();

    // Verify logout button appears
    await expect(page.locator('button:has-text("Logout")')).toBeVisible();

    // Verify WASM demo section is visible (only shown when logged in)
    await expect(page.locator('text=WASM Layout Demo')).toBeVisible();
  });

  test('should fail login with invalid credentials', async ({ page }) => {
    // Wait for page to load
    await page.waitForLoadState('networkidle');

    // Click the Login button
    await page.click('button:has-text("Login")');

    // Fill in invalid credentials
    await page.fill('[data-testid="login-username"]', 'wrong@example.com');
    await page.fill('[data-testid="login-password"]', 'WrongPassword');

    // Click login button
    await page.click('[data-testid="btn-login"]');

    // Wait a bit for the error message
    await page.waitForTimeout(2000);

    // Verify error message appears
    await expect(page.locator('text=Login failed').or(page.locator('text=Invalid email or password'))).toBeVisible();

    // Verify user is NOT logged in - Login button should still be visible
    await expect(page.locator('button:has-text("Login")')).toBeVisible();
  });

  test('should successfully logout after login', async ({ page }) => {
    // Wait for page to load
    await page.waitForLoadState('networkidle');

    // Login first
    await page.click('button:has-text("Login")');
    await expect(page.locator('[data-testid="login-username"]')).toBeVisible();
    await page.fill('[data-testid="login-username"]', 'admin@example.com');
    await page.fill('[data-testid="login-password"]', 'AdminPass123');
    await page.click('[data-testid="btn-login"]');

    // Wait for login to complete
    await expect(page.locator('text=admin@example.com')).toBeVisible({ timeout: 10000 });
    await expect(page.locator('button:has-text("Logout")')).toBeVisible();

    // Click logout button
    await page.click('button:has-text("Logout")');

    // Wait for logout to complete
    await page.waitForTimeout(1000);

    // Verify user is logged out - Login button should appear again
    await expect(page.locator('button:has-text("Login")')).toBeVisible({ timeout: 10000 });
    await expect(page.locator('text=admin@example.com')).not.toBeVisible();

    // Verify welcome screen is shown (appears when not logged in)
    await expect(page.locator('text=Welcome to VeloAssure')).toBeVisible();
  });

  test('should switch between Login and Register forms', async ({ page }) => {
    // Wait for page to load
    await page.waitForLoadState('networkidle');

    // Click Login button
    await page.click('button:has-text("Login")');
    await expect(page.locator('text=Login to Continue')).toBeVisible();

    // Click Register button in login form
    await page.click('[data-testid="btn-show-register"]');

    // Verify register form is shown
    await expect(page.locator('text=Register / Request Access')).toBeVisible();
    await expect(page.locator('[data-testid="register-email"]')).toBeVisible();
    await expect(page.locator('[data-testid="register-password"]')).toBeVisible();
  });

  test('should persist login across page refreshes (cookie test)', async ({ page }) => {
    // Wait for page to load
    await page.waitForLoadState('networkidle');

    // Login
    await page.click('button:has-text("Login")');
    await page.fill('[data-testid="login-username"]', 'admin@example.com');
    await page.fill('[data-testid="login-password"]', 'AdminPass123');
    await page.click('[data-testid="btn-login"]');

    // Wait for login to complete
    await expect(page.locator('text=admin@example.com')).toBeVisible({ timeout: 10000 });

    // Refresh the page
    await page.reload();
    await page.waitForLoadState('networkidle');

    // Verify user is still logged in after refresh
    await expect(page.locator('text=admin@example.com')).toBeVisible({ timeout: 10000 });
    await expect(page.locator('button:has-text("Logout")')).toBeVisible();
  });
});
