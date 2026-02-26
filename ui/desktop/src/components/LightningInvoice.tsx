import React, { useState, useEffect } from 'react';
import { Zap, Loader2, Check, AlertTriangle } from 'lucide-react';
import { getApiUrl } from '../config';

// BOLT11 invoices start with lnbc (mainnet), lntb (testnet), lntbs (signet), lnbcrt (regtest)
const BOLT11_REGEX = /\b(lnbc[a-z0-9]+1[qpzry9x8gf2tvdw0s3jn54khce6mua7l]+)\b/gi;

/** Extract all BOLT11 invoice strings from text. */
export function extractBolt11Invoices(text: string): string[] {
  const matches = text.match(BOLT11_REGEX);
  if (!matches) return [];
  return [...new Set(matches.map((m) => m.toLowerCase()))];
}

interface ParsedInvoice {
  amount_sats: number | null;
  description: string | null;
}

interface LightningInvoiceProps {
  bolt11: string;
}

export const LightningInvoice: React.FC<LightningInvoiceProps> = ({ bolt11 }) => {
  const [parsed, setParsed] = useState<ParsedInvoice | null>(null);
  const [parsing, setParsing] = useState(true);
  const [paying, setPaying] = useState(false);
  const [paid, setPaid] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [confirmed, setConfirmed] = useState(false);

  // Parse the invoice on mount to show amount before paying.
  useEffect(() => {
    const parse = async () => {
      try {
        const secretKey = await window.electron.getSecretKey();
        const resp = await fetch(getApiUrl('/wallet/parse-invoice'), {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
            'X-Secret-Key': secretKey,
          },
          body: JSON.stringify({ bolt11 }),
        });

        if (resp.ok) {
          setParsed(await resp.json());
        } else {
          try {
            const data = await resp.json();
            setError(data.error || 'Failed to parse invoice');
          } catch {
            setError('Failed to parse invoice');
          }
        }
      } catch {
        setError('Failed to connect to wallet');
      } finally {
        setParsing(false);
      }
    };
    parse();
  }, [bolt11]);

  const handlePay = async () => {
    setPaying(true);
    setError(null);
    try {
      const secretKey = await window.electron.getSecretKey();
      const resp = await fetch(getApiUrl('/wallet/pay'), {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'X-Secret-Key': secretKey,
        },
        body: JSON.stringify({ bolt11 }),
      });

      if (!resp.ok) {
        try {
          const data = await resp.json();
          setError(data.error || 'Payment failed');
        } catch {
          setError(`Payment failed (${resp.status})`);
        }
        setConfirmed(false);
        return;
      }

      setPaid(true);
    } catch (e) {
      setError(`Error: ${e instanceof Error ? e.message : String(e)}`);
      setConfirmed(false);
    } finally {
      setPaying(false);
    }
  };

  if (paid) {
    return (
      <div className="inline-flex items-center gap-1.5 rounded-md bg-green-500/10 border border-green-600/30 text-green-700 dark:text-green-300 text-xs font-medium px-2.5 py-1 my-1">
        <Check className="h-3 w-3" />
        Paid{parsed?.amount_sats ? ` ${parsed.amount_sats.toLocaleString()} sats` : ''}
      </div>
    );
  }

  if (parsing) {
    return (
      <div className="inline-flex items-center gap-1.5 text-xs text-gray-500 my-1">
        <Loader2 className="h-3 w-3 animate-spin" />
        Parsing invoice...
      </div>
    );
  }

  if (error && !parsed) {
    return (
      <div className="inline-flex items-center gap-1 text-xs text-red-600 dark:text-red-400 my-1">
        <AlertTriangle className="h-3 w-3" />
        {error}
      </div>
    );
  }

  const amountLabel = parsed?.amount_sats
    ? `${parsed.amount_sats.toLocaleString()} sats`
    : 'unknown amount';

  if (!confirmed) {
    return (
      <div className="flex items-center gap-2 my-1">
        <button
          onClick={() => setConfirmed(true)}
          className="inline-flex items-center gap-1.5 rounded-md bg-orange-600 hover:bg-orange-500 text-white text-xs font-medium px-2.5 py-1 transition-colors"
        >
          <Zap className="h-3 w-3" />
          Pay {amountLabel}
        </button>
      </div>
    );
  }

  return (
    <div className="flex items-center gap-2 my-1">
      <span className="text-xs text-gray-600 dark:text-gray-400">
        Send {amountLabel}?
      </span>
      <button
        onClick={handlePay}
        disabled={paying}
        className="inline-flex items-center gap-1.5 rounded-md bg-orange-600 hover:bg-orange-500 text-white text-xs font-medium px-2.5 py-1 transition-colors disabled:opacity-50"
      >
        {paying ? (
          <Loader2 className="h-3 w-3 animate-spin" />
        ) : (
          <Zap className="h-3 w-3" />
        )}
        Confirm
      </button>
      <button
        onClick={() => setConfirmed(false)}
        disabled={paying}
        className="text-xs text-gray-500 hover:text-gray-700 dark:hover:text-gray-300"
      >
        Cancel
      </button>
      {error && (
        <span className="inline-flex items-center gap-1 text-xs text-red-600 dark:text-red-400">
          <AlertTriangle className="h-3 w-3" />
          {error}
        </span>
      )}
    </div>
  );
};
