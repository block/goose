import React from 'react';
import { useNetworkStatus, NetworkStatus } from '../hooks/useNetworkStatus';
import { WifiOff, Wifi, AlertTriangle } from 'lucide-react';

interface NetworkStatusIndicatorProps {
  className?: string;
}

export const NetworkStatusIndicator: React.FC<NetworkStatusIndicatorProps> = ({
  className = '',
}) => {
  const { networkStatus } = useNetworkStatus();

  const getStatusIcon = (status: NetworkStatus) => {
    switch (status) {
      case 'online':
        return <Wifi className="w-4 h-4 text-green-600" />;
      case 'offline':
        return <WifiOff className="w-4 h-4 text-red-600" />;
      case 'degraded':
        return <AlertTriangle className="w-4 h-4 text-yellow-600" />;
      case 'unknown':
        return <Wifi className="w-4 h-4 text-gray-500 animate-pulse" />;
    }
  };

  return (
    <div className={`inline-flex items-center justify-center ${className}`}>
      {getStatusIcon(networkStatus.status)}
    </div>
  );
};

// Network status context provider for global access
interface NetworkStatusContextValue {
  networkStatus: ReturnType<typeof useNetworkStatus>['networkStatus'];
  isReconnecting: boolean;
  reconnectAttempts: number;
}

const NetworkStatusContext = React.createContext<NetworkStatusContextValue | null>(null);

export const NetworkStatusProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const networkStatusData = useNetworkStatus();

  return (
    <NetworkStatusContext.Provider value={networkStatusData}>
      {children}
    </NetworkStatusContext.Provider>
  );
};

export const useNetworkStatusContext = () => {
  const context = React.useContext(NetworkStatusContext);
  if (!context) {
    throw new Error('useNetworkStatusContext must be used within NetworkStatusProvider');
  }
  return context;
};
