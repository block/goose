import { Locator, Page } from '@playwright/test';
import { isVideoRecording } from './video';

const VISUAL_DELAY_MS = Number(process.env.E2E_VISUAL_DELAY_MS ?? (isVideoRecording() ? '500' : '0'));

const PAGE_ACTION_METHODS = new Set([
  'click',
  'dblclick',
  'tap',
  'fill',
  'press',
  'check',
  'uncheck',
  'setChecked',
  'selectOption',
  'dragAndDrop',
  'goto',
  'reload'
]);

const LOCATOR_QUERY_METHODS = new Set([
  'locator',
  'getByAltText',
  'getByLabel',
  'getByPlaceholder',
  'getByRole',
  'getByTestId',
  'getByText',
  'getByTitle'
]);

const LOCATOR_ACTION_METHODS = new Set([
  'click',
  'dblclick',
  'tap',
  'fill',
  'press',
  'check',
  'uncheck',
  'setChecked',
  'selectOption',
  'hover',
  'focus',
  'blur'
]);

const VISUAL_DELAY_DECORATED = Symbol('visual-delay-decorated');

async function applyVisualDelay(page: Page): Promise<void> {
  if (VISUAL_DELAY_MS > 0) {
    await page.waitForTimeout(VISUAL_DELAY_MS);
  }
}

function withVisualDelayLocator(locator: Locator, page: Page): Locator {
  const existingMark = (locator as unknown as Record<PropertyKey, unknown>)[VISUAL_DELAY_DECORATED];
  if (existingMark) {
    return locator;
  }

  for (const methodName of LOCATOR_ACTION_METHODS) {
    const original = (locator as unknown as Record<string, unknown>)[methodName];
    if (typeof original === 'function') {
      (locator as unknown as Record<string, unknown>)[methodName] = async (...args: unknown[]) => {
        const resolved = await (original as (...params: unknown[]) => unknown).apply(locator, args);
        await applyVisualDelay(page);
        return resolved;
      };
    }
  }

  for (const methodName of LOCATOR_QUERY_METHODS) {
    const original = (locator as unknown as Record<string, unknown>)[methodName];
    if (typeof original === 'function') {
      (locator as unknown as Record<string, unknown>)[methodName] = (...args: unknown[]) => {
        const next = (original as (...params: unknown[]) => unknown).apply(locator, args);
        if (next && typeof next === 'object') {
          return withVisualDelayLocator(next as Locator, page);
        }
        return next;
      };
    }
  }

  Object.defineProperty(locator, VISUAL_DELAY_DECORATED, {
    value: true,
    enumerable: false,
    configurable: false,
    writable: false
  });

  return locator;
}

export function withVisualDelayPage(page: Page): Page {
  if (VISUAL_DELAY_MS <= 0) {
    return page;
  }

  return new Proxy(page, {
    get(target, prop, receiver) {
      const propName = String(prop);
      const value = Reflect.get(target, prop, receiver);

      if (typeof value !== 'function') {
        return value;
      }

      return (...args: unknown[]) => {
        const result = value.apply(target, args);
        if (LOCATOR_QUERY_METHODS.has(propName) && result && typeof result === 'object') {
          return withVisualDelayLocator(result as Locator, target);
        }
        if (PAGE_ACTION_METHODS.has(propName)) {
          return Promise.resolve(result).then(async (resolved) => {
            await applyVisualDelay(target);
            return resolved;
          });
        }
        return result;
      };
    }
  }) as Page;
}
