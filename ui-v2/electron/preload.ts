import { contextBridge, ipcRenderer } from 'electron';

// Define the API interface
interface ElectronAPI {
  copyToClipboard(text: string): Promise<void>;
}

// Expose protected methods that allow the renderer process to use
// the ipcRenderer without exposing the entire object
contextBridge.exposeInMainWorld('electronAPI', {
  copyToClipboard: (text: string) => ipcRenderer.invoke('clipboard-copy', text),
} as ElectronAPI);
