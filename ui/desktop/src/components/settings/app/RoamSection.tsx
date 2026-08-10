import { useCallback, useEffect, useState } from 'react';
import QRCode from 'qrcode';
import { Switch } from '../../ui/switch';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../../ui/card';
import { Button } from '../../ui/button';
import { Check, Copy } from 'lucide-react';
import { defineMessages, useIntl } from '../../../i18n';

const i18n = defineMessages({
  title: {
    id: 'roamSection.title',
    defaultMessage: 'Remote access (roam)',
  },
  description: {
    id: 'roamSection.description',
    defaultMessage:
      'Reach this goose from your phone or another machine, peer to peer. No accounts and no servers in between — devices pair by exchanging public keys.',
  },
  enable: {
    id: 'roamSection.enable',
    defaultMessage: 'Enable remote access',
  },
  enableDescription: {
    id: 'roamSection.enableDescription',
    defaultMessage: 'Also expose the local goose backend over goose roam (p2p).',
  },
  restartNote: {
    id: 'roamSection.restartNote',
    defaultMessage: 'Restart goose to apply.',
  },
  pairTitle: {
    id: 'roamSection.pairTitle',
    defaultMessage: 'Pair a device',
  },
  pairHelp: {
    id: 'roamSection.pairHelp',
    defaultMessage:
      'Scan this code from the goose web client on your phone, then accept the new device by running `goose roam peers accept` in a terminal.',
  },
  fingerprint: {
    id: 'roamSection.fingerprint',
    defaultMessage: 'Fingerprint',
  },
  copyCard: {
    id: 'roamSection.copyCard',
    defaultMessage: 'Copy connection card',
  },
  copied: {
    id: 'roamSection.copied',
    defaultMessage: 'Copied',
  },
  waiting: {
    id: 'roamSection.waiting',
    defaultMessage:
      'Waiting for the roaming endpoint to come online… (starts with the goose backend)',
  },
  devicesTitle: {
    id: 'roamSection.devicesTitle',
    defaultMessage: 'Paired devices',
  },
  devicesHelp: {
    id: 'roamSection.devicesHelp',
    defaultMessage:
      'Devices whose keys this goose accepts. Revoking a device disconnects it within seconds and it can no longer connect.',
  },
  noDevices: {
    id: 'roamSection.noDevices',
    defaultMessage: 'No devices paired yet.',
  },
  revoke: {
    id: 'roamSection.revoke',
    defaultMessage: 'Revoke',
  },
  revokeConfirm: {
    id: 'roamSection.revokeConfirm',
    defaultMessage: 'Really revoke?',
  },
  unnamedDevice: {
    id: 'roamSection.unnamedDevice',
    defaultMessage: 'Unnamed device',
  },
});

type RoamPeer = {
  name: string | null;
  endpointId: string;
  fingerprint: string;
  accepted: boolean;
  addedMs: number | null;
};

type RoamStatus = {
  card: string;
  endpointId: string;
  fingerprint: string;
  startedAt: number;
} | null;

