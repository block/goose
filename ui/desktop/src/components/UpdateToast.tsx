import React, { useEffect, useState } from 'react';

const IPC_UPDATE_DOWNLOADED = 'update-downloaded';
const IPC_INSTALL_UPDATE = 'install-update';

// Small unobtrusive component that shows a compact "Restart to install" button
export default function UpdateToast(): React.JSX.Element | null {
  const [updateAvailable, setUpdateAvailable] = useState(false);
  const [version, setVersion] = useState<string | null>(null);

  useEffect(() => {
    type VersionPayload = { version?: string } | undefined;
    const wrapped = (payload: VersionPayload) => {
      setVersion(payload?.version || null);
      setUpdateAvailable(true);
    };
    // Use preload-exposed API to avoid direct electron imports in renderer
    window.electron.on<VersionPayload>(IPC_UPDATE_DOWNLOADED, wrapped);

    return () => {
      // Off uses the original listener reference; our wrapped mapping in preload
      // will find and remove the correct ipcRenderer listener.
      window.electron.off(IPC_UPDATE_DOWNLOADED, wrapped as (...args: unknown[]) => void);
    };
  }, []);

  if (!updateAvailable) return null;

  return (
    <div style={{ position: 'fixed', right: 12, bottom: 12, zIndex: 9999 }}>
      <button
        onClick={() => {
          window.electron.emit(IPC_INSTALL_UPDATE);
        }}
        style={{
          padding: '8px 12px',
          borderRadius: 6,
          background: '#007aff',
          color: 'white',
          border: 'none',
        }}
        aria-label={`Restart to install update${version ? ` ${version}` : ''}`}
      >
        Restart to install
      </button>
    </div>
  );
}
