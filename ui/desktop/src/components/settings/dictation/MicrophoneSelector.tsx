import { useState, useEffect, useCallback } from 'react';
import { ChevronDown, RefreshCw, Mic, Check } from 'lucide-react';
import {
  DropdownMenu,
  DropdownMenuTrigger,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuLabel,
} from '../../ui/dropdown-menu';

interface MicrophoneSelectorProps {
  selectedDeviceId: string | null;
  onDeviceChange: (deviceId: string | null) => void;
}

interface AudioDevice {
  deviceId: string;
  label: string;
}

export const MicrophoneSelector = ({
  selectedDeviceId,
  onDeviceChange,
}: MicrophoneSelectorProps) => {
  const [devices, setDevices] = useState<AudioDevice[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [permissionDenied, setPermissionDenied] = useState(false);
  const [testingMic, setTestingMic] = useState(false);
  const [micLevel, setMicLevel] = useState(0);

  const loadDevices = useCallback(async () => {
    if (!navigator.mediaDevices?.enumerateDevices) return;

    setIsLoading(true);
    try {
      let deviceList = await navigator.mediaDevices.enumerateDevices();
      let audioInputs = deviceList.filter((d) => d.kind === 'audioinput');

      // If labels are empty, we need to request permission first
      const hasLabels = audioInputs.some((d) => d.label);
      if (!hasLabels && audioInputs.length > 0) {
        try {
          const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
          stream.getTracks().forEach((track) => track.stop());
          // Re-enumerate after permission granted
          deviceList = await navigator.mediaDevices.enumerateDevices();
          audioInputs = deviceList.filter((d) => d.kind === 'audioinput');
          setPermissionDenied(false);
        } catch (err: unknown) {
          const error = err as { name?: string };
          if (error.name === 'NotAllowedError' || error.name === 'SecurityError') {
            setPermissionDenied(true);
          }
        }
      }

      setDevices(
        audioInputs.map((d, index) => ({
          deviceId: d.deviceId,
          label: d.label || `Microphone ${index + 1}`,
        }))
      );
    } catch (err) {
      console.error('Failed to enumerate audio devices:', err);
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    loadDevices();

    const handleDeviceChange = () => {
      loadDevices();
    };
    navigator.mediaDevices?.addEventListener('devicechange', handleDeviceChange);
    return () => {
      navigator.mediaDevices?.removeEventListener('devicechange', handleDeviceChange);
    };
  }, [loadDevices]);

  const getSelectedLabel = (): string => {
    if (!selectedDeviceId) return 'System default';
    const device = devices.find((d) => d.deviceId === selectedDeviceId);
    return device?.label || 'System default';
  };

  const handleTestMicrophone = async () => {
    if (testingMic) {
      setTestingMic(false);
      setMicLevel(0);
      return;
    }

    setTestingMic(true);
    try {
      const constraints = selectedDeviceId
        ? { audio: { deviceId: { exact: selectedDeviceId } } }
        : { audio: true as const };

      const stream = await navigator.mediaDevices.getUserMedia(constraints);
      const audioContext = new AudioContext();
      const source = audioContext.createMediaStreamSource(stream);
      const analyser = audioContext.createAnalyser();
      analyser.fftSize = 256;
      source.connect(analyser);

      const dataArray = new Uint8Array(analyser.frequencyBinCount);
      let animationId: number;
      let stopped = false;

      const updateLevel = () => {
        if (stopped) return;
        analyser.getByteFrequencyData(dataArray);
        const average = dataArray.reduce((sum, val) => sum + val, 0) / dataArray.length;
        setMicLevel(Math.min(100, (average / 128) * 100));
        animationId = requestAnimationFrame(updateLevel);
      };
      updateLevel();

      // Auto-stop after 5 seconds
      setTimeout(() => {
        stopped = true;
        cancelAnimationFrame(animationId);
        stream.getTracks().forEach((track) => track.stop());
        audioContext.close().catch(() => {});
        setTestingMic(false);
        setMicLevel(0);
      }, 5000);
    } catch (err) {
      console.error('Failed to test microphone:', err);
      setTestingMic(false);
      setMicLevel(0);
    }
  };

  return (
    <div className="space-y-2">
      <div className="py-2 px-2 hover:bg-background-muted rounded-lg transition-all">
        <div className="flex items-center justify-between gap-2">
          <div className="min-w-0">
            <h3 className="text-text-default">Preferred Microphone</h3>
            <p className="text-xs text-text-muted mt-[2px]">
              Select which microphone to use for voice dictation
            </p>
          </div>
          <button
            onClick={() => loadDevices()}
            className="p-1.5 text-text-muted hover:text-text-default transition-colors rounded-md hover:bg-background-subtle flex-shrink-0"
            title="Refresh device list"
          >
            <RefreshCw className={`w-3.5 h-3.5 ${isLoading ? 'animate-spin' : ''}`} />
          </button>
        </div>

        <div className="mt-2 relative">
          <DropdownMenu onOpenChange={(open) => open && loadDevices()}>
            <DropdownMenuTrigger asChild>
              <button className="flex items-center justify-between gap-2 w-full px-3 py-1.5 text-sm border border-border-subtle rounded-md hover:border-border-default transition-colors text-text-default bg-background-default">
                <span className="truncate">{getSelectedLabel()}</span>
                <ChevronDown className="w-4 h-4 flex-shrink-0" />
              </button>
            </DropdownMenuTrigger>
            <DropdownMenuContent
              align="start"
              side="bottom"
              sideOffset={4}
              collisionPadding={16}
              className="w-[var(--radix-dropdown-menu-trigger-width)] min-w-[200px] max-w-[calc(100vw-2rem)]"
            >
              <DropdownMenuLabel className="text-xs text-text-muted">
                Audio Input Device
              </DropdownMenuLabel>
              <DropdownMenuSeparator />
              <DropdownMenuItem onClick={() => onDeviceChange(null)}>
                <span className="flex-1">System default</span>
                {!selectedDeviceId && <Check className="w-4 h-4 ml-2 flex-shrink-0" />}
              </DropdownMenuItem>
              <DropdownMenuSeparator />
              {devices.map((device) => (
                <DropdownMenuItem
                  key={device.deviceId}
                  onClick={() => onDeviceChange(device.deviceId)}
                >
                  <span className="flex-1 truncate">{device.label}</span>
                  {selectedDeviceId === device.deviceId && (
                    <Check className="w-4 h-4 ml-2 flex-shrink-0" />
                  )}
                </DropdownMenuItem>
              ))}
              {devices.length === 0 && !isLoading && (
                <div className="px-2 py-1.5 text-sm text-text-muted">No microphones found</div>
              )}
              {isLoading && (
                <div className="px-2 py-1.5 text-sm text-text-muted">Loading devices...</div>
              )}
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      </div>

      {permissionDenied && (
        <div className="mx-2 p-2 bg-yellow-50 dark:bg-yellow-900/20 rounded-md">
          <p className="text-xs text-yellow-700 dark:text-yellow-400">
            Microphone access was denied. To see device names, allow microphone access in your OS
            settings (macOS: System Settings → Privacy & Security → Microphone).
          </p>
        </div>
      )}

      <div className="flex items-center gap-2 px-2">
        <button
          onClick={handleTestMicrophone}
          className="flex items-center gap-1.5 px-3 py-1.5 text-xs border border-border-subtle rounded-md hover:border-border-default transition-colors text-text-muted hover:text-text-default bg-background-default"
        >
          <Mic className={`w-3.5 h-3.5 ${testingMic ? 'text-red-500' : ''}`} />
          {testingMic ? 'Testing...' : 'Test microphone'}
        </button>
        {testingMic && (
          <div className="flex-1 h-2 bg-background-subtle rounded-full overflow-hidden">
            <div
              className="h-full bg-green-500 rounded-full transition-all duration-75"
              style={{ width: `${micLevel}%` }}
            />
          </div>
        )}
      </div>

      <p className="text-xs text-text-muted px-2">
        If you don&apos;t see your device, make sure the app is allowed to access the microphone in
        your OS settings.
      </p>
    </div>
  );
};