export default function RoamSection() {
  const intl = useIntl();
  const [enabled, setEnabled] = useState(false);
  const [savedEnabled, setSavedEnabled] = useState(false);
  const [status, setStatus] = useState<RoamStatus>(null);
  const [qrDataUrl, setQrDataUrl] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const [peers, setPeers] = useState<RoamPeer[]>([]);
  const [confirmRevoke, setConfirmRevoke] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    const res = await window.electron.getRoamStatus();
    setStatus(res.status);
    setPeers(await window.electron.listRoamPeers());
  }, []);

  const revoke = useCallback(
    async (endpointId: string) => {
      if (confirmRevoke !== endpointId) {
        setConfirmRevoke(endpointId);
        return;
      }
      setConfirmRevoke(null);
      await window.electron.revokeRoamPeer(endpointId);
      setPeers(await window.electron.listRoamPeers());
    },
    [confirmRevoke]
  );

  useEffect(() => {
    const load = async () => {
      const value = await window.electron.getSetting('roamEnabled');
      setEnabled(value);
      setSavedEnabled(value);
      if (value) await refresh();
    };
    void load();
  }, [refresh]);

  // The status file appears once the roaming endpoint is online (a few
  // seconds after the backend starts); poll while enabled but not yet seen.
  useEffect(() => {
    if (!savedEnabled || status) return;
    const t = setInterval(() => void refresh(), 3000);
    return () => clearInterval(t);
  }, [savedEnabled, status, refresh]);

  useEffect(() => {
    if (!status?.card) {
      setQrDataUrl(null);
      return;
    }
    QRCode.toDataURL(status.card, { margin: 1, width: 220 })
      .then(setQrDataUrl)
      .catch(() => setQrDataUrl(null));
  }, [status?.card]);

  const handleToggle = async (value: boolean) => {
    setEnabled(value);
    await window.electron.setSetting('roamEnabled', value);
  };

  const copyCard = async () => {
    if (!status?.card) return;
    await navigator.clipboard.writeText(status.card);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };

  const showPairing = savedEnabled && enabled;

  return (
    <section id="roam" className="space-y-4 pr-4 mt-1">
      <Card className="pb-2">
        <CardHeader className="pb-0">
          <CardTitle>{intl.formatMessage(i18n.title)}</CardTitle>
          <CardDescription>{intl.formatMessage(i18n.description)}</CardDescription>
        </CardHeader>
        <CardContent className="pt-4 space-y-4 px-4">
          <div className="flex items-center justify-between">
            <div>
              <div className="text-sm font-medium">{intl.formatMessage(i18n.enable)}</div>
              <div className="text-xs text-text-muted">
                {intl.formatMessage(i18n.enableDescription)}
              </div>
            </div>
            <Switch checked={enabled} onCheckedChange={handleToggle} />
          </div>

          {enabled !== savedEnabled && (
            <div className="text-xs text-text-muted">{intl.formatMessage(i18n.restartNote)}</div>
          )}

          {showPairing &&
            (status ? (
              <div className="space-y-3 border-t border-border-subtle pt-4">
                <div className="text-sm font-medium">{intl.formatMessage(i18n.pairTitle)}</div>
                {qrDataUrl && (
                  <img
                    src={qrDataUrl}
                    alt="goose roam connection card QR code"
                    className="rounded-md border border-border-subtle bg-white p-1"
                    width={220}
                    height={220}
                  />
                )}
                <div className="text-xs text-text-muted">{intl.formatMessage(i18n.pairHelp)}</div>
                <div className="text-xs text-text-muted font-mono">
                  {intl.formatMessage(i18n.fingerprint)}: {status.fingerprint}
                </div>
                <Button variant="outline" size="sm" onClick={copyCard}>
                  {copied ? <Check className="w-3.5 h-3.5" /> : <Copy className="w-3.5 h-3.5" />}
                  {copied
                    ? intl.formatMessage(i18n.copied)
                    : intl.formatMessage(i18n.copyCard)}
                </Button>
              </div>
            ) : (
              <div className="text-xs text-text-muted border-t border-border-subtle pt-4">
                {intl.formatMessage(i18n.waiting)}
              </div>
            ))}

          {showPairing && (
            <div className="space-y-2 border-t border-border-subtle pt-4">
              <div className="text-sm font-medium">{intl.formatMessage(i18n.devicesTitle)}</div>
              <div className="text-xs text-text-muted">{intl.formatMessage(i18n.devicesHelp)}</div>
              {peers.filter((p) => p.accepted).length === 0 ? (
                <div className="text-xs text-text-muted">{intl.formatMessage(i18n.noDevices)}</div>
              ) : (
                <div className="space-y-1">
                  {peers
                    .filter((p) => p.accepted)
                    .map((p) => (
                      <div
                        key={p.endpointId}
                        className="flex items-center justify-between gap-2 rounded-md border border-border-subtle px-3 py-2"
                      >
                        <div className="min-w-0">
                          <div className="text-sm truncate">
                            {p.name ?? intl.formatMessage(i18n.unnamedDevice)}
                          </div>
                          <div className="text-[11px] text-text-muted font-mono truncate">
                            {p.fingerprint}
                          </div>
                        </div>
                        <Button
                          variant={confirmRevoke === p.endpointId ? 'destructive' : 'outline'}
                          size="sm"
                          className="shrink-0"
                          onClick={() => void revoke(p.endpointId)}
                          onBlur={() =>
                            setConfirmRevoke((c) => (c === p.endpointId ? null : c))
                          }
                        >
                          {confirmRevoke === p.endpointId
                            ? intl.formatMessage(i18n.revokeConfirm)
                            : intl.formatMessage(i18n.revoke)}
                        </Button>
                      </div>
                    ))}
                </div>
              )}
            </div>
          )}
        </CardContent>
      </Card>
    </section>
  );
}
