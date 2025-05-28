import { Switch } from '../../ui/switch';
import { useNotificationSettings } from './useNotificationSettings';
import { InfoCircledIcon, PlayIcon } from '@radix-ui/react-icons';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '../../ui/Tooltip';
import { Label } from '../../ui/label';
import { Select } from '../../ui/Select';
import { Button } from '../../ui/button';
import { Input } from '../../ui/input';
import { createDarkSelectStyles, darkSelectTheme } from '../../ui/select-styles';
import { useState, useEffect } from 'react';
import { NOTIFICATION_CONSTANTS } from '../../../notifications/types';
// Available system sounds on macOS
const SYSTEM_SOUNDS = [
  { value: 'default', label: 'Default' },
  { value: 'Basso', label: 'Basso' },
  { value: 'Blow', label: 'Blow' },
  { value: 'Bottle', label: 'Bottle' },
  { value: 'Frog', label: 'Frog' },
  { value: 'Funk', label: 'Funk' },
  { value: 'Glass', label: 'Glass' },
  { value: 'Hero', label: 'Hero' },
  { value: 'Morse', label: 'Morse' },
  { value: 'Ping', label: 'Ping' },
  { value: 'Pop', label: 'Pop' },
  { value: 'Purr', label: 'Purr' },
  { value: 'Sosumi', label: 'Sosumi' },
  { value: 'Submarine', label: 'Submarine' },
  { value: 'Tink', label: 'Tink' },
] as const;

