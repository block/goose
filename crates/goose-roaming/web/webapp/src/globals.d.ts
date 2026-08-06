// Ambient declarations for the desktop-component reuse. The desktop app gets
// these from Electron's preload; in the browser we provide a tiny shim
// (shim.ts) with the same surface the reused components actually call.
export {};

declare global {
  interface Window {
    electron: {
      openExternal(url: string): Promise<void>;
      showMessageBox(opts: {
        type?: string;
        buttons?: string[];
        title?: string;
        message?: string;
        detail?: string;
      }): Promise<{ response: number } | void>;
    };
    appConfig?: { get(key: string): unknown };
  }
}
