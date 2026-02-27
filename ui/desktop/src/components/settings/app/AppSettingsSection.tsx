import { useState, useEffect, useRef } from 'react';
import { Switch } from '../../ui/switch';
import { Button } from '../../ui/button';
import { Settings, ChevronDown, ChevronUp } from 'lucide-react';
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from '../../ui/dialog';
import UpdateSection from './UpdateSection';

import { COST_TRACKING_ENABLED, UPDATES_ENABLED } from '../../../updates';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../../ui/card';
import ThemeSelector from '../../GooseSidebar/ThemeSelector';
import BlockLogoBlack from './icons/block-lockup_black.png';
import BlockLogoWhite from './icons/block-lockup_white.png';
import TelemetrySettings from './TelemetrySettings';
import { trackSettingToggled } from '../../../utils/analytics';
import { NavigationModeSelector } from './NavigationModeSelector';
import { NavigationStyleSelector } from './NavigationStyleSelector';
import { NavigationPositionSelector } from './NavigationPositionSelector';
import { NavigationCustomizationSettings } from './NavigationCustomizationSettings';
import { NavigationProvider, useNavigationContextSafe } from '../../Layout/NavigationContext';

interface AppSettingsSectionProps {
  scrollToSection?: string;
}

const NavigationSettingsContent: React.FC = () => {
  const [isExpanded, setIsExpanded] = useState(false);
  const navContext = useNavigationContextSafe();
  const isOverlayMode = navContext?.navigationMode === 'overlay';

  return (
    <Card className="rounded-lg">
      <CardHeader className="pb-0">
        <button
          onClick={() => setIsExpanded(!isExpanded)}
          className="w-full flex items-center justify-between text-left"
        >
          <div>
            <CardTitle className="mb-1">Navigation</CardTitle>
            <CardDescription>Customize navigation layout and behavior</CardDescription>
          </div>
          {isExpanded ? (
            <ChevronUp className="w-5 h-5 text-text-secondary" />
          ) : (
            <ChevronDown className="w-5 h-5 text-text-secondary" />
          )}
        </button>
      </CardHeader>
      {isExpanded && (
        <CardContent className="pt-4 px-4 space-y-6">
          <div>
            <h3 className="text-sm font-medium text-text-primary mb-3">Mode</h3>
            <NavigationModeSelector />
          </div>
          {!isOverlayMode && (
            <div>
              <h3 className="text-sm font-medium text-text-primary mb-3">Style</h3>
              <NavigationStyleSelector />
            </div>
          )}
          {!isOverlayMode && (
            <div>
              <h3 className="text-sm font-medium text-text-primary mb-3">Position</h3>
              <NavigationPositionSelector />
            </div>
          )}
          <div>
            <h3 className="text-sm font-medium text-text-primary mb-3">Customize Items</h3>
            <NavigationCustomizationSettings />
          </div>
        </CardContent>
      )}
    </Card>
  );
};

// Navigation Settings Card - wrapped in its own provider for settings page
const NavigationSettingsCard: React.FC = () => {
  const navContext = useNavigationContextSafe();

  // If already in a NavigationProvider context, render directly
  if (navContext) {
    return <NavigationSettingsContent />;
  }

  // Otherwise wrap with provider
  return (
    <NavigationProvider>
      <NavigationSettingsContent />
    </NavigationProvider>
  );
};

export default function AppSettingsSection({ scrollToSection }: AppSettingsSectionProps) {
  const [menuBarIconEnabled, setMenuBarIconEnabled] = useState(true);
  const [dockIconEnabled, setDockIconEnabled] = useState(true);
  const [wakelockEnabled, setWakelockEnabled] = useState(true);
  const [isMacOS, setIsMacOS] = useState(false);
  const [isDockSwitchDisabled, setIsDockSwitchDisabled] = useState(false);
  const [showNotificationModal, setShowNotificationModal] = useState(false);
  const [showPricing, setShowPricing] = useState(true);
  const [isDarkMode, setIsDarkMode] = useState(false);
  const updateSectionRef = useRef<HTMLDivElement>(null);
  const shouldShowUpdates = !window.appConfig.get('GOOSE_VERSION');

  useEffect(() => {
    setIsMacOS(window.electron.platform === 'darwin');
  }, []);

  useEffect(() => {
    const updateTheme = () => {
      setIsDarkMode(document.documentElement.classList.contains('dark'));
    };

    updateTheme();

    const observer = new MutationObserver(updateTheme);
    observer.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ['class'],
    });

    return () => observer.disconnect();
  }, []);

  useEffect(() => {
    window.electron.getSetting('showPricing').then(setShowPricing);
  }, []);

  useEffect(() => {
    if (scrollToSection === 'update' && updateSectionRef.current) {
      setTimeout(() => {
        updateSectionRef.current?.scrollIntoView({ behavior: 'smooth', block: 'center' });
      }, 100);
    }
  }, [scrollToSection]);

  useEffect(() => {
    window.electron.getMenuBarIconState().then((enabled) => {
      setMenuBarIconEnabled(enabled);
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
      trackSettingToggled('menu_bar_icon', newState);
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
      trackSettingToggled('dock_icon', newState);
    }
  };

  const handleWakelockToggle = async () => {
    const newState = !wakelockEnabled;
    const success = await window.electron.setWakelock(newState);
    if (success) {
      setWakelockEnabled(newState);
      trackSettingToggled('prevent_sleep', newState);
    }
  };

  const handleShowPricingToggle = async (checked: boolean) => {
    setShowPricing(checked);
    await window.electron.setSetting('showPricing', checked);
    trackSettingToggled('cost_tracking', checked);
    // Trigger event for other components
    window.dispatchEvent(new CustomEvent('showPricingChanged'));
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
              <h3 className="text-text-primary text-xs">Notifications</h3>
              <p className="text-xs text-text-secondary max-w-md mt-[2px]">
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
              <h3 className="text-text-primary text-xs">Menu bar icon</h3>
              <p className="text-xs text-text-secondary max-w-md mt-[2px]">
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
                <h3 className="text-text-primary text-xs">Dock icon</h3>
                <p className="text-xs text-text-secondary max-w-md mt-[2px]">
                  Show goose in the dock
                </p>
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

          {/* Prevent Sleep */}
          <div className="flex items-center justify-between">
            <div>
              <h3 className="text-text-primary text-xs">Prevent Sleep</h3>
              <p className="text-xs text-text-secondary max-w-md mt-[2px]">
                Keep your computer awake while goose is running a task (screen can still lock)
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
                <h3 className="text-text-primary">Cost Tracking</h3>
                <p className="text-xs text-text-secondary max-w-md mt-[2px]">
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

      {/* Navigation Settings */}
      <NavigationSettingsCard />

      <TelemetrySettings isWelcome={false} />

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
