/**
 * Unit tests for auth flow state machines
 *
 * Tests the state transitions in PasskeySignupFlow, PasskeySigninFlow,
 * ClaimPrompt, and PasskeyGate components.
 */

import { describe, it, expect } from 'vitest';
import {
  SIGNUP_STATES,
  SIGNIN_STATES,
  GATE_STATES,
  getNextSignupState,
  getNextSigninStateAfterPasskey,
  getNextSigninStateAfterEmail,
  getGateState,
  getClaimPromptState,
  selectEmailVerificationStrategy
} from '../../lib/auth-flows.js';

describe('PasskeySignupFlow state transitions', () => {
  describe('email → verify', () => {
    it('transitions from email to verify after email submission', () => {
      const next = getNextSignupState(SIGNUP_STATES.EMAIL, {});
      expect(next).toBe(SIGNUP_STATES.VERIFY);
    });
  });

  describe('verify → passkey', () => {
    it('transitions to passkey when status is complete', () => {
      const next = getNextSignupState(SIGNUP_STATES.VERIFY, { status: 'complete' });
      expect(next).toBe(SIGNUP_STATES.PASSKEY);
    });

    it('transitions to passkey when status is missing_requirements', () => {
      // Clerk returns this when passkeys are Optional
      const next = getNextSignupState(SIGNUP_STATES.VERIFY, { status: 'missing_requirements' });
      expect(next).toBe(SIGNUP_STATES.PASSKEY);
    });

    it('stays in verify for other statuses', () => {
      const next = getNextSignupState(SIGNUP_STATES.VERIFY, { status: 'needs_first_factor' });
      expect(next).toBe(SIGNUP_STATES.VERIFY);
    });
  });

  describe('passkey → claiming', () => {
    it('transitions from passkey to claiming after passkey creation', () => {
      const next = getNextSignupState(SIGNUP_STATES.PASSKEY, {});
      expect(next).toBe(SIGNUP_STATES.CLAIMING);
    });
  });

  describe('claiming → done', () => {
    it('transitions from claiming to done after subdomain claimed', () => {
      const next = getNextSignupState(SIGNUP_STATES.CLAIMING, {});
      expect(next).toBe(SIGNUP_STATES.DONE);
    });
  });

  describe('full flow', () => {
    it('follows complete signup flow', () => {
      let state = SIGNUP_STATES.EMAIL;

      // Submit email
      state = getNextSignupState(state, {});
      expect(state).toBe(SIGNUP_STATES.VERIFY);

      // Verify code - complete
      state = getNextSignupState(state, { status: 'complete' });
      expect(state).toBe(SIGNUP_STATES.PASSKEY);

      // Create passkey
      state = getNextSignupState(state, {});
      expect(state).toBe(SIGNUP_STATES.CLAIMING);

      // Claim subdomain
      state = getNextSignupState(state, {});
      expect(state).toBe(SIGNUP_STATES.DONE);
    });
  });
});

describe('PasskeySigninFlow state transitions', () => {
  describe('passkey attempt', () => {
    it('transitions to complete on successful passkey auth', () => {
      const next = getNextSigninStateAfterPasskey(SIGNIN_STATES.PASSKEY, { success: true });
      expect(next).toBe('complete');
    });

    it('transitions to email on failed passkey', () => {
      const next = getNextSigninStateAfterPasskey(SIGNIN_STATES.PASSKEY, { success: false });
      expect(next).toBe(SIGNIN_STATES.EMAIL);
    });

    it('transitions to email on cancelled passkey', () => {
      const next = getNextSigninStateAfterPasskey(SIGNIN_STATES.PASSKEY, {
        success: false,
        cancelled: true
      });
      expect(next).toBe(SIGNIN_STATES.EMAIL);
    });
  });

  describe('email submission', () => {
    it('transitions to verify_link when email_link available', () => {
      const next = getNextSigninStateAfterEmail(SIGNIN_STATES.EMAIL, {
        hasEmailLink: true,
        hasEmailCode: true
      });
      expect(next).toBe(SIGNIN_STATES.VERIFY_LINK);
    });

    it('transitions to verify_code when only email_code available', () => {
      const next = getNextSigninStateAfterEmail(SIGNIN_STATES.EMAIL, {
        hasEmailLink: false,
        hasEmailCode: true
      });
      expect(next).toBe(SIGNIN_STATES.VERIFY_CODE);
    });

    it('stays in email when no verification method available', () => {
      const next = getNextSigninStateAfterEmail(SIGNIN_STATES.EMAIL, {
        hasEmailLink: false,
        hasEmailCode: false
      });
      expect(next).toBe(SIGNIN_STATES.EMAIL);
    });
  });

  describe('full flows', () => {
    it('follows passkey success flow', () => {
      let state = SIGNIN_STATES.PASSKEY;
      state = getNextSigninStateAfterPasskey(state, { success: true });
      expect(state).toBe('complete');
    });

    it('follows passkey → email → verify_code flow', () => {
      let state = SIGNIN_STATES.PASSKEY;

      // Passkey cancelled
      state = getNextSigninStateAfterPasskey(state, { success: false, cancelled: true });
      expect(state).toBe(SIGNIN_STATES.EMAIL);

      // Email submitted, only code available
      state = getNextSigninStateAfterEmail(state, { hasEmailLink: false, hasEmailCode: true });
      expect(state).toBe(SIGNIN_STATES.VERIFY_CODE);
    });
  });
});

