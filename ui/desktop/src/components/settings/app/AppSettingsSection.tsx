import { useState, useEffect, useRef } from 'react';
import { Switch } from '../../ui/switch';
import { Button } from '../../ui/button';
import { Settings, RefreshCw, ExternalLink } from 'lucide-react';
import Modal from '../../Modal';
import UpdateSection from './UpdateSection';
import { COST_TRACKING_ENABLED, UPDATES_ENABLED } from '../../../updates';
import { getApiUrl, getSecretKey } from '../../../config';

interface AppSettingsSectionProps {
  scrollToSection?: string;
}

export default function AppSettingsSection({ scrollToSection }: AppSettingsSectionProps) {
  const [menuBarIconEnabled, setMenuBarIconEnabled] = useState(true);
  const [dockIconEnabled, setDockIconEnabled] = useState(true);
  const [quitConfirmationEnabled, setQuitConfirmationEnabled] = useState(true);
  const [isMacOS, setIsMacOS] = useState(false);
  const [isDockSwitchDisabled, setIsDockSwitchDisabled] = useState(false);
  const [showNotificationModal, setShowNotificationModal] = useState(false);
  const [pricingStatus, setPricingStatus] = useState<'loading' | 'success' | 'error'>('loading');
  const [lastFetchTime, setLastFetchTime] = useState<Date | null>(null);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [showPricing, setShowPricing] = useState(true);
  const updateSectionRef = useRef<HTMLDivElement>(null);

  // Check if running on macOS
  useEffect(() => {
    setIsMacOS(window.electron.platform === 'darwin');
  }, []);

  // Load show pricing setting
  useEffect(() => {
    const stored = localStorage.getItem('show_pricing');
    setShowPricing(stored !== 'false');
  }, []);

  // Check pricing status on mount
  useEffect(() => {
    checkPricingStatus();
  }, []);

  const checkPricingStatus = async () => {
    try {
      const apiUrl = getApiUrl('/config/pricing');
      const secretKey = getSecretKey();

      const headers: HeadersInit = { 'Content-Type': 'application/json' };
      if (secretKey) {
        headers['X-Secret-Key'] = secretKey;
      }

      const response = await fetch(apiUrl, {
        method: 'POST',
        headers,
        body: JSON.stringify({ configured_only: true }),
      });

      if (response.ok) {
        await response.json(); // Consume the response
        setPricingStatus('success');
        setLastFetchTime(new Date());
      } else {
        setPricingStatus('error');
      }
    } catch (error) {
      setPricingStatus('error');
    }
  };

  const handleRefreshPricing = async () => {
    setIsRefreshing(true);
    try {
      const apiUrl = getApiUrl('/config/pricing');
      const secretKey = getSecretKey();

      const headers: HeadersInit = { 'Content-Type': 'application/json' };
      if (secretKey) {
        headers['X-Secret-Key'] = secretKey;
      }

      const response = await fetch(apiUrl, {
        method: 'POST',
        headers,
        body: JSON.stringify({ configured_only: false }),
      });

      if (response.ok) {
        setPricingStatus('success');
        setLastFetchTime(new Date());
        // Trigger a reload of the cost database
        window.dispatchEvent(new CustomEvent('pricing-updated'));
      } else {
        setPricingStatus('error');
      }
    } catch (error) {
      setPricingStatus('error');
    } finally {
      setIsRefreshing(false);
    }
  };

  // Handle scrolling to update section
  useEffect(() => {
    if (scrollToSection === 'update' && updateSectionRef.current) {
      // Use a timeout to ensure the DOM is ready
      setTimeout(() => {
        updateSectionRef.current?.scrollIntoView({ behavior: 'smooth', block: 'center' });
      }, 100);
    }
  }, [scrollToSection]);

  // Load menu bar and dock icon states
  useEffect(() => {
    window.electron.getMenuBarIconState().then((enabled) => {
      setMenuBarIconEnabled(enabled);
    });

    window.electron.getQuitConfirmationState().then((enabled) => {
      setQuitConfirmationEnabled(enabled);
    });

    if (isMacOS) {
      window.electron.getDockIconState().then((enabled) => {
        setDockIconEnabled(enabled);
      });
    }
  }, [isMacOS]);

  const handleMenuBarIconToggle = async () => {
    const newState = !menuBarIconEnabled;
    // If we're turning off the menu bar icon and the dock icon is hidden,
    // we need to show the dock icon to maintain accessibility
    if (!newState && !dockIconEnabled && isMacOS) {
      const success = await window.electron.setDockIcon(true);
      if (success) {
        setDockIconEnabled(true);
      }
    }
    const success = await window.electron.setMenuBarIcon(newState);
    if (success) {
      setMenuBarIconEnabled(newState);
    }
  };

  const handleDockIconToggle = async () => {
    const newState = !dockIconEnabled;
    // If we're turning off the dock icon and the menu bar icon is hidden,
    // we need to show the menu bar icon to maintain accessibility
    if (!newState && !menuBarIconEnabled) {
      const success = await window.electron.setMenuBarIcon(true);
      if (success) {
        setMenuBarIconEnabled(true);
      }
    }

    // Disable the switch to prevent rapid toggling
    setIsDockSwitchDisabled(true);
    setTimeout(() => {
      setIsDockSwitchDisabled(false);
    }, 1000);

    // Set the dock icon state
    const success = await window.electron.setDockIcon(newState);
    if (success) {
      setDockIconEnabled(newState);
    }
  };

  const handleQuitConfirmationToggle = async () => {
    const newState = !quitConfirmationEnabled;
    const success = await window.electron.setQuitConfirmation(newState);
    if (success) {
      setQuitConfirmationEnabled(newState);
    }
  };

  const handleShowPricingToggle = (checked: boolean) => {
    setShowPricing(checked);
    localStorage.setItem('show_pricing', String(checked));
    // Trigger storage event for other components
    window.dispatchEvent(new CustomEvent('storage'));
  };

  return (
    <section id="appSettings" className="px-8">
      <div className="flex justify-between items-center mb-2">
        <h2 className="text-xl font-medium text-textStandard">App Settings</h2>
      </div>
      <div className="pb-8">
        <p className="text-sm text-textStandard mb-6">Configure Goose app</p>
        <div>
          {/* Task Notifications */}
          <div className="flex items-center justify-between mb-4">
            <div>
              <h3 className="text-textStandard">Notifications</h3>
              <p className="text-xs text-textSubtle max-w-md mt-[2px]">
                Notifications are managed by your OS{' - '}
                <span
                  className="underline hover:cursor-pointer"
                  onClick={() => setShowNotificationModal(true)}
                >
                  Configuration guide
                </span>
              </p>
            </div>
            <div className="flex items-center">
              <Button
                className="flex items-center gap-2 justify-center text-textStandard bg-bgApp border border-borderSubtle hover:border-borderProminent hover:bg-bgApp [&>svg]:!size-4"
                onClick={async () => {
                  try {
                    await window.electron.openNotificationsSettings();
                  } catch (error) {
                    console.error('Failed to open notification settings:', error);
                  }
                }}
              >
                <Settings />
                Open Settings
              </Button>
            </div>
          </div>

          {/* Menu Bar */}
          <div className="flex items-center justify-between mb-4">
            <div>
              <h3 className="text-textStandard">Menu Bar Icon</h3>
              <p className="text-xs text-textSubtle max-w-md mt-[2px]">
                Show Goose in the menu bar
              </p>
            </div>
            <div className="flex items-center">
              <Switch
                checked={menuBarIconEnabled}
                onCheckedChange={handleMenuBarIconToggle}
                variant="mono"
              />
            </div>
          </div>

          {/* Dock Icon */}
          {isMacOS && (
            <div className="flex items-center justify-between mb-4">
              <div>
                <h3 className="text-textStandard">Dock Icon</h3>
                <p className="text-xs text-textSubtle max-w-md mt-[2px]">Show Goose in the dock</p>
              </div>
              <div className="flex items-center">
                <Switch
                  disabled={isDockSwitchDisabled}
                  checked={dockIconEnabled}
                  onCheckedChange={handleDockIconToggle}
                  variant="mono"
                />
              </div>
            </div>
          )}

          {/* Quit Confirmation */}
          <div className="flex items-center justify-between mb-4">
            <div>
              <h3 className="text-textStandard">Quit Confirmation</h3>
              <p className="text-xs text-textSubtle max-w-md mt-[2px]">
                Show confirmation dialog when quitting the app
              </p>
            </div>
            <div className="flex items-center">
              <Switch
                checked={quitConfirmationEnabled}
                onCheckedChange={handleQuitConfirmationToggle}
                variant="mono"
              />
            </div>
          </div>

          {/* Cost Tracking */}
          {COST_TRACKING_ENABLED && (
            <div className="flex items-center justify-between mb-4">
              <div>
                <h3 className="text-textStandard">Cost Tracking</h3>
                <p className="text-xs text-textSubtle max-w-md mt-[2px]">
                  Show model pricing and usage costs
                </p>
              </div>
              <div className="flex items-center">
                <Switch
                  checked={showPricing}
                  onCheckedChange={handleShowPricingToggle}
                  variant="mono"
                />
              </div>
            </div>
          )}

          {/* Pricing Status - only show if cost tracking is enabled */}
          {COST_TRACKING_ENABLED && showPricing && (
            <>
              <div className="flex items-center justify-between text-xs mb-2 px-4">
                <span className="text-textSubtle">Pricing Source:</span>
                <a
                  href="https://openrouter.ai/docs#models"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="text-blue-600 dark:text-blue-400 hover:underline flex items-center gap-1"
                >
                  OpenRouter Docs
                  <ExternalLink size={10} />
                </a>
              </div>

              <div className="flex items-center justify-between text-xs mb-2 px-4">
                <span className="text-textSubtle">Status:</span>
                <div className="flex items-center gap-2">
                  <span
                    className={`font-medium ${
                      pricingStatus === 'success'
                        ? 'text-green-600 dark:text-green-400'
                        : pricingStatus === 'error'
                          ? 'text-red-600 dark:text-red-400'
                          : 'text-textSubtle'
                    }`}
                  >
                    {pricingStatus === 'success'
                      ? '✓ Connected'
                      : pricingStatus === 'error'
                        ? '✗ Failed'
                        : '... Checking'}
                  </span>
                  <button
                    className="p-0.5 hover:bg-gray-200 dark:hover:bg-gray-700 rounded transition-colors disabled:opacity-50"
                    onClick={handleRefreshPricing}
                    disabled={isRefreshing}
                    title="Refresh pricing data"
                    type="button"
                  >
                    <RefreshCw
                      size={8}
                      className={`text-textSubtle hover:text-textStandard ${isRefreshing ? 'animate-spin-fast' : ''}`}
                    />
                  </button>
                </div>
              </div>

              {lastFetchTime && (
                <div className="flex items-center justify-between text-xs mb-2 px-4">
                  <span className="text-textSubtle">Last updated:</span>
                  <span className="text-textSubtle">{lastFetchTime.toLocaleTimeString()}</span>
                </div>
              )}

              {pricingStatus === 'error' && (
                <p className="text-xs text-red-600 dark:text-red-400 px-4">
                  Unable to fetch pricing data. Costs will not be displayed.
                </p>
              )}
            </>
          )}
        </div>

        {/* Help & Feedback Section */}
        <div className="mt-8 pt-8 border-t border-gray-200 dark:border-gray-700">
          <h3 className="text-lg font-medium text-textStandard mb-1">Help & Feedback</h3>
          <p className="text-sm text-textSubtle mb-4">
            Help us improve Goose by reporting issues or requesting new features.
          </p>
          <div className="flex space-x-4">
            <a
              href="https://github.com/block/goose/issues/new?template=bug_report.md"
              target="_blank"
              rel="noopener noreferrer"
              className="text-sm text-blue-600 dark:text-blue-400 hover:underline"
            >
              Report a Bug
            </a>
            <a
              href="https://github.com/block/goose/issues/new?template=feature_request.md"
              target="_blank"
              rel="noopener noreferrer"
              className="text-sm text-blue-600 dark:text-blue-400 hover:underline"
            >
              Request a Feature
            </a>
          </div>
        </div>

        {/* Update Section */}
        {UPDATES_ENABLED && (
          <div ref={updateSectionRef} className="mt-8 pt-8 border-t border-gray-200">
            <UpdateSection />
          </div>
        )}
      </div>

      {/* Notification Instructions Modal */}
      {showNotificationModal && (
        <Modal
          onClose={() => setShowNotificationModal(false)}
          footer={
            <Button
              onClick={() => setShowNotificationModal(false)}
              variant="ghost"
              className="w-full h-[60px] rounded-none hover:bg-bgSubtle text-textSubtle hover:text-textStandard text-md font-regular"
            >
              Close
            </Button>
          }
        >
          {/* Title and Icon */}
          <div className="flex flex-col mb-6">
            <div>
              <Settings className="text-iconStandard" size={24} />
            </div>
            <div className="mt-2">
              <h2 className="text-2xl font-regular text-textStandard">
                How to Enable Notifications
              </h2>
            </div>
          </div>

          {/* Content */}
          <div>
            {isMacOS ? (
              <div className="space-y-4">
                <p className="text-textStandard">To enable notifications for Goose on macOS:</p>
                <ol className="list-decimal list-inside space-y-3 text-textStandard ml-4">
                  <li>Click the "Open Settings" button</li>
                  <li>Find "Goose" in the list of applications</li>
                  <li>Click on "Goose" to open its notification settings</li>
                  <li>Toggle "Allow Notifications" to ON</li>
                  <li>Choose your preferred notification style</li>
                </ol>
              </div>
            ) : window.electron.platform === 'win32' ? (
              <div className="space-y-4">
                <p className="text-textStandard">To enable notifications for Goose on Windows:</p>
                <ol className="list-decimal list-inside space-y-3 text-textStandard ml-4">
                  <li>Click the "Open Settings" button</li>
                  <li>
                    In the Notifications & actions settings, scroll down to "Get notifications from
                    these senders"
                  </li>
                  <li>Find "Goose" in the list of applications</li>
                  <li>Click on "Goose" to expand its notification settings</li>
                  <li>Toggle the main switch to ON to enable notifications</li>
                  <li>Customize notification banners, sounds, and other preferences as desired</li>
                </ol>
              </div>
            ) : (
              <div className="space-y-4">
                <p className="text-textStandard">To enable notifications for Goose on Linux:</p>
                <ol className="list-decimal list-inside space-y-3 text-textStandard ml-4">
                  <li>Click the "Open Settings" button</li>
                  <li>
                    In the notification settings panel, look for application-specific settings
                  </li>
                  <li>Find "Goose" or "Electron" in the list of applications</li>
                  <li>Enable notifications for the application</li>
                  <li>Configure notification preferences such as sound and display options</li>
                </ol>
                <div className="mt-4 p-3 bg-bgSubtle rounded-md">
                  <p className="text-sm text-textSubtle">
                    <strong>Note:</strong> The exact steps may vary depending on your desktop
                    environment (GNOME, KDE, XFCE, etc.). If the "Open Settings" button doesn't
                    work, you can manually access notification settings through your system's
                    settings application.
                  </p>
                </div>
              </div>
            )}
          </div>
        </Modal>
      )}
    </section>
  );
}
