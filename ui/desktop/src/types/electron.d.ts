interface IElectronAPI {
  hideWindow: () => void;
  createChatWindow: (query?: string, dir?: string, version?: string) => void;
  getConfig: () => {
    GOOSE_SERVER__PORT: number;
    GOOSE_API_HOST: string;
    apiCredsMissing: boolean;
    secretKey: string;
  };
  directoryChooser: () => void;
}

declare global {
  interface Window {
    electron: IElectronAPI;
  }
}
