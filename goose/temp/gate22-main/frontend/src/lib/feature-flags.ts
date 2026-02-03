/**
 * Feature flag utilities for controlling application features
 */

/**
 * Check if subscription features are enabled
 * @returns {boolean} True if subscription features should be shown
 */
export function isSubscriptionEnabled(): boolean {
  const enabled = process.env.NEXT_PUBLIC_SUBSCRIPTION_ENABLED === "true";
  return enabled;
}
