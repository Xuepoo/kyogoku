import { test, expect } from '@playwright/test';

test('Kyogoku GUI UI Verification', async ({ page }) => {
  // Navigate to the app
  await page.goto('/');

  // Check title
  await expect(page).toHaveTitle(/Kyogoku \(京极\)/);

  // Check Configuration Form
  const configHeader = page.locator('h2:has-text("Configuration")');
  await expect(configHeader).toBeVisible();

  // Check API Provider dropdown
  const providerSelect = page.locator('#api-provider');
  await expect(providerSelect).toBeVisible();
  await expect(providerSelect).toHaveValue('openai'); // Default value (or stored value if cache works)
  
  // Check Save Changes button
  const saveButton = page.locator('button[type="submit"]');
  await expect(saveButton).toBeVisible();
  
  // Check File Upload area
  const dropZone = page.locator('#drop-zone');
  await expect(dropZone).toBeVisible();
  
  // Check Supported Formats list
  const formatList = page.locator('h3:has-text("Supported Formats")');
  await expect(formatList).toBeVisible();
});
