/**
 * Clerk Webhook Event Factories
 *
 * Creates realistic Clerk webhook payloads for testing.
 * Includes proper Svix headers for signature verification.
 *
 * Clerk Webhook Events: https://clerk.com/docs/webhooks/overview
 */

import { createHmac } from 'crypto';

// Default test secret (base64 encoded)
const DEFAULT_SECRET = 'whsec_' + Buffer.from('test-webhook-secret').toString('base64');

/**
 * Generate Svix signature headers
 */
function generateSvixHeaders(payload, secret = DEFAULT_SECRET) {
  const svixId = `msg_${Date.now()}_${Math.random().toString(36).slice(2)}`;
  const svixTimestamp = String(Math.floor(Date.now() / 1000));

  const secretBytes = secret.startsWith('whsec_')
    ? Buffer.from(secret.slice(6), 'base64')
    : Buffer.from(secret, 'base64');

  const payloadString = typeof payload === 'string' ? payload : JSON.stringify(payload);
  const signedPayload = `${svixId}.${svixTimestamp}.${payloadString}`;
  const signature = createHmac('sha256', secretBytes)
    .update(signedPayload)
    .digest('base64');

  return {
    'svix-id': svixId,
    'svix-timestamp': svixTimestamp,
    'svix-signature': `v1,${signature}`,
    'content-type': 'application/json'
  };
}

/**
 * Create a mock Request object for webhook testing
 */
function createMockRequest(body, headers = {}, method = 'POST') {
  const bodyString = typeof body === 'string' ? body : JSON.stringify(body);

  return {
    method,
    headers: {
      get: (key) => headers[key.toLowerCase()] || null,
      has: (key) => key.toLowerCase() in headers
    },
    json: async () => typeof body === 'string' ? JSON.parse(body) : body,
    text: async () => bodyString,
    clone: () => createMockRequest(body, headers, method)
  };
}

// ============== User Events ==============

/**
 * Create user.created webhook event
 */
export function createUserCreatedEvent(options = {}) {
  const {
    userId = `user_${Date.now()}`,
    email = 'test@example.com',
    firstName = 'Test',
    lastName = 'User',
    createdAt = Date.now(),
    secret = DEFAULT_SECRET
  } = options;

  const payload = {
    type: 'user.created',
    data: {
      id: userId,
      email_addresses: [
        {
          id: `idn_${Date.now()}`,
          email_address: email,
          verification: { status: 'verified' }
        }
      ],
      first_name: firstName,
      last_name: lastName,
      created_at: createdAt,
      updated_at: createdAt
    },
    object: 'event'
  };

  const headers = generateSvixHeaders(payload, secret);

  return {
    payload,
    headers,
    request: createMockRequest(payload, headers)
  };
}

/**
 * Create user.deleted webhook event
 */
export function createUserDeletedEvent(options = {}) {
  const {
    userId = `user_${Date.now()}`,
    secret = DEFAULT_SECRET
  } = options;

  const payload = {
    type: 'user.deleted',
    data: {
      id: userId,
      deleted: true
    },
    object: 'event'
  };

  const headers = generateSvixHeaders(payload, secret);

  return {
    payload,
    headers,
    request: createMockRequest(payload, headers)
  };
}

// ============== Subscription Events ==============

/**
 * Create subscription.created webhook event
 */
export function createSubscriptionCreatedEvent(options = {}) {
  const {
    subscriptionId = `sub_${Date.now()}`,
    userId = `user_${Date.now()}`,
    planId = 'pro',
    status = 'active',
    billingPeriod = 'monthly',
    amount = 900, // $9.00 in cents
    createdAt = Date.now(),
    currentPeriodEnd = Date.now() + 30 * 24 * 60 * 60 * 1000,
    secret = DEFAULT_SECRET
  } = options;

  const payload = {
    type: 'subscription.created',
    data: {
      id: subscriptionId,
      user_id: userId,
      subscriber_id: userId,
      plan_id: planId,
      status,
      billing_period: billingPeriod,
      amount,
      created_at: createdAt,
      current_period_end: currentPeriodEnd
    },
    object: 'event'
  };

  const headers = generateSvixHeaders(payload, secret);

  return {
    payload,
    headers,
    request: createMockRequest(payload, headers)
  };
}

/**
 * Create subscription.updated webhook event
 */
export function createSubscriptionUpdatedEvent(options = {}) {
  const {
    subscriptionId = `sub_${Date.now()}`,
    userId = `user_${Date.now()}`,
    planId = 'pro',
    status = 'active',
    billingPeriod = 'monthly',
    amount = 900,
    currentPeriodEnd = Date.now() + 30 * 24 * 60 * 60 * 1000,
    secret = DEFAULT_SECRET
  } = options;

  const payload = {
    type: 'subscription.updated',
    data: {
      id: subscriptionId,
      user_id: userId,
      subscriber_id: userId,
      plan_id: planId,
      status,
      billing_period: billingPeriod,
      amount,
      current_period_end: currentPeriodEnd
    },
    object: 'event'
  };

  const headers = generateSvixHeaders(payload, secret);

  return {
    payload,
    headers,
    request: createMockRequest(payload, headers)
  };
}

