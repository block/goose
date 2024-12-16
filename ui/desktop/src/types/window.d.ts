interface Window {
  electron: {
    createChatWindow: (query?: string) => void;
    getConfig: () => {
      GOOSE_SERVER__PORT: number;
      GOOSE_API_HOST: string;
      apiCredsMissing: boolean;
      secretKey: string;
    };
    logInfo: (message: string) => void;
    showNotification: (options: { title: string; body: string }) => void;
    selectFileOrDirectory: () => Promise<string | null>;
    hideWindow: () => void;
  };
  appConfig: {
    get: (key: string) => any;
  };
}