export default function NotificationsSection() {
  const {
    isEnabled,
    setEnabled,
    types,
    toggleType,
    onlyWhenUnfocused,
    setOnlyWhenUnfocused,
    sounds,
    setSound,
    delayBetweenNotifications,
    setDelayBetweenNotifications,
  } = useNotificationSettings();

  const isMacOS = window.electron.platform === 'darwin';

  // Move hooks to top level
  const [delayInput, setDelayInput] = useState<string>(String(delayBetweenNotifications));
  useEffect(() => {
    setDelayInput(String(delayBetweenNotifications));
  }, [delayBetweenNotifications]);

  // Don't render on non-macOS platforms
  if (!isMacOS) {
    return null;
  }

  const handleTest = (type: 'taskCompletion' | 'error') => {
    console.log(`Testing ${type} notification with sound:`, sounds[type]);
    window.electron.showNotification({
      title: `Test: ${type === 'taskCompletion' ? 'Task Complete' : 'Error Occurred'}`,
      body:
        type === 'taskCompletion'
          ? 'This is a test completion notification 🦢'
          : 'This is a test error notification ⚠️',
      type: type,
      sound: sounds[type],
    });
  };

  return (
    <section id="notifications" className="px-8">
      <div className="flex items-center justify-between mb-2">
        <div className="flex items-center gap-2">
          <h2 className="text-xl font-medium text-textStandard">Notifications</h2>
          <TooltipProvider>
            <Tooltip>
              <TooltipTrigger>
                <InfoCircledIcon className="h-5 w-5 text-yellow-500" />
              </TooltipTrigger>
              <TooltipContent>
                <p className="text-sm">
                  Notifications not working? <br />
                  Check the app notification settings in <br />
                  <Button
                    variant="link"
                    className="text-sm p-0 h-auto text-blue-500 hover:text-blue-400"
                    onClick={() => window.electron.openSystemNotificationSettings()}
                  >
                    System Preferences
                  </Button>
                </p>
              </TooltipContent>
            </Tooltip>
          </TooltipProvider>
        </div>
        <Switch checked={isEnabled} onCheckedChange={setEnabled} variant="mono" />
      </div>

      <div className="border-b border-borderSubtle pb-8">
        <p className="text-sm text-textStandard mb-4">Configure desktop notification settings</p>

        {isEnabled && (
          <div className="space-y-6">
            {/* Notification Types, Delay, and Display Conditions */}
            <div className="pb-3 space-y-4">
              {/* Task Completion */}
              <div className="flex items-center justify-between">
                <div>
                  <Label htmlFor="taskCompletion">Task Completion</Label>
                  <p className="text-sm text-textSubtle">When Goose finishes a task</p>
                </div>
                <div className="flex items-center gap-2 min-w-[200px]">
                  <Select
                    id="taskCompletion"
                    value={SYSTEM_SOUNDS.find((opt) => opt.value === sounds.taskCompletion)}
                    onChange={(option: { value: string; label: string } | null) =>
                      setSound('taskCompletion', option?.value || 'default')
                    }
                    options={SYSTEM_SOUNDS}
                    classNamePrefix="goose-select"
                    styles={createDarkSelectStyles('200px')}
                    theme={darkSelectTheme}
                    menuPlacement="top"
                  />
                  <Button
                    variant="ghost"
                    size="icon"
                    className="h-8 w-8 text-textSubtle hover:text-textStandard"
                    onClick={() => handleTest('taskCompletion')}
                  >
                    <PlayIcon className="h-4 w-4" />
                  </Button>
                  <Switch
                    checked={types.includes('taskCompletion')}
                    onCheckedChange={() => toggleType('taskCompletion')}
                    variant="mono"
                  />
                </div>
              </div>

              {/* Errors & Alerts */}
              <div className="flex items-center justify-between">
                <div>
                  <Label htmlFor="error">Errors & Alerts</Label>
                  <p className="text-sm text-textSubtle">
                    When something goes wrong or needs attention
                  </p>
                </div>
                <div className="flex items-center gap-2 min-w-[200px]">
                  <Select
                    id="error"
                    value={SYSTEM_SOUNDS.find((opt) => opt.value === sounds.error)}
                    onChange={(option: { value: string; label: string } | null) =>
                      setSound('error', option?.value || 'default')
                    }
                    options={SYSTEM_SOUNDS}
                    classNamePrefix="goose-select"
                    styles={createDarkSelectStyles('200px')}
                    theme={darkSelectTheme}
                    menuPlacement="top"
                  />
                  <Button
                    variant="ghost"
                    size="icon"
                    className="h-8 w-8 text-textSubtle hover:text-textStandard"
                    onClick={() => handleTest('error')}
                  >
                    <PlayIcon className="h-4 w-4" />
                  </Button>
                  <Switch
                    checked={types.includes('error')}
                    onCheckedChange={() => toggleType('error')}
                    variant="mono"
                  />
                </div>
              </div>

              {/* Display Conditions */}
              <div className="flex items-center justify-between">
                <div>
                  <Label htmlFor="onlyWhenUnfocused">Only when app is not focused</Label>
                  <p className="text-sm text-textSubtle">
                    Show notifications only when you're using other applications
                  </p>
                </div>
                <Switch
                  id="onlyWhenUnfocused"
                  checked={onlyWhenUnfocused}
                  onCheckedChange={setOnlyWhenUnfocused}
                  variant="mono"
                />
              </div>

              {/* Notification Delay */}
              <div className="flex items-center justify-between">
                <div>
                  <Label htmlFor="notificationDelay">Delay between notifications</Label>
                  <p className="text-sm text-textSubtle">
                    Time to wait between showing consecutive notifications (milliseconds)
                  </p>
                </div>
                <div className="flex items-center gap-2">
                  <Input
                    id="notificationDelay"
                    type="number"
                    step={10}
                    className="w-[120px]"
                    value={delayInput}
                    onChange={(e) => {
                      const value = e.target.value;
                      setDelayInput(value);
                      const num = parseInt(value, 10);
                      // Only update if within valid range (0–10000ms)
                      if (
                        !isNaN(num) &&
                        num >= NOTIFICATION_CONSTANTS.DEFAULT_MIN_NOTIFICATION_DELAY &&
                        num <= NOTIFICATION_CONSTANTS.DEFAULT_MAX_NOTIFICATION_DELAY
                      ) {
                        setDelayBetweenNotifications(num);
                      }
                    }}
                    onBlur={() => {
                      const num = parseInt(delayInput, 10);
                      if (
                        delayInput === '' ||
                        isNaN(num) ||
                        num < NOTIFICATION_CONSTANTS.DEFAULT_MIN_NOTIFICATION_DELAY ||
                        num > NOTIFICATION_CONSTANTS.DEFAULT_MAX_NOTIFICATION_DELAY
                      ) {
                        setDelayInput(String(delayBetweenNotifications));
                      }
                    }}
                  />
                  <span className="text-sm text-textSubtle">ms</span>
                </div>
              </div>
            </div>
          </div>
        )}
      </div>
    </section>
  );
}
