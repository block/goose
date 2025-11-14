import React, { createContext, useContext, useEffect, useState, ReactNode } from 'react';
import { MatrixService, MatrixUser, MatrixRoom, GooseAIMessage } from '../services/MatrixService';

interface MatrixContextType {
  // Connection state
  isConnected: boolean;
  isReady: boolean;
  currentUser: MatrixUser | null;
  
  // Data
  friends: MatrixUser[];
  rooms: MatrixRoom[];
  
  // Actions
  login: (username: string, password: string) => Promise<void>;
  register: (username: string, password: string) => Promise<void>;
  logout: () => Promise<void>;
  searchUsers: (query: string) => Promise<MatrixUser[]>;
  addFriend: (userId: string) => Promise<void>;
  createAISession: (name: string, inviteUserIds?: string[]) => Promise<string>;
  sendMessage: (roomId: string, message: string) => Promise<void>;
  sendAIPrompt: (roomId: string, prompt: string, sessionId: string, model?: string) => Promise<void>;
  
  // Events
  onMessage: (callback: (data: any) => void) => () => void;
  onAIMessage: (callback: (message: GooseAIMessage) => void) => () => void;
  onPresenceChange: (callback: (data: any) => void) => () => void;
}

const MatrixContext = createContext<MatrixContextType | null>(null);

interface MatrixProviderProps {
  children: ReactNode;
  matrixService: MatrixService;
}

export const MatrixProvider: React.FC<MatrixProviderProps> = ({ children, matrixService }) => {
  const [isConnected, setIsConnected] = useState(false);
  const [isReady, setIsReady] = useState(false);
  const [currentUser, setCurrentUser] = useState<MatrixUser | null>(null);
  const [friends, setFriends] = useState<MatrixUser[]>([]);
  const [rooms, setRooms] = useState<MatrixRoom[]>([]);

  useEffect(() => {
    // Initialize Matrix service
    matrixService.initialize().catch(console.error);

    // Setup event listeners
    const handleConnected = () => {
      setIsConnected(true);
      updateData();
    };

    const handleReady = () => {
      setIsReady(true);
      updateData();
    };

    const handleDisconnected = () => {
      setIsConnected(false);
      setIsReady(false);
      setCurrentUser(null);
      setFriends([]);
      setRooms([]);
    };

    const handleLogin = () => {
      updateData();
    };

    const handleSync = ({ state }: { state: string }) => {
      if (state === 'PREPARED') {
        updateData();
      }
    };

    const handleMembershipChange = () => {
      updateData();
    };

    const handlePresenceChange = () => {
      updateData();
    };

    // Add event listeners
    matrixService.on('connected', handleConnected);
    matrixService.on('ready', handleReady);
    matrixService.on('disconnected', handleDisconnected);
    matrixService.on('login', handleLogin);
    matrixService.on('register', handleLogin);
    matrixService.on('sync', handleSync);
    matrixService.on('membershipChange', handleMembershipChange);
    matrixService.on('presenceChange', handlePresenceChange);

    // Cleanup
    return () => {
      matrixService.off('connected', handleConnected);
      matrixService.off('ready', handleReady);
      matrixService.off('disconnected', handleDisconnected);
      matrixService.off('login', handleLogin);
      matrixService.off('register', handleLogin);
      matrixService.off('sync', handleSync);
      matrixService.off('membershipChange', handleMembershipChange);
      matrixService.off('presenceChange', handlePresenceChange);
    };
  }, [matrixService]);

  const updateData = () => {
    setCurrentUser(matrixService.getCurrentUser());
    setFriends(matrixService.getFriends());
    setRooms(matrixService.getRooms());
  };

  const login = async (username: string, password: string) => {
    await matrixService.login(username, password);
  };

  const register = async (username: string, password: string) => {
    await matrixService.register(username, password);
  };

  const logout = async () => {
    await matrixService.disconnect();
  };

  const searchUsers = async (query: string) => {
    return await matrixService.searchUsers(query);
  };

  const addFriend = async (userId: string) => {
    await matrixService.createDirectMessage(userId);
    // Data will be updated via the membershipChange event
  };

  const createAISession = async (name: string, inviteUserIds: string[] = []) => {
    return await matrixService.createAISession(name, inviteUserIds);
  };

  const sendMessage = async (roomId: string, message: string) => {
    await matrixService.sendMessage(roomId, message);
  };

  const sendAIPrompt = async (roomId: string, prompt: string, sessionId: string, model?: string) => {
    await matrixService.sendAIPrompt(roomId, prompt, sessionId, model);
  };

  const onMessage = (callback: (data: any) => void) => {
    matrixService.on('message', callback);
    return () => matrixService.off('message', callback);
  };

  const onAIMessage = (callback: (message: GooseAIMessage) => void) => {
    matrixService.on('aiMessage', callback);
    return () => matrixService.off('aiMessage', callback);
  };

  const onPresenceChange = (callback: (data: any) => void) => {
    matrixService.on('presenceChange', callback);
    return () => matrixService.off('presenceChange', callback);
  };

  const contextValue: MatrixContextType = {
    isConnected,
    isReady,
    currentUser,
    friends,
    rooms,
    login,
    register,
    logout,
    searchUsers,
    addFriend,
    createAISession,
    sendMessage,
    sendAIPrompt,
    onMessage,
    onAIMessage,
    onPresenceChange,
  };

  return (
    <MatrixContext.Provider value={contextValue}>
      {children}
    </MatrixContext.Provider>
  );
};

export const useMatrix = (): MatrixContextType => {
  const context = useContext(MatrixContext);
  if (!context) {
    throw new Error('useMatrix must be used within a MatrixProvider');
  }
  return context;
};

export default MatrixContext;
