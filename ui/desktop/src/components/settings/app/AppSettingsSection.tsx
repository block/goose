import { useState, useEffect, useRef } from 'react';
import { Switch } from '../../ui/switch';
import { Button } from '../../ui/button';
import { Settings, RefreshCw, ExternalLink } from 'lucide-react';
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from '../../ui/dialog';
import UpdateSection from './UpdateSection';
import { COST_TRACKING_ENABLED, UPDATES_ENABLED } from '../../../updates';
import { getApiUrl } from '../../../config';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../../ui/card';
import ThemeSelector from '../../GooseSidebar/ThemeSelector';
import BlockLogoBlack from './icons/block-lockup_black.png';
import BlockLogoWhite from './icons/block-lockup_white.png';

interface AppSettingsSectionProps {
  scrollToSection?: string;
}

export default function AppSettingsSection({ scrollToSection }: AppSettingsSectionProps) {
  const [menuBarIconEnabled, setMenuBarIconEnabled] = useState(true);
  const [dockIconEnabled, setDockIconEnabled] = useState(true);
  const [quitConfirmationEnabled, setQuitConfirmationEnabled] = useState(true);
  const [wakelockEnabled, setWakelockEnabled] = useState(true);
  const [isMacOS, setIsMacOS] = useState(false);
  const [isDockSwitchDisabled, setIsDockSwitchDisabled] = useState(false);
  const [showNotificationModal, setShowNotificationModal] = useState(false);
  const [pricingStatus, setPricingStatus] = useState<'loading' | 'success' | 'error'>('loading');
  const [lastFetchTime, setLastFetchTime] = useState<Date | null>(null);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [showPricing, setShowPricing] = useState(true);
  const [isDarkMode, setIsDarkMode] = useState(false);
  const updateSectionRef = useRef<HTMLDivElement>(null);

  // Check if GOOSE_VERSION is set to determine if Updates section should be shown
  const shouldShowUpdates = !window.appConfig.get('GOOSE_VERSION');

  // Check if running on macOS
  useEffect(() => {
    setIsMacOS(window.electron.platform === 'darwin');
  }, []);

  // Detect theme changes
  useEffect(() => {
    const updateTheme = () => {
      setIsDarkMode(document.documentElement.classList.contains('dark'));
    };

    // Initial check
    updateTheme();

    // Listen for theme changes
    const observer = new MutationObserver(updateTheme);
    observer.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ['class'],
    });

    return () => observer.disconnect();
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
      const secretKey = await window.electron.getSecretKey();

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
        await response.json();
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
      const secretKey = await window.electron.getSecretKey();

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

    window.electron.getWakelockState().then((enabled) => {
      setWakelockEnabled(enabled);
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

  const handleWakelockToggle = async () => {
    const newState = !wakelockEnabled;
    const success = await window.electron.setWakelock(newState);
    if (success) {
      setWakelockEnabled(newState);
    }
  };

  const handleShowPricingToggle = (checked: boolean) => {
    setShowPricing(checked);
    localStorage.setItem('show_pricing', String(checked));
    // Trigger storage event for other components
    window.dispatchEvent(new CustomEvent('storage'));
  };

  return (
    <div className="space-y-4 pr-4 pb-8 mt-1">
      <Card className="rounded-lg">
        <CardHeader className="pb-0">
          <CardTitle className="">Appearance</CardTitle>
          <CardDescription>Configure how goose appears on your system</CardDescription>
        </CardHeader>
        <CardContent className="pt-4 space-y-4 px-4">
          <div className="flex items-center justify-between">
            <div>
              <h3 className="text-text-default text-xs">Notifications</h3>
              <p className="text-xs text-text-muted max-w-md mt-[2px]">
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
                className="flex items-center gap-2 justify-center"
                variant="secondary"
                size="sm"
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

          <div className="flex items-center justify-between">
            <div>
              <h3 className="text-text-default text-xs">Menu bar icon</h3>
              <p className="text-xs text-text-muted max-w-md mt-[2px]">
                Show goose in the menu bar
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

          {isMacOS && (
            <div className="flex items-center justify-between">
              <div>
                <h3 className="text-text-default text-xs">Dock icon</h3>
                <p className="text-xs text-text-muted max-w-md mt-[2px]">Show goose in the dock</p>
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
          <div className="flex items-center justify-between">
            <div>
              <h3 className="text-text-default text-xs">Quit confirmation</h3>
              <p className="text-xs text-text-muted max-w-md mt-[2px]">
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

          {/* Prevent Sleep */}
          <div className="flex items-center justify-between">
            <div>
              <h3 className="text-text-default text-xs">Prevent Sleep</h3>
              <p className="text-xs text-text-muted max-w-md mt-[2px]">
                Keep your computer awake while Goose is running a task (screen can still lock)
              </p>
            </div>
            <div className="flex items-center">
              <Switch
                checked={wakelockEnabled}
                onCheckedChange={handleWakelockToggle}
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
        </CardContent>
      </Card>

      <Card className="rounded-lg">
        <CardHeader className="pb-0">
          <CardTitle className="mb-1">Theme</CardTitle>
          <CardDescription>Customize the look and feel of goose</CardDescription>
        </CardHeader>
        <CardContent className="pt-4 px-4">
          <ThemeSelector className="w-auto" hideTitle horizontal />
        </CardContent>
      </Card>

      <Card className="rounded-lg">
        <CardHeader className="pb-0">
          <CardTitle className="mb-1">Help & feedback</CardTitle>
          <CardDescription>
            Help us improve goose by reporting issues or requesting new features
          </CardDescription>
        </CardHeader>
        <CardContent className="pt-4 px-4">
          <div className="flex space-x-4">
            <Button
              onClick={() => {
                window.open(
                  'https://github.com/block/goose/issues/new?template=bug_report.md',
                  '_blank'
                );
              }}
              variant="secondary"
              size="sm"
            >
              Report a Bug
            </Button>
            <Button
              onClick={() => {
                window.open(
                  'https://github.com/block/goose/issues/new?template=feature_request.md',
                  '_blank'
                );
              }}
              variant="secondary"
              size="sm"
            >
              Request a Feature
            </Button>
          </div>
        </CardContent>
      </Card>

      {/* Version Section - only show if GOOSE_VERSION is set */}
      {!shouldShowUpdates && (
        <Card className="rounded-lg">
          <CardHeader className="pb-0">
            <CardTitle className="mb-1">Version</CardTitle>
          </CardHeader>
          <CardContent className="pt-4 px-4">
            <div className="flex items-center gap-3">
              <img
                src={isDarkMode ? BlockLogoWhite : BlockLogoBlack}
                alt="Block Logo"
                className="h-8 w-auto"
              />
              <span className="text-2xl font-mono text-black dark:text-white">
                {String(window.appConfig.get('GOOSE_VERSION') || 'Development')}
              </span>
            </div>
          </CardContent>
        </Card>
      )}

      {/* Update Section - only show if GOOSE_VERSION is NOT set */}
      {UPDATES_ENABLED && shouldShowUpdates && (
        <div ref={updateSectionRef}>
          <Card className="rounded-lg">
            <CardHeader className="pb-0">
              <CardTitle className="mb-1">Updates</CardTitle>
              <CardDescription>
                Check for and install updates to keep goose running at its best
              </CardDescription>
            </CardHeader>
            <CardContent className="px-4">
              <UpdateSection />
            </CardContent>
          </Card>
        </div>
      )}

      {/* Notification Instructions Modal */}
      <Dialog
        open={showNotificationModal}
        onOpenChange={(open) => !open && setShowNotificationModal(false)}
      >
        <DialogContent className="sm:max-w-[500px]">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <Settings className="text-iconStandard" size={24} />
              How to Enable Notifications
            </DialogTitle>
          </DialogHeader>

          <div className="py-4">
            {/* OS-specific instructions */}
            {isMacOS ? (
              <div className="space-y-4">
                <p>To enable notifications on macOS:</p>
                <ol className="list-decimal pl-5 space-y-2">
                  <li>Open System Preferences</li>
                  <li>Click on Notifications</li>
                  <li>Find and select goose in the application list</li>
                  <li>Enable notifications and adjust settings as desired</li>
                </ol>
              </div>
            ) : (
              <div className="space-y-4">
                <p>To enable notifications on Windows:</p>
                <ol className="list-decimal pl-5 space-y-2">
                  <li>Open Settings</li>
                  <li>Go to System &gt; Notifications</li>
                  <li>Find and select goose in the application list</li>
                  <li>Toggle notifications on and adjust settings as desired</li>
                </ol>
              </div>
            )}
          </div>

          <DialogFooter>
            <Button variant="outline" onClick={() => setShowNotificationModal(false)}>
              Close
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
