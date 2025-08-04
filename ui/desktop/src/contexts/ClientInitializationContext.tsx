import React, { createContext, useContext, useEffect, useState, ReactNode } from 'react';
import { ensureClientInitialized } from '../utils';

interface ClientInitializationContextType {
  isInitialized: boolean;
  initializationError: Error | null;
}

const ClientInitializationContext = createContext<ClientInitializationContextType | undefined>(
  undefined
);

interface ClientInitializationProviderProps {
  children: ReactNode;
}

export const ClientInitializationProvider: React.FC<ClientInitializationProviderProps> = ({
  children,
}) => {
  const [isInitialized, setIsInitialized] = useState(false);
  const [initializationError, setInitializationError] = useState<Error | null>(null);

  useEffect(() => {
    const initializeClient = async () => {
      try {
        await ensureClientInitialized();
        setIsInitialized(true);
      } catch (error) {
        console.error('Failed to initialize API client:', error);
        setInitializationError(error instanceof Error ? error : new Error('Unknown error'));
      }
    };

    initializeClient();
  }, []);

  return (
    <ClientInitializationContext.Provider value={{ isInitialized, initializationError }}>
      {children}
    </ClientInitializationContext.Provider>
  );
};

export const useClientInitialization = () => {
  const context = useContext(ClientInitializationContext);
  if (context === undefined) {
    throw new Error('useClientInitialization must be used within a ClientInitializationProvider');
  }
  return context;
};

// Helper component to ensure initialization before rendering children
export const RequireClientInitialization: React.FC<{ children: ReactNode }> = ({ children }) => {
  const { isInitialized, initializationError } = useClientInitialization();

  if (initializationError) {
    throw initializationError;
  }

  if (!isInitialized) {
    return (
      <div className="flex justify-center items-center py-12">
        <div className="animate-spin rounded-full h-8 w-8 border-t-2 border-b-2 border-textStandard"></div>
      </div>
    );
  }

  return <>{children}</>;
};
