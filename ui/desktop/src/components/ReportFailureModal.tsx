import { useState, useEffect, useCallback } from 'react';
import { Button } from './ui/button';
import { Textarea } from './ui/textarea';
import { Card, CardContent } from './ui/card';
import { AlertCircle, CheckCircle, ExternalLink, Loader2 } from 'lucide-react';
import Modal from './Modal';
import { getApiUrl, getSecretKey } from '../config';

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
  const [recentErrors, setRecentErrors] = useState<string[]>([]);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [submitStatus, setSubmitStatus] = useState<'idle' | 'success' | 'error'>('idle');
  const [issueUrl, setIssueUrl] = useState('');

  const getExtensionCount = async (): Promise<number> => {
    try {
      // Get extension count from the API
      const apiUrl = getApiUrl('/extension/list');
      const secretKey = getSecretKey();

      const headers: HeadersInit = {
        'Content-Type': 'application/json',
      };

      if (secretKey) {
        headers['X-Secret-Key'] = secretKey;
      }

      const response = await fetch(apiUrl, {
        method: 'GET',
        headers,
      });

      if (response.ok) {
        const extensions = await response.json();
        return Array.isArray(extensions) ? extensions.length : 0;
      }
    } catch (error) {
      console.error('Failed to get extension count:', error);
    }
    return 0;
  };

  const collectSystemInfo = useCallback(async () => {
    try {
      const secretKey = getSecretKey();
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

      // Try to get current provider type (without sensitive data)
      try {
        const providerApiUrl = getApiUrl('/agent/providers');
        const providerResponse = await fetch(providerApiUrl, {
          method: 'GET',
          headers: {
            'Content-Type': 'application/json',
            ...(secretKey && { 'X-Secret-Key': secretKey }),
          },
        });

        if (providerResponse.ok) {
          const providers = await providerResponse.json();
          // Get the first configured provider name (without sensitive details)
          if (Array.isArray(providers) && providers.length > 0) {
            info.providerType = providers[0].name || 'Unknown Provider';
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

  const collectRecentErrors = useCallback(async () => {
    try {
      // For now, collect from browser console errors and any stored logs
      const recentErrors: string[] = [];

      const storedErrors = localStorage.getItem('goose_recent_errors');
      if (storedErrors) {
        try {
          const parsed = JSON.parse(storedErrors);
          if (Array.isArray(parsed)) {
            recentErrors.push(...parsed.slice(-10));
          }
        } catch (parseError) {
          console.debug('Failed to parse stored errors:', parseError);
        }
      }

      if (recentErrors.length === 0) {
        recentErrors.push('Recent errors are logged to the main process via ErrorBoundary');
      }

      setRecentErrors(recentErrors);
    } catch (error) {
      console.error('Failed to collect recent errors:', error);
      setRecentErrors(['Failed to collect error logs - but this error is now captured!']);
    }
  }, []);

  // Collect system information when modal opens
  useEffect(() => {
    if (isOpen) {
      collectSystemInfo();
      collectRecentErrors();
      // Reset form state
      setDescription('');
      setSubmitStatus('idle');
      setIssueUrl('');
    }
  }, [isOpen, collectSystemInfo, collectRecentErrors]);

  const handleSubmit = async () => {
    if (!description.trim()) {
      window.alert('Please describe what happened before submitting.');
      return;
    }

    setIsSubmitting(true);
    setSubmitStatus('idle');

    try {
      // Prepare the issue data
      const issueData = {
        title: `[FAILURE REPORT] ${description.substring(0, 60)}${description.length > 60 ? '...' : ''}`,
        description,
        systemInfo,
        recentErrors,
        timestamp: new Date().toISOString(),
      };

      const apiUrl = getApiUrl('/report-failure');
      const secretKey = getSecretKey();

      const headers: HeadersInit = {
        'Content-Type': 'application/json',
      };

      if (secretKey) {
        headers['X-Secret-Key'] = secretKey;
      }

      const response = await fetch(apiUrl, {
        method: 'POST',
        headers,
        body: JSON.stringify(issueData),
      });

      if (response.ok) {
        const result = await response.json();
        setIssueUrl(result.issueUrl);
        setSubmitStatus('success');
      } else {
        setSubmitStatus('error');
      }
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
                Thank you for helping us improve Goose. Your report has been submitted.
              </p>
              {issueUrl && (
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => window.open(issueUrl, '_blank')}
                  className="flex items-center gap-2"
                >
                  View Issue <ExternalLink size={14} />
                </Button>
              )}
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
