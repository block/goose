/**
 * Auth Flow State Machines
 *
 * Pure functions for auth flow logic, extracted for testing.
 * These represent the state transitions in the auth components.
 */

/**
 * PasskeySignupFlow states
 */
export const SIGNUP_STATES = {
  EMAIL: 'email',
  VERIFY: 'verify',
  PASSKEY: 'passkey',
  CLAIMING: 'claiming',
  DONE: 'done'
};

/**
 * PasskeySigninFlow states
 */
export const SIGNIN_STATES = {
  PASSKEY: 'passkey',
  EMAIL: 'email',
  VERIFY_LINK: 'verify_link',
  VERIFY_CODE: 'verify_code'
};

/**
 * ClaimPrompt/PasskeyGate states
 */
export const GATE_STATES = {
  CHECKING: 'checking',
  PASSKEY: 'passkey',
  CLAIM: 'claim',
  READY: 'ready'
};

/**
 * Determine next signup state after email submission
 * @param {string} currentState
 * @param {object} signUpResult - Clerk signup result
 * @returns {string} Next state
 */
export function getNextSignupState(currentState, signUpResult) {
  if (currentState === SIGNUP_STATES.EMAIL) {
    // After email submission, go to verify
    return SIGNUP_STATES.VERIFY;
  }

  if (currentState === SIGNUP_STATES.VERIFY) {
    // After verification, check status
    if (signUpResult.status === 'complete' || signUpResult.status === 'missing_requirements') {
      return SIGNUP_STATES.PASSKEY;
    }
    // Stay in verify if not complete
    return SIGNUP_STATES.VERIFY;
  }

  if (currentState === SIGNUP_STATES.PASSKEY) {
    return SIGNUP_STATES.CLAIMING;
  }

  if (currentState === SIGNUP_STATES.CLAIMING) {
    return SIGNUP_STATES.DONE;
  }

  return currentState;
}

/**
 * Determine next signin state after passkey attempt
 * @param {string} currentState
 * @param {{ success: boolean, cancelled?: boolean }} result
 * @returns {string} Next state
 */
export function getNextSigninStateAfterPasskey(currentState, result) {
  if (currentState === SIGNIN_STATES.PASSKEY) {
    if (result.success) {
      return 'complete'; // Special state indicating done
    }
    // Failed or cancelled - go to email fallback
    return SIGNIN_STATES.EMAIL;
  }
  return currentState;
}

/**
 * Determine next signin state after email submission
 * @param {string} currentState
 * @param {{ hasEmailLink: boolean, hasEmailCode: boolean }} factors
 * @returns {string} Next state
 */
export function getNextSigninStateAfterEmail(currentState, factors) {
  if (currentState === SIGNIN_STATES.EMAIL) {
    if (factors.hasEmailLink) {
      return SIGNIN_STATES.VERIFY_LINK;
    }
    if (factors.hasEmailCode) {
      return SIGNIN_STATES.VERIFY_CODE;
    }
    // No verification method - stay in email with error
    return SIGNIN_STATES.EMAIL;
  }
  return currentState;
}

/**
 * Determine gate state based on user's passkeys
 * @param {object|null} user - Clerk user object
 * @returns {string} Gate state
 */
export function getGateState(user) {
  if (!user) {
    return GATE_STATES.CHECKING;
  }

  const hasPasskey = user.passkeys && user.passkeys.length > 0;
  return hasPasskey ? GATE_STATES.READY : GATE_STATES.PASSKEY;
}

/**
 * Determine claim prompt state based on user's passkeys
 * @param {object|null} user - Clerk user object
 * @returns {string} Claim prompt state
 */
export function getClaimPromptState(user) {
  if (!user) {
    return GATE_STATES.CHECKING;
  }

  const hasPasskey = user.passkeys && user.passkeys.length > 0;
  return hasPasskey ? GATE_STATES.CLAIM : GATE_STATES.PASSKEY;
}

/**
 * Determine which email verification strategy to use
 * @param {Array} supportedFactors - Clerk's supportedFirstFactors array
 * @returns {{ strategy: string|null, emailAddressId: string|null }}
 */
export function selectEmailVerificationStrategy(supportedFactors) {
  if (!supportedFactors || !Array.isArray(supportedFactors)) {
    return { strategy: null, emailAddressId: null };
  }

  // Prefer email_link, fallback to email_code
  const emailLink = supportedFactors.find(f => f.strategy === 'email_link');
  if (emailLink) {
    return { strategy: 'email_link', emailAddressId: emailLink.emailAddressId };
  }

  const emailCode = supportedFactors.find(f => f.strategy === 'email_code');
  if (emailCode) {
    return { strategy: 'email_code', emailAddressId: emailCode.emailAddressId };
  }

  return { strategy: null, emailAddressId: null };
}
