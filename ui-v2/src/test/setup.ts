import '@testing-library/jest-dom';

// Add custom matchers
declare global {
  namespace Vi {
    interface Assertion {
      // Define the custom matcher without depending on jest types
      toHaveBeenCalledExactlyOnceWith(...args: unknown[]): void;
    }
    interface AsymmetricMatchersContaining {
      // Define the custom matcher without depending on jest types
      toHaveBeenCalledExactlyOnceWith(...args: unknown[]): void;
    }
  }
}
