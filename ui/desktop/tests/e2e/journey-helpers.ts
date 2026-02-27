import type { Page, TestInfo } from '@playwright/test';

export async function dismissOptionalChooseModelModal(page: Page) {
  // We’ve seen this modal appear on first-run / onboarding.
  const close = page.getByRole('button', { name: /close/i }).first();
  const cancel = page.getByRole('button', { name: /cancel/i }).first();

  // If the modal exists, it usually blocks interaction; we try common exit paths.
  const modalTitle = page.getByText(/choose a model/i).first();
  if ((await modalTitle.count()) === 0) return;

  if ((await cancel.count()) > 0) {
    await cancel.click().catch(() => undefined);
    return;
  }

  if ((await close.count()) > 0) {
    await close.click().catch(() => undefined);
    return;
  }

  await page.keyboard.press('Escape').catch(() => undefined);
}

export async function dismissOptionalTelemetryModal(page: Page) {
  // TelemetryOptOutModal offers “No thanks”
  const noThanks = page.getByRole('button', { name: /no thanks/i }).first();
  if ((await noThanks.count()) === 0) return;
  await noThanks.click().catch(() => undefined);
}

export async function dismissOptionalAnnouncementModal(page: Page) {
  // AnnouncementModal is usually dismissible via Escape.
  const announcementHeading = page.getByRole('heading', { name: /announcement/i }).first();
  if ((await announcementHeading.count()) === 0) return;
  await page.keyboard.press('Escape').catch(() => undefined);
}

export async function bootstrapFirstRunUI(page: Page) {
  await dismissOptionalTelemetryModal(page);
  await dismissOptionalChooseModelModal(page);
  await dismissOptionalAnnouncementModal(page);
}

export async function clickIfPresent(page: Page, selector: string) {
  const el = page.locator(selector);
  if ((await el.count()) === 0) return false;
  await el.first().click();
  return true;
}

export async function clickSidebarItem(page: Page, itemLabelLowercase: string) {
  const testId = `sidebar-${itemLabelLowercase}-button`;
  const button = page.locator(`[data-testid="${testId}"]`);
  if ((await button.count()) === 0) return false;
  await button.first().click();
  return true;
}

export function hashRouteUrl(page: Page, route: string) {
  const base = page.url().split('#')[0];
  const normalized = route.startsWith('/') ? route : `/${route}`;
  return `${base}#${normalized}`;
}

export async function attachOnFailure(testInfo: TestInfo, name: string, body: string) {
  if (testInfo.status === testInfo.expectedStatus) return;
  await testInfo.attach(name, { body, contentType: 'text/plain' });
}
