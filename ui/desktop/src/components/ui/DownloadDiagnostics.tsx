import React, { useState } from 'react';
import { AlertTriangle } from 'lucide-react';
import { Button } from './button';
import { toastError } from '../../toasts';
import { diagnostics } from '../../api';

interface DiagnosticsModalProps {
  isOpen: boolean;
  onClose: () => void;
  sessionId: string;
}

export const DiagnosticsModal: React.FC<DiagnosticsModalProps> = ({
  isOpen,
  onClose,
  sessionId,
}) => {
  const [isDownloading, setIsDownloading] = useState(false);

  const handleDownload = async () => {
    setIsDownloading(true);

    try {
      const response = await diagnostics({
        path: { session_id: sessionId },
        throwOnError: true,
      });

      const blob = new Blob([response.data], { type: 'application/zip' });
      const url = window.URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `diagnostics_${sessionId}.zip`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      window.URL.revokeObjectURL(url);

      onClose();
    } catch {
      toastError({
        title: 'Diagnostics Error',
        msg: 'Failed to download diagnostics',
      });
    } finally {
      setIsDownloading(false);
    }
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
      <div className="bg-background-default border border-borderStandard rounded-lg p-6 max-w-md mx-4">
        <div className="flex items-start gap-3 mb-4">
          <AlertTriangle className="text-orange-500 flex-shrink-0 mt-1" size={20} />
          <div>
            <h3 className="text-lg font-semibold text-textStandard mb-2">Download Diagnostics</h3>
            <p className="text-sm text-textSubtle mb-3">
              Hit the download button to get a zip file containing all the important information to
              diagnose a problem in goose. You can share this file with the team or if you are a
              developer look at it yourself.
            </p>
            <ul className="text-sm text-textSubtle list-disc list-inside space-y-1 mb-3">
              <li>Basic system info</li>
              <li>Your current session messages</li>
              <li>Recent log files</li>
              <li>Configuration settings</li>
            </ul>
            <p className="text-sm text-textSubtle">
              <strong>Warning:</strong> If your session contains sensitive information, do not share
              this file publicly.
            </p>
          </div>
        </div>
        <div className="flex gap-2 justify-end">
          <Button onClick={onClose} variant="outline" size="sm" disabled={isDownloading}>
            Cancel
          </Button>
          <Button
            onClick={handleDownload}
            variant="outline"
            size="sm"
            disabled={isDownloading}
            className="bg-slate-600 text-white hover:bg-slate-700"
          >
            {isDownloading ? 'Downloading...' : 'Download'}
          </Button>
        </div>
      </div>
    </div>
  );
};
