import 'vitest';

declare module 'vitest' {
  interface Assertion<T = unknown> {
    toBeOkResponse(): T;
  }
  interface AsymmetricMatchersContaining {
    toBeOkResponse(): unknown;
  }
}
