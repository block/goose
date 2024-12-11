interface IElectronAPI {
  hideWindow: () => void;
  createChatWindow: (query: string) => void;
  getConfig: () => {
    GOOSE_SERVER__PORT: number;
    GOOSE_API_HOST: string;
    apiCredsMissing: boolean;
    secretKey: string;
  };
}

declare global {
  interface Window {
    electron: IElectronAPI;
  }
}