import { describe, expect, it, vi } from 'vitest';
import {
  installGoosedCertificateVerifier,
  installGoosedCertificateVerifierForWindow,
} from './goosedCertificateVerifier';

describe('goosed certificate verifier', () => {
  it('installs the verifier on the BrowserWindow session', () => {
    const setCertificateVerifyProc = vi.fn();

    installGoosedCertificateVerifierForWindow({
      webContents: {
        session: { setCertificateVerifyProc },
      },
    });

    expect(setCertificateVerifyProc).toHaveBeenCalledOnce();
    expect(setCertificateVerifyProc).toHaveBeenCalledWith(expect.any(Function));
  });

  it('does not reinstall the verifier for the same session', () => {
    const session = { setCertificateVerifyProc: vi.fn() };

    installGoosedCertificateVerifier(session);
    installGoosedCertificateVerifier(session);

    expect(session.setCertificateVerifyProc).toHaveBeenCalledOnce();
  });
});
