/**
 * Cryptographic utilities for Fireproof Connect token generation
 *
 * Shared between deploy-exe.js and deploy-connect.js
 */

import { webcrypto } from 'crypto';
const { subtle } = webcrypto;

// Base58btc alphabet (multibase compatible)
const BASE58_ALPHABET = '123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz';

/**
 * Encode bytes to base58
 */
export function base58Encode(bytes) {
  const uint8 = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
  const digits = [0];
  for (const byte of uint8) {
    let carry = byte;
    for (let i = 0; i < digits.length; i++) {
      carry += digits[i] << 8;
      digits[i] = carry % 58;
      carry = Math.floor(carry / 58);
    }
    while (carry > 0) {
      digits.push(carry % 58);
      carry = Math.floor(carry / 58);
    }
  }
  // Leading zeros become '1'
  let result = '';
  for (const byte of uint8) {
    if (byte === 0) result += '1';
    else break;
  }
  // Convert digits to characters
  for (let i = digits.length - 1; i >= 0; i--) {
    result += BASE58_ALPHABET[digits[i]];
  }
  return result;
}

/**
 * Encode a JWK object to base58btc (matching Fireproof's jwk2env format)
 * Format: 'z' + base58btc(utf8(JSON.stringify(jwk)))
 */
export function jwkToEnv(jwk) {
  const jsonStr = JSON.stringify(jwk);
  const bytes = new TextEncoder().encode(jsonStr);
  return 'z' + base58Encode(bytes);
}

/**
 * Generate EC P-256 key pair and return as JWK-encoded env strings
 */
export async function generateSessionTokens() {
  const keyPair = await subtle.generateKey(
    { name: 'ECDSA', namedCurve: 'P-256' },
    true,
    ['sign', 'verify']
  );

  const publicJwk = await subtle.exportKey('jwk', keyPair.publicKey);
  const privateJwk = await subtle.exportKey('jwk', keyPair.privateKey);

  publicJwk.alg = 'ES256';
  privateJwk.alg = 'ES256';

  const publicEnv = jwkToEnv(publicJwk);
  const privateEnv = jwkToEnv(privateJwk);

  return { publicEnv, privateEnv };
}

/**
 * Generate Device CA key pair and certificate
 * @param {Object} options - Certificate options
 * @param {string} options.issuer - Issuer name (default: 'Docker Dev CA')
 * @param {string} options.organization - Organization name (default: 'Vibes DIY Development')
 * @param {string} options.locality - Locality (default: 'Local')
 * @param {string} options.state - State/Province (default: 'Development')
 */
export async function generateDeviceCAKeys(options = {}) {
  const {
    issuer = 'Docker Dev CA',
    organization = 'Vibes DIY Development',
    locality = 'Local',
    state = 'Development'
  } = options;

  const keyPair = await subtle.generateKey(
    { name: 'ECDSA', namedCurve: 'P-256' },
    true,
    ['sign', 'verify']
  );

  const privateJwk = await subtle.exportKey('jwk', keyPair.privateKey);
  privateJwk.alg = 'ES256';
  const privKey = jwkToEnv(privateJwk);

  const publicJwk = await subtle.exportKey('jwk', keyPair.publicKey);

  const now = Math.floor(Date.now() / 1000);
  const oneYear = 365 * 24 * 60 * 60;

  const kidBytes = new Uint8Array(32);
  webcrypto.getRandomValues(kidBytes);
  const kid = base58Encode(kidBytes);

  const header = {
    alg: 'ES256',
    typ: 'CERT+JWT',
    kid: kid,
    x5c: []
  };

  const jtiBytes = new Uint8Array(32);
  webcrypto.getRandomValues(jtiBytes);

  const serialBytes = new Uint8Array(32);
  webcrypto.getRandomValues(serialBytes);

  const payload = {
    iss: issuer,
    sub: issuer,
    aud: 'certificate-users',
    iat: now,
    nbf: now,
    exp: now + oneYear,
    jti: base58Encode(jtiBytes),
    certificate: {
      version: '3',
      serialNumber: base58Encode(serialBytes),
      subject: {
        commonName: issuer,
        organization: organization,
        locality: locality,
        stateOrProvinceName: state,
        countryName: 'WD'
      },
      issuer: {
        commonName: issuer,
        organization: organization,
        locality: locality,
        stateOrProvinceName: state,
        countryName: 'WD'
      },
      validity: {
        notBefore: new Date(now * 1000).toISOString(),
        notAfter: new Date((now + oneYear) * 1000).toISOString()
      },
      subjectPublicKeyInfo: {
        kty: publicJwk.kty,
        crv: publicJwk.crv,
        x: publicJwk.x,
        y: publicJwk.y
      },
      signatureAlgorithm: 'ES256',
      keyUsage: ['digitalSignature', 'keyEncipherment'],
      extendedKeyUsage: ['serverAuth']
    }
  };

  const headerB64 = Buffer.from(JSON.stringify(header)).toString('base64url');
  const payloadB64 = Buffer.from(JSON.stringify(payload)).toString('base64url');

  const dataToSign = new TextEncoder().encode(`${headerB64}.${payloadB64}`);
  const signature = await subtle.sign(
    { name: 'ECDSA', hash: 'SHA-256' },
    keyPair.privateKey,
    dataToSign
  );
  const signatureB64 = Buffer.from(signature).toString('base64url');

  const cert = `${headerB64}.${payloadB64}.${signatureB64}`;

  return { privKey, cert };
}
