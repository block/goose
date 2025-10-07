import { useState, useEffect, useCallback } from 'react';
import { Button } from './ui/button';
import { Textarea } from './ui/textarea';
import { Card, CardContent } from './ui/card';
import { AlertCircle, CheckCircle, Loader2 } from 'lucide-react';
import Modal from './Modal';
import { getExtensions, getSessionHistory } from '../api/sdk.gen';

interface SystemInfo {
  gooseVersion: string;
  osVersion: string;
  platform: string;
  architecture: string;
  providerType?: string;
  extensionCount: number;
}

interface ReportFailureModalProps {
  isOpen: boolean;
  onClose: () => void;
}

export default function ReportFailureModal({ isOpen, onClose }: ReportFailureModalProps) {
  const [description, setDescription] = useState('');
  const [systemInfo, setSystemInfo] = useState<SystemInfo | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [submitStatus, setSubmitStatus] = useState<'idle' | 'success' | 'error'>('idle');

  const getExtensionCount = async (): Promise<number> => {
    try {
      const response = await getExtensions({});

      if (response.data && Array.isArray(response.data)) {
        return response.data.length;
      }
    } catch (error) {
      console.error('Failed to get extension count:', error);
    }
    return 0;
  };

  const collectSystemInfo = useCallback(async () => {
    try {
      const info: SystemInfo = {
        gooseVersion: String(
          window.appConfig?.get('GOOSE_VERSION') ||
            window.appConfig?.get('version') ||
            'Development'
        ),
        osVersion: navigator.userAgent,
        platform: window.electron?.platform || 'Unknown',
        architecture: window.electron?.arch || 'Unknown',
        extensionCount: await getExtensionCount(),
      };

      try {
        const activeSessionId: string | undefined = window.appConfig?.get('CURRENT_SESSION_ID') as
          | string
          | undefined;

        if (activeSessionId) {
          const sessionResponse = await getSessionHistory({
            path: { session_id: activeSessionId },
          });

          const sessionData = sessionResponse.data as { metadata?: { provider_name?: string } };

          if (sessionData?.metadata?.provider_name) {
            info.providerType = sessionData.metadata.provider_name;
          } else {
            info.providerType = 'Unknown Provider (Session Metadata Incomplete)';
          }
        }
      } catch (error) {
        console.debug('Provider detection failed:', error);
      }

      setSystemInfo(info);
    } catch (error) {
      console.error('Failed to collect system info:', error);
    }
  }, []);

  // Collect system information when modal opens
  useEffect(() => {
    if (isOpen) {
      collectSystemInfo();
      // Reset form state
      setDescription('');
      setSubmitStatus('idle');
    }
  }, [isOpen, collectSystemInfo]);

  const formatGitHubIssueBody = () => {
    if (!systemInfo) return '';

    return `**Describe the bug**

${description}

**Please provide following information:**
- **OS & Arch:** ${systemInfo.platform} ${systemInfo.architecture}
- **Interface:** UI (Desktop App)
- **Version:** ${systemInfo.gooseVersion}
- **Provider & Model:** ${systemInfo.providerType || 'Unknown'}
- **Extensions:** ${systemInfo.extensionCount} installed


**Additional context**
- **Timestamp:** ${new Date().toISOString()}
- **Reported via:** Goose Desktop App automated failure reporting

---
*This issue was created via the "Report a Failure" feature.*`;
  };

  const handleSubmit = async () => {
    if (!description.trim()) {
      window.alert('Please describe what happened before submitting.');
      return;
    }

    setIsSubmitting(true);
    setSubmitStatus('idle');

    try {
      const issueTitle = encodeURIComponent(
        `[FAILURE REPORT] ${description.substring(0, 60)}${description.length > 60 ? '...' : ''}`
      );
      const issueBody = encodeURIComponent(formatGitHubIssueBody());
      const labels = encodeURIComponent('bug,needs-triage,failure-report');

      const githubUrl = `https://github.com/block/goose/issues/new?title=${issueTitle}&body=${issueBody}&labels=${labels}`;

      window.electron?.openUrl?.(githubUrl) || window.open(githubUrl, '_blank');

      setSubmitStatus('success');
    } catch (error) {
      console.error('Failed to submit failure report:', error);
      setSubmitStatus('error');
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleClose = () => {
    if (!isSubmitting) {
      onClose();
    }
  };

  if (!isOpen) return null;

  return (
    <Modal onClose={handleClose} preventBackdropClose={isSubmitting}>
      <div className="space-y-6">
        <div>
          <h2 className="text-2xl font-regular mb-2">Report a Failure</h2>
          <p className="text-sm text-text-muted">
            Help us improve Goose by reporting what went wrong. Your feedback helps our team
            diagnose and fix issues.
          </p>
        </div>

        {submitStatus === 'success' ? (
          <div className="text-center space-y-4">
            <CheckCircle className="mx-auto text-green-500" size={48} />
            <div>
              <h3 className="text-lg font-medium mb-2">Report Submitted Successfully!</h3>
              <p className="text-sm text-text-muted mb-4">
                Your report has been logged and GitHub has opened in your browser to create an
                issue. Please complete the issue creation to help us improve Goose!
              </p>
            </div>
          </div>
        ) : (
          <>
            <div>
              <label htmlFor="description" className="block text-sm font-medium mb-2">
                What happened? <span className="text-red-500">*</span>
              </label>
              <Textarea
                id="description"
                value={description}
                onChange={(e) => setDescription(e.target.value)}
                placeholder="Please describe what you were trying to do and what went wrong. Include any error messages you saw and steps to reproduce the issue..."
                className="min-h-[120px] resize-none"
                disabled={isSubmitting}
              />
            </div>

            <div>
              <h3 className="text-sm font-medium mb-2">System Information</h3>
              <Card>
                <CardContent className="pt-4">
                  {systemInfo ? (
                    <div className="space-y-2 text-xs">
                      <div className="flex justify-between">
                        <span className="text-text-muted">Goose Version:</span>
                        <span className="font-mono">{systemInfo.gooseVersion}</span>
                      </div>
                      <div className="flex justify-between">
                        <span className="text-text-muted">Platform:</span>
                        <span className="font-mono">{systemInfo.platform}</span>
                      </div>
                      <div className="flex justify-between">
                        <span className="text-text-muted">Architecture:</span>
                        <span className="font-mono">{systemInfo.architecture}</span>
                      </div>
                      {/* Only display the provider field if the value is set */}
                      {systemInfo.providerType && (
                        <div className="flex justify-between">
                          <span className="text-text-muted">Provider:</span>
                          <span className="font-mono">{systemInfo.providerType}</span>
                        </div>
                      )}
                    </div>
                  ) : (
                    <div className="flex items-center gap-2 text-sm text-text-muted">
                      <Loader2 className="animate-spin" size={16} />
                      Collecting system information...
                    </div>
                  )}
                </CardContent>
              </Card>
            </div>

            {submitStatus === 'error' && (
              <div className="flex items-center gap-2 p-3 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg">
                <AlertCircle className="text-red-500 flex-shrink-0" size={16} />
                <p className="text-sm text-red-700 dark:text-red-300">
                  Failed to submit report. Please try again or{' '}
                  <button
                    className="underline hover:no-underline"
                    onClick={() =>
                      window.open(
                        'https://github.com/block/goose/issues/new?template=bug_report.md',
                        '_blank'
                      )
                    }
                  >
                    report manually on GitHub
                  </button>
                  .
                </p>
              </div>
            )}
          </>
        )}

        <div className="flex justify-end gap-3 pt-4 border-t border-border-subtle">
          <Button variant="outline" onClick={handleClose} disabled={isSubmitting}>
            {submitStatus === 'success' ? 'Close' : 'Cancel'}
          </Button>
          {submitStatus !== 'success' && (
            <Button
              onClick={handleSubmit}
              disabled={isSubmitting || !description.trim()}
              className="flex items-center gap-2"
            >
              {isSubmitting && <Loader2 className="animate-spin" size={16} />}
              {isSubmitting ? 'Submitting...' : 'Submit Report'}
            </Button>
          )}
        </div>
      </div>
    </Modal>
  );
}
