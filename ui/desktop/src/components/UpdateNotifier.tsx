import { useEffect } from 'react';
import { toast } from 'react-toastify';

export function UpdateNotifier() {
  useEffect(() => {
    // Handle update events
    const handleUpdateAvailable = (info: { version: string }) => {
      toast.info(`New version ${info.version} available and downloading...`, {
        autoClose: 5000,
      });
    };

    const handleUpdateDownloaded = (info: { version: string }) => {
      toast.success(
        `Version ${info.version} has been downloaded and will be installed when you quit the app.`,
        {
          autoClose: false,
        }
      );
    };

    const handleUpdateError = (error: string) => {
      toast.error(`Update error: ${error}`, {
        autoClose: 5000,
      });
    };

    // Subscribe to update events
    window.electron.on('update-available', handleUpdateAvailable);
    window.electron.on('update-downloaded', handleUpdateDownloaded);
    window.electron.on('update-error', handleUpdateError);

    // Cleanup
    return () => {
      window.electron.off('update-available', handleUpdateAvailable);
      window.electron.off('update-downloaded', handleUpdateDownloaded);
      window.electron.off('update-error', handleUpdateError);
    };
  }, []);

  // This component doesn't render anything
  return null;
}