/**
 * Create subscription.canceled webhook event
 */
export function createSubscriptionCanceledEvent(options = {}) {
  const {
    subscriptionId = `sub_${Date.now()}`,
    userId = `user_${Date.now()}`,
    canceledAt = Date.now(),
    secret = DEFAULT_SECRET
  } = options;

  const payload = {
    type: 'subscription.canceled',
    data: {
      id: subscriptionId,
      user_id: userId,
      subscriber_id: userId,
      status: 'canceled',
      canceled_at: canceledAt
    },
    object: 'event'
  };

  const headers = generateSvixHeaders(payload, secret);

  return {
    payload,
    headers,
    request: createMockRequest(payload, headers)
  };
}

// ============== Invoice Events ==============

/**
 * Create invoice.paid webhook event
 */
export function createInvoicePaidEvent(options = {}) {
  const {
    invoiceId = `inv_${Date.now()}`,
    userId = `user_${Date.now()}`,
    amount = 900,
    currency = 'usd',
    paidAt = Date.now(),
    secret = DEFAULT_SECRET
  } = options;

  const payload = {
    type: 'invoice.paid',
    data: {
      id: invoiceId,
      user_id: userId,
      subscriber_id: userId,
      amount_paid: amount,
      total: amount,
      currency,
      paid_at: paidAt,
      status: 'paid'
    },
    object: 'event'
  };

  const headers = generateSvixHeaders(payload, secret);

  return {
    payload,
    headers,
    request: createMockRequest(payload, headers)
  };
}

/**
 * Create invoice.payment_failed webhook event
 */
export function createInvoicePaymentFailedEvent(options = {}) {
  const {
    invoiceId = `inv_${Date.now()}`,
    userId = `user_${Date.now()}`,
    amount = 900,
    failedAt = Date.now(),
    secret = DEFAULT_SECRET
  } = options;

  const payload = {
    type: 'invoice.payment_failed',
    data: {
      id: invoiceId,
      user_id: userId,
      subscriber_id: userId,
      amount_due: amount,
      failed_at: failedAt,
      status: 'payment_failed'
    },
    object: 'event'
  };

  const headers = generateSvixHeaders(payload, secret);

  return {
    payload,
    headers,
    request: createMockRequest(payload, headers)
  };
}

// ============== Test Helpers ==============

/**
 * Create an event with an invalid signature (for testing rejection)
 */
export function createInvalidSignatureEvent(eventType = 'user.created', options = {}) {
  const payload = {
    type: eventType,
    data: options.data || { id: 'test' },
    object: 'event'
  };

  const headers = {
    'svix-id': 'msg_invalid',
    'svix-timestamp': String(Math.floor(Date.now() / 1000)),
    'svix-signature': 'v1,invalid_signature_here',
    'content-type': 'application/json'
  };

  return {
    payload,
    headers,
    request: createMockRequest(payload, headers)
  };
}

/**
 * Create an event with expired timestamp
 */
export function createExpiredTimestampEvent(eventType = 'user.created', options = {}) {
  const { secret = DEFAULT_SECRET } = options;

  const payload = {
    type: eventType,
    data: options.data || { id: 'test' },
    object: 'event'
  };

  // Timestamp from 10 minutes ago
  const oldTimestamp = String(Math.floor(Date.now() / 1000) - 600);
  const svixId = 'msg_expired';

  const secretBytes = secret.startsWith('whsec_')
    ? Buffer.from(secret.slice(6), 'base64')
    : Buffer.from(secret, 'base64');

  const payloadString = JSON.stringify(payload);
  const signedPayload = `${svixId}.${oldTimestamp}.${payloadString}`;
  const signature = createHmac('sha256', secretBytes)
    .update(signedPayload)
    .digest('base64');

  const headers = {
    'svix-id': svixId,
    'svix-timestamp': oldTimestamp,
    'svix-signature': `v1,${signature}`,
    'content-type': 'application/json'
  };

  return {
    payload,
    headers,
    request: createMockRequest(payload, headers)
  };
}

/**
 * Create an unknown event type (for testing graceful handling)
 */
export function createUnknownEvent(options = {}) {
  const { secret = DEFAULT_SECRET } = options;

  const payload = {
    type: 'unknown.event.type',
    data: { id: 'unknown' },
    object: 'event'
  };

  const headers = generateSvixHeaders(payload, secret);

  return {
    payload,
    headers,
    request: createMockRequest(payload, headers)
  };
}

// Export the default secret for tests that need to verify signatures
export { DEFAULT_SECRET, generateSvixHeaders, createMockRequest };
