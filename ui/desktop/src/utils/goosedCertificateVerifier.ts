type Certificate = {
  fingerprint: string;
};

type CertificateVerifyRequest = {
  hostname: string;
  certificate: Certificate;
};

type CertificateVerifyProc = (
  request: CertificateVerifyRequest,
  callback: (verificationResult: number) => void
) => void;

type CertificateVerifierSession = {
  setCertificateVerifyProc: (proc: CertificateVerifyProc) => void;
};

type WindowWithSession = {
  webContents: {
    session: CertificateVerifierSession;
  };
};

let pinnedCertFingerprint: string | null = null;
let trustedExternalHostname: string | null = null;
const installedSessions = new WeakSet<CertificateVerifierSession>();

function isLocalhost(hostname: string): boolean {
  return hostname === '127.0.0.1' || hostname === 'localhost';
}

function isTrustedHost(hostname: string): boolean {
  if (isLocalhost(hostname)) return true;
  return trustedExternalHostname !== null && hostname === trustedExternalHostname;
}

function normalizeFingerprint(fp: string): string {
  if (fp.startsWith('sha256/')) {
    const b64 = fp.slice('sha256/'.length);
    const buf = Buffer.from(b64, 'base64');
    return Array.from(buf)
      .map((b) => b.toString(16).padStart(2, '0'))
      .join(':')
      .toUpperCase();
  }
  return fp.toUpperCase();
}

export function setGoosedPinnedCertFingerprint(fingerprint: string | null) {
  pinnedCertFingerprint = fingerprint ? normalizeFingerprint(fingerprint) : null;
}

export function setTrustedExternalGoosedHostname(hostname: string | null) {
  trustedExternalHostname = hostname;
}

export function handleGoosedCertificateError(
  event: { preventDefault: () => void },
  url: string,
  certificate: Certificate,
  callback: (isTrusted: boolean) => void
) {
  const parsed = new URL(url);
  if (!isTrustedHost(parsed.hostname)) {
    callback(false);
    return;
  }

  event.preventDefault();

  const fingerprint = normalizeFingerprint(certificate.fingerprint);
  if (!pinnedCertFingerprint) {
    pinnedCertFingerprint = fingerprint;
    callback(true);
    return;
  }

  callback(fingerprint === pinnedCertFingerprint.toUpperCase());
}

export function installGoosedCertificateVerifier(session: CertificateVerifierSession) {
  if (installedSessions.has(session)) return;
  installedSessions.add(session);

  session.setCertificateVerifyProc((request, callback) => {
    if (!isTrustedHost(request.hostname)) {
      callback(-3);
      return;
    }

    const fingerprint = normalizeFingerprint(request.certificate.fingerprint);
    if (!pinnedCertFingerprint) {
      pinnedCertFingerprint = fingerprint;
      callback(0);
      return;
    }

    callback(fingerprint === pinnedCertFingerprint.toUpperCase() ? 0 : -2);
  });
}

export function installGoosedCertificateVerifierForWindow(window: WindowWithSession) {
  installGoosedCertificateVerifier(window.webContents.session);
}
