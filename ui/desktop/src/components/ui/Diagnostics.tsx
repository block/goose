import React, { useState } from 'react';
import { AlertTriangle, Download, Github } from 'lucide-react';
import { Button } from './button';
import { toastError } from '../../toasts';
import { diagnostics, systemInfo } from '../../api';

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
  const [isFilingBug, setIsFilingBug] = useState(false);

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

  const handleFileGitHubIssue = async () => {
    setIsFilingBug(true);

    try {
      const response = await systemInfo({ throwOnError: true });
      const info = response.data;

      const providerModel =
        info.provider && info.model
          ? `${info.provider} – ${info.model}`
          : info.provider || info.model || '[e.g. Google – gemini-1.5-pro]';

      const extensions =
        info.enabled_extensions.length > 0
          ? info.enabled_extensions.join(', ')
          : '[e.g. Computer Controller, Figma]';

      const body = `**Describe the bug**

💡 Before filing, please check common issues:  
https://block.github.io/goose/docs/troubleshooting  

📦 To help us debug faster, attach your **diagnostics zip** if possible.  
👉 How to capture it: https://block.github.io/goose/docs/troubleshooting/diagnostics-and-reporting/

A clear and concise description of what the bug is.

---

**To Reproduce**
Steps to reproduce the behavior:
1. Go to '...'
2. Click on '....'
3. Scroll down to '....'
4. See error

---

**Expected behavior**
A clear and concise description of what you expected to happen.

---

**Screenshots**
If applicable, add screenshots to help explain your problem.

---

**Please provide the following information**
- **OS & Arch:** ${info.os} ${info.os_version} ${info.architecture}
- **Interface:** UI
- **Version:** ${info.app_version}
- **Extensions enabled:** ${extensions}
- **Provider & Model:** ${providerModel}

---

**Additional context**
Add any other context about the problem here.
`;

      const params = new URLSearchParams({
        template: 'bug_report.md',
        body: body,
        labels: 'bug',
      });

      window.open(`https://github.com/block/goose/issues/new?${params.toString()}`, '_blank');
      onClose();
    } catch {
      toastError({
        title: 'Error',
        msg: 'Failed to get system information',
      });
    } finally {
      setIsFilingBug(false);
    }
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
      <div className="bg-background-default border border-border-default rounded-lg p-6 max-w-md mx-4">
        <div className="flex items-start gap-3 mb-4">
          <AlertTriangle className="text-orange-500 flex-shrink-0 mt-1" size={20} />
          <div>
            <h3 className="text-lg font-semibold text-text-default mb-2">Report a Problem</h3>
            <p className="text-sm text-text-muted mb-3">
              You can download a diagnostics zip file to share with the team, or file a bug directly
              on GitHub with your system details pre-filled. A diagnostics report contains the
              following:
            </p>
            <ul className="text-sm text-text-muted list-disc list-inside space-y-1 mb-3">
              <li>Basic system info</li>
              <li>Your current session messages</li>
              <li>Recent log files</li>
              <li>Configuration settings</li>
            </ul>
            <p className="text-sm text-text-muted">
              <strong>Warning:</strong> If your session contains sensitive information, do not share
              the diagnostics file publicly.
            </p>
            <p className="text-sm text-text-muted">
              If you file a bug, consider attaching the diagnostics report to it.
            </p>
          </div>
        </div>
        <div className="flex gap-2 justify-end">
          <Button
            onClick={onClose}
            variant="outline"
            size="sm"
            disabled={isDownloading || isFilingBug}
          >
            Cancel
          </Button>
          <Button
            onClick={handleDownload}
            variant="outline"
            size="sm"
            disabled={isDownloading || isFilingBug}
          >
            <Download size={16} className="mr-1" />
            {isDownloading ? 'Downloading...' : 'Download'}
          </Button>
          <Button
            onClick={handleFileGitHubIssue}
            variant="outline"
            size="sm"
            disabled={isDownloading || isFilingBug}
            className="bg-slate-600 text-white hover:bg-slate-700"
          >
            <Github size={16} className="mr-1" />
            {isFilingBug ? 'Opening...' : 'File Bug on GitHub'}
          </Button>
        </div>
      </div>
    </div>
  );
};
