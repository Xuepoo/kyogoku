import { test, expect } from '@playwright/test';

test('Kyogoku GUI UI Verification', async ({ page }) => {
  // Navigate to the app
  await page.goto('/');

  // Check title
  await expect(page).toHaveTitle(/Kyogoku \(京极\)/);

  // Check Settings Header (was Configuration)
  // Use data-i18n attribute to be robust, or just text
  const settingsHeader = page.locator('span[data-i18n="settings"]');
  await expect(settingsHeader).toBeVisible();
  await expect(settingsHeader).toHaveText('Settings');

  // Check API Provider dropdown
  const providerSelect = page.locator('#api-provider');
  await expect(providerSelect).toBeVisible();
  await expect(providerSelect).toHaveValue('openai'); // Default value

  // Check Language Selector
  const langSelect = page.locator('#language-select');
  await expect(langSelect).toBeVisible();
  await expect(langSelect).toHaveValue('en-US');

  // Check Save Button
  const saveButton = page.locator('#save-btn');
  await expect(saveButton).toBeVisible();
  await expect(saveButton).toContainText('Save');
  
  // Check Reload Button
  const reloadButton = page.locator('#reload-btn');
  await expect(reloadButton).toBeVisible();

  // Check File Upload area (Drop Zone)
  const dropZone = page.locator('#drop-zone');
  await expect(dropZone).toBeVisible();
  await expect(dropZone).toContainText('Click to upload files');
  
  // Check Supported Formats indicators
  const epubBadge = dropZone.locator('span:has-text(".epub")');
  await expect(epubBadge).toBeVisible();

  // Check Output Directory field
  const outputDir = page.locator('#output-directory');
  await expect(outputDir).toBeVisible();
  
  // Check Cost Estimation Panel (should be hidden initially)
  const costPanel = page.locator('#cost-panel');
  await expect(costPanel).toBeHidden();
});

test('Save Configuration Error Handling', async ({ page }) => {
  await page.goto('/');
  
  // Click Save button
  const saveButton = page.locator('#save-btn');
  await saveButton.click();
  
  // Expect a toast notification with error
  // The toast container is #toast-container
  const toastContainer = page.locator('#toast-container');
  await expect(toastContainer).toBeVisible();
  
  // Check for error toast
  // The error message should contain "Failed to save" or similar because backend is missing
  // or "is not a function" if window.__TAURI_IPC__ is missing
  const toast = toastContainer.locator('div');
  await expect(toast).toBeVisible();
  // We don't strict match the text because the error message depends on browser/environment
  // but it should be visible.
});

test('Language Selection', async ({ page }) => {
  await page.goto('/');
  const languageSelect = page.locator('#language-select');
  await expect(languageSelect).toBeVisible();
  
  // Check if options exist
  await expect(languageSelect.locator('option[value="en-US"]')).toBeAttached();
  await expect(languageSelect.locator('option[value="zh-CN"]')).toBeAttached();
  await expect(languageSelect.locator('option[value="ja-JP"]')).toBeAttached();
});

test('Theme Toggle', async ({ page }) => {
  await page.goto('/');
  
  // Locate theme toggle button (moon/sun icon)
  // It has id="theme-toggle"
  const themeToggle = page.locator('#theme-toggle');
  await expect(themeToggle).toBeVisible();
  
  // Check initial state (assuming system default or light)
  // Let's just toggle it and see if class changes
  const html = page.locator('html');
  
  // Click toggle
  await themeToggle.click();
  
  // Check if class changed. 
  // If it was light, it should become dark or vice-versa.
  // We can check for 'dark' class presence toggle
  const isDark = await html.getAttribute('class');
  
  await themeToggle.click();
  const isDarkAfter = await html.getAttribute('class');
  
  expect(isDark).not.toBe(isDarkAfter);
});
