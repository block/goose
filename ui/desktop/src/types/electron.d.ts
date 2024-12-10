interface IElectronAPI {
  hideWindow: () => void;
  createChatWindow: (query: string) => void;
  getConfig: () => {
    GOOSE_SERVER__PORT: number;
    GOOSE_API_HOST: string;
    apiCredsMissing: boolean;
  };
  getSession: (sessionId: string) => object;
  saveSession: (session: { name: string; messages: Array<object>; directory: string }) => string;
}

declare global {
  interface Window {
    electron: IElectronAPI;
  }
}