describe('PasskeyGate state determination', () => {
  it('returns checking when user is null', () => {
    expect(getGateState(null)).toBe(GATE_STATES.CHECKING);
  });

  it('returns checking when user is undefined', () => {
    expect(getGateState(undefined)).toBe(GATE_STATES.CHECKING);
  });

  it('returns ready when user has passkeys', () => {
    const user = { passkeys: [{ id: 'pk_1' }] };
    expect(getGateState(user)).toBe(GATE_STATES.READY);
  });

  it('returns passkey when user has no passkeys', () => {
    const user = { passkeys: [] };
    expect(getGateState(user)).toBe(GATE_STATES.PASSKEY);
  });

  it('returns passkey when passkeys is undefined', () => {
    const user = {};
    expect(getGateState(user)).toBe(GATE_STATES.PASSKEY);
  });

  it('returns passkey when passkeys is null', () => {
    const user = { passkeys: null };
    expect(getGateState(user)).toBe(GATE_STATES.PASSKEY);
  });

  it('returns ready with multiple passkeys', () => {
    const user = { passkeys: [{ id: 'pk_1' }, { id: 'pk_2' }] };
    expect(getGateState(user)).toBe(GATE_STATES.READY);
  });
});

describe('ClaimPrompt state determination', () => {
  it('returns checking when user is null', () => {
    expect(getClaimPromptState(null)).toBe(GATE_STATES.CHECKING);
  });

  it('returns claim when user has passkeys', () => {
    const user = { passkeys: [{ id: 'pk_1' }] };
    expect(getClaimPromptState(user)).toBe(GATE_STATES.CLAIM);
  });

  it('returns passkey when user has no passkeys', () => {
    const user = { passkeys: [] };
    expect(getClaimPromptState(user)).toBe(GATE_STATES.PASSKEY);
  });

  it('enforces passkey before claim', () => {
    // User without passkey must create one before claiming
    const userWithoutPasskey = { passkeys: [] };
    expect(getClaimPromptState(userWithoutPasskey)).toBe(GATE_STATES.PASSKEY);

    // User with passkey can proceed to claim
    const userWithPasskey = { passkeys: [{ id: 'pk_1' }] };
    expect(getClaimPromptState(userWithPasskey)).toBe(GATE_STATES.CLAIM);
  });
});

describe('selectEmailVerificationStrategy', () => {
  it('prefers email_link when both available', () => {
    const factors = [
      { strategy: 'email_code', emailAddressId: 'email_code_123' },
      { strategy: 'email_link', emailAddressId: 'email_link_456' }
    ];
    const result = selectEmailVerificationStrategy(factors);
    expect(result.strategy).toBe('email_link');
    expect(result.emailAddressId).toBe('email_link_456');
  });

  it('falls back to email_code when email_link unavailable', () => {
    const factors = [
      { strategy: 'email_code', emailAddressId: 'email_code_123' },
      { strategy: 'password', identifier: 'user@example.com' }
    ];
    const result = selectEmailVerificationStrategy(factors);
    expect(result.strategy).toBe('email_code');
    expect(result.emailAddressId).toBe('email_code_123');
  });

  it('returns null when no email strategy available', () => {
    const factors = [
      { strategy: 'password', identifier: 'user@example.com' },
      { strategy: 'phone_code', phoneNumberId: 'phone_123' }
    ];
    const result = selectEmailVerificationStrategy(factors);
    expect(result.strategy).toBe(null);
    expect(result.emailAddressId).toBe(null);
  });

  it('handles empty array', () => {
    const result = selectEmailVerificationStrategy([]);
    expect(result.strategy).toBe(null);
    expect(result.emailAddressId).toBe(null);
  });

  it('handles null/undefined', () => {
    expect(selectEmailVerificationStrategy(null)).toEqual({ strategy: null, emailAddressId: null });
    expect(selectEmailVerificationStrategy(undefined)).toEqual({ strategy: null, emailAddressId: null });
  });

  it('extracts emailAddressId correctly', () => {
    const factors = [
      { strategy: 'email_code', emailAddressId: 'idn_2abc123def456' }
    ];
    const result = selectEmailVerificationStrategy(factors);
    expect(result.emailAddressId).toBe('idn_2abc123def456');
  });
});
