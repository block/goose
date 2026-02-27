import React, { useState, useEffect, useRef } from 'react';
import { Zap, Copy, Check, Loader2 } from 'lucide-react';
import { getApiUrl } from '../../config';

interface Invoice {
  bolt11: string;
  qr_svg: string;
  amount_sats: number | null;
}

interface LightningPaymentProps {
  onPaymentReceived?: () => void;
}

export const LightningPayment: React.FC<LightningPaymentProps> = ({ onPaymentReceived }) => {
  const [amountSats, setAmountSats] = useState<string>('1000');
  const [invoice, setInvoice] = useState<Invoice | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const [paymentReceived, setPaymentReceived] = useState(false);
  const eventSourceRef = useRef<EventSource | null>(null);

  // Connect to SSE for payment events when an invoice is displayed.
  useEffect(() => {
    if (!invoice) return;

    const evtSource = new EventSource(getApiUrl('/wallet/events'));
    eventSourceRef.current = evtSource;

    evtSource.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data);
        if (data.amount_sats) {
          setPaymentReceived(true);
          onPaymentReceived?.();
        }
      } catch {
        // Ignore parse errors on keep-alive pings.
      }
    };

    evtSource.onerror = () => {
      // SSE will auto-reconnect; nothing to do here.
    };

    return () => {
      evtSource.close();
      eventSourceRef.current = null;
    };
  }, [invoice, onPaymentReceived]);

  const handleCreateInvoice = async () => {
    setLoading(true);
    setError(null);
    setInvoice(null);
    setPaymentReceived(false);

    try {
      const amount = parseInt(amountSats, 10);
      if (isNaN(amount) || amount <= 0) {
        setError('Please enter a valid amount in satoshis');
        setLoading(false);
        return;
      }

      const secretKey = await window.electron.getSecretKey();
      const resp = await fetch(getApiUrl('/wallet/invoice'), {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'X-Secret-Key': secretKey,
        },
        body: JSON.stringify({ amount_sats: amount }),
      });

      if (!resp.ok) {
        const text = await resp.text();
        setError(`Failed to create invoice: ${text}`);
        setLoading(false);
        return;
      }

      const data: Invoice = await resp.json();
      setInvoice(data);
    } catch (e) {
      setError(`Error: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      setLoading(false);
    }
  };

  const handleCopy = async () => {
    if (!invoice) return;
    try {
      await navigator.clipboard.writeText(invoice.bolt11);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      // Clipboard write failed — not critical.
    }
  };

  if (paymentReceived) {
    return (
      <div className="rounded-lg border border-green-600/30 dark:border-green-500/30 bg-green-500/10 dark:bg-green-500/10 p-4 my-2">
        <div className="flex items-center gap-2">
          <Check className="h-5 w-5 text-green-600 dark:text-green-400" />
          <span className="text-sm font-semibold text-green-800 dark:text-green-200">
            Payment received!
          </span>
        </div>
      </div>
    );
  }

  return (
    <div className="rounded-lg border border-orange-600/30 dark:border-orange-500/30 bg-orange-500/10 dark:bg-orange-500/10 p-4 my-2">
      <div className="flex items-start gap-3">
        <Zap className="h-4 w-4 text-orange-600 dark:text-orange-400 mt-0.5 shrink-0" />
        <div className="flex-1">
          <div className="text-sm font-semibold text-orange-800 dark:text-orange-200">
            Pay with Lightning
          </div>

          {!invoice && (
            <div className="mt-3 flex items-center gap-2">
              <input
                type="number"
                min="1"
                value={amountSats}
                onChange={(e) => setAmountSats(e.target.value)}
                placeholder="Amount in sats"
                className="w-32 rounded-md border border-orange-300 dark:border-orange-600 bg-white dark:bg-gray-800 text-sm px-3 py-1.5 text-orange-900 dark:text-orange-100"
              />
              <span className="text-xs text-orange-700 dark:text-orange-300">sats</span>
              <button
                onClick={handleCreateInvoice}
                disabled={loading}
                className="inline-flex items-center gap-1.5 rounded-md bg-orange-600 hover:bg-orange-500 dark:bg-orange-700 dark:hover:bg-orange-600 text-white text-sm font-medium px-3 py-1.5 transition-colors disabled:opacity-50"
              >
                {loading ? (
                  <Loader2 className="h-3.5 w-3.5 animate-spin" />
                ) : (
                  <Zap className="h-3.5 w-3.5" />
                )}
                Create Invoice
              </button>
            </div>
          )}

          {error && (
            <div className="mt-2 text-sm text-red-600 dark:text-red-400">{error}</div>
          )}

          {invoice && (
            <div className="mt-3 space-y-3">
              {/* QR Code */}
              <div
                className="bg-white rounded-lg p-2 w-fit"
                dangerouslySetInnerHTML={{ __html: invoice.qr_svg }}
              />

              {/* BOLT11 string */}
              <div className="flex items-center gap-2">
                <code className="text-xs text-orange-800 dark:text-orange-200 bg-orange-100 dark:bg-orange-900/30 rounded px-2 py-1 break-all max-w-[300px] truncate">
                  {invoice.bolt11}
                </code>
                <button
                  onClick={handleCopy}
                  className="shrink-0 p-1 rounded hover:bg-orange-200 dark:hover:bg-orange-800 transition-colors"
                  title="Copy invoice"
                >
                  {copied ? (
                    <Check className="h-3.5 w-3.5 text-green-600" />
                  ) : (
                    <Copy className="h-3.5 w-3.5 text-orange-600 dark:text-orange-400" />
                  )}
                </button>
              </div>

              {/* Waiting indicator */}
              <div className="flex items-center gap-2 text-sm text-orange-700 dark:text-orange-300">
                <Loader2 className="h-3.5 w-3.5 animate-spin" />
                Waiting for payment...
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
