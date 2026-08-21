import { Buffer } from 'node:buffer';

export type BackendCertificatePinScope = 'lease' | 'hostname-tofu';

export interface BackendCertificateTrust {
  hostname: string;
  fingerprint: string | null;
  pinScope: BackendCertificatePinScope;
}

export interface BackendCertificateTrustRegistration {
  trust: BackendCertificateTrust;
  release: () => void;
}

function normalizeHostname(hostname: string): string {
  return hostname.toLowerCase();
}

export function normalizeFingerprint(fingerprint: string): string {
  if (fingerprint.startsWith('sha256/')) {
    const base64 = fingerprint.slice('sha256/'.length);
    const buffer = Buffer.from(base64, 'base64');
    return Array.from(buffer)
      .map((byte) => byte.toString(16).padStart(2, '0'))
      .join(':')
      .toUpperCase();
  }
  return fingerprint.toUpperCase();
}

export class BackendCertificateTrustStore {
  private readonly trusts = new Set<BackendCertificateTrust>();

  register(
    hostname: string,
    fingerprint: string | null,
    pinScope: BackendCertificatePinScope = 'lease'
  ): BackendCertificateTrustRegistration {
    const normalizedHostname = normalizeHostname(hostname);
    const inheritedFingerprint =
      pinScope === 'hostname-tofu' && fingerprint === null
        ? (this.forHostname(normalizedHostname).find(
            (trust) => trust.pinScope === 'hostname-tofu' && trust.fingerprint !== null
          )?.fingerprint ?? null)
        : null;
    const trust: BackendCertificateTrust = {
      hostname: normalizedHostname,
      fingerprint: fingerprint ? normalizeFingerprint(fingerprint) : inheritedFingerprint,
      pinScope,
    };
    this.trusts.add(trust);
    return {
      trust,
      release: () => {
        this.trusts.delete(trust);
      },
    };
  }

  verify(hostname: string, fingerprint: string): boolean {
    const normalizedFingerprint = normalizeFingerprint(fingerprint);
    const trusts = this.forHostname(hostname);
    if (trusts.length === 0) {
      return false;
    }

    const unpinnedHostnameTofuTrusts = trusts.filter(
      (trust) => trust.pinScope === 'hostname-tofu' && trust.fingerprint === null
    );
    const exactMatches = trusts.filter((trust) => trust.fingerprint === normalizedFingerprint);
    if (exactMatches.length > 0) {
      if (exactMatches.some((trust) => trust.pinScope === 'hostname-tofu')) {
        this.bindAll(unpinnedHostnameTofuTrusts, normalizedFingerprint);
      }
      return true;
    }

    if (unpinnedHostnameTofuTrusts.length > 0) {
      this.bindAll(unpinnedHostnameTofuTrusts, normalizedFingerprint);
      return true;
    }

    const unpinnedLeaseTrust = trusts.find(
      (trust) => trust.pinScope === 'lease' && trust.fingerprint === null
    );
    if (!unpinnedLeaseTrust) {
      return false;
    }

    unpinnedLeaseTrust.fingerprint = normalizedFingerprint;
    return true;
  }

  has(hostname: string): boolean {
    return this.forHostname(hostname).length > 0;
  }

  private forHostname(hostname: string): BackendCertificateTrust[] {
    const normalizedHostname = normalizeHostname(hostname);
    return [...this.trusts].filter((trust) => trust.hostname === normalizedHostname);
  }

  private bindAll(trusts: BackendCertificateTrust[], fingerprint: string): void {
    for (const trust of trusts) {
      trust.fingerprint = fingerprint;
    }
  }
}
