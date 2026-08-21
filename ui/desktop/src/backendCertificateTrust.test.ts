import { describe, expect, it } from 'vitest';
import { BackendCertificateTrustStore } from './backendCertificateTrust';

const HOSTNAME = 'backend.example';
const CERTIFICATE_A = 'AA:AA';
const CERTIFICATE_B = 'BB:BB';
const CERTIFICATE_C = 'CC:CC';

describe('BackendCertificateTrustStore', () => {
  it('inherits a learned hostname TOFU pin for later registrations', () => {
    const store = new BackendCertificateTrustStore();
    const first = store.register(HOSTNAME, null, 'hostname-tofu');

    expect(store.verify(HOSTNAME, CERTIFICATE_A)).toBe(true);

    const second = store.register(HOSTNAME, null, 'hostname-tofu');
    expect(first.trust.fingerprint).toBe(CERTIFICATE_A);
    expect(second.trust.fingerprint).toBe(CERTIFICATE_A);
    expect(store.verify(HOSTNAME, CERTIFICATE_B)).toBe(false);
  });

  it('binds concurrent hostname TOFU registrations to the same first certificate', () => {
    const store = new BackendCertificateTrustStore();
    const first = store.register(HOSTNAME, null, 'hostname-tofu');
    const second = store.register(HOSTNAME, null, 'hostname-tofu');

    expect(store.verify(HOSTNAME, CERTIFICATE_A)).toBe(true);
    expect(first.trust.fingerprint).toBe(CERTIFICATE_A);
    expect(second.trust.fingerprint).toBe(CERTIFICATE_A);
    expect(store.verify(HOSTNAME, CERTIFICATE_B)).toBe(false);
  });

  it('does not bind hostname TOFU from a lease-scoped exact match', () => {
    const store = new BackendCertificateTrustStore();
    store.register(HOSTNAME, CERTIFICATE_A);
    const tofu = store.register(HOSTNAME, null, 'hostname-tofu');

    expect(store.verify(HOSTNAME, CERTIFICATE_A)).toBe(true);
    expect(tofu.trust.fingerprint).toBeNull();
    expect(store.verify(HOSTNAME, CERTIFICATE_B)).toBe(true);
    expect(tofu.trust.fingerprint).toBe(CERTIFICATE_B);
    expect(store.verify(HOSTNAME, CERTIFICATE_C)).toBe(false);
  });

  it.each(['first', 'second'] as const)(
    'preserves the hostname TOFU pin when the %s registration is released',
    (releasedRegistration) => {
      const store = new BackendCertificateTrustStore();
      const first = store.register(HOSTNAME, null, 'hostname-tofu');
      const second = store.register(HOSTNAME, null, 'hostname-tofu');
      expect(store.verify(HOSTNAME, CERTIFICATE_A)).toBe(true);

      if (releasedRegistration === 'first') {
        first.release();
      } else {
        second.release();
      }

      expect(store.verify(HOSTNAME, CERTIFICATE_B)).toBe(false);
      expect(store.verify(HOSTNAME, CERTIFICATE_A)).toBe(true);
    }
  );

  it('allows fresh TOFU after the last registration is released', () => {
    const store = new BackendCertificateTrustStore();
    const first = store.register(HOSTNAME, null, 'hostname-tofu');
    const second = store.register(HOSTNAME, null, 'hostname-tofu');
    expect(store.verify(HOSTNAME, CERTIFICATE_A)).toBe(true);

    first.release();
    second.release();
    expect(store.has(HOSTNAME)).toBe(false);

    store.register(HOSTNAME, null, 'hostname-tofu');
    expect(store.verify(HOSTNAME, CERTIFICATE_B)).toBe(true);
  });

  it('keeps hostname TOFU pins independent across hosts', () => {
    const store = new BackendCertificateTrustStore();
    store.register('one.example', null, 'hostname-tofu');
    store.register('two.example', null, 'hostname-tofu');

    expect(store.verify('one.example', CERTIFICATE_A)).toBe(true);
    expect(store.verify('two.example', CERTIFICATE_B)).toBe(true);
    expect(store.verify('one.example', CERTIFICATE_B)).toBe(false);
    expect(store.verify('two.example', CERTIFICATE_A)).toBe(false);
  });

  it('requires an exact match for an explicit certificate pin', () => {
    const store = new BackendCertificateTrustStore();
    store.register(HOSTNAME, CERTIFICATE_A);

    expect(store.verify(HOSTNAME, CERTIFICATE_A)).toBe(true);
    expect(store.verify(HOSTNAME, CERTIFICATE_B)).toBe(false);
  });

  it('keeps explicit pins from overlapping lease rotations valid', () => {
    const store = new BackendCertificateTrustStore();
    store.register(HOSTNAME, CERTIFICATE_A);
    store.register(HOSTNAME, CERTIFICATE_B);

    expect(store.verify(HOSTNAME, CERTIFICATE_A)).toBe(true);
    expect(store.verify(HOSTNAME, CERTIFICATE_B)).toBe(true);
    expect(store.verify(HOSTNAME, CERTIFICATE_C)).toBe(false);
  });

  it('keeps explicit and hostname TOFU pins independently valid', () => {
    const store = new BackendCertificateTrustStore();
    store.register(HOSTNAME, null, 'hostname-tofu');
    expect(store.verify(HOSTNAME, CERTIFICATE_A)).toBe(true);

    store.register(HOSTNAME, CERTIFICATE_B);
    const laterTofu = store.register(HOSTNAME, null, 'hostname-tofu');

    expect(laterTofu.trust.fingerprint).toBe(CERTIFICATE_A);
    expect(store.verify(HOSTNAME, CERTIFICATE_A)).toBe(true);
    expect(store.verify(HOSTNAME, CERTIFICATE_B)).toBe(true);
    expect(store.verify(HOSTNAME, CERTIFICATE_C)).toBe(false);
  });

  it('allows local lease-scoped registrations to learn different certificates', () => {
    const store = new BackendCertificateTrustStore();
    const first = store.register('127.0.0.1', null);
    expect(store.verify('127.0.0.1', CERTIFICATE_A)).toBe(true);

    const second = store.register('127.0.0.1', null);
    expect(store.verify('127.0.0.1', CERTIFICATE_B)).toBe(true);

    expect(first.trust.fingerprint).toBe(CERTIFICATE_A);
    expect(second.trust.fingerprint).toBe(CERTIFICATE_B);
    expect(store.verify('127.0.0.1', CERTIFICATE_A)).toBe(true);
    expect(store.verify('127.0.0.1', CERTIFICATE_B)).toBe(true);
  });
});
