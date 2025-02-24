import React, { createContext, useContext, useState, ReactNode, useEffect } from 'react';
import { getActiveProviders } from './utils';

// Create a context for active keys
const ActiveKeysContext = createContext<
  | {
      activeKeys: string[];
      setActiveKeys: (keys: string[]) => void;
    }
  | undefined
>(undefined);

export const ActiveKeysProvider = ({ children }: { children: ReactNode }) => {
  const [activeKeys, setActiveKeys] = useState<string[]>([]); // Start with an empty list
  const [isLoading, setIsLoading] = useState(true); // Track loading state

  // Fetch active keys from the backend
  useEffect(() => {
    const fetchActiveProviders = async () => {
      try {
        const providers = await getActiveProviders(); // Fetch the active providers
        setActiveKeys(providers); // Update state with fetched providers
      } catch (error) {
        console.error('Error fetching active providers:', error);
      } finally {
        setIsLoading(false); // Ensure loading is marked as complete
      }
    };

    fetchActiveProviders(); // Call the async function
  }, []);

  // Provide active keys and ability to update them
  return (
    <ActiveKeysContext.Provider value={{ activeKeys, setActiveKeys }}>
      {!isLoading ? children : <div>Loading...</div>} {/* Conditional rendering */}
    </ActiveKeysContext.Provider>
  );
};

// Custom hook to access active keys
export const useActiveKeys = () => {
  const context = useContext(ActiveKeysContext);
  if (!context) {
    throw new Error('useActiveKeys must be used within an ActiveKeysProvider');
  }
  return context;
};
