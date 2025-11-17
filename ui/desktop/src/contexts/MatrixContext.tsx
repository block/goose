import React, { createContext, useContext, useEffect, useState, ReactNode } from 'react';
import { MatrixService, MatrixUser, MatrixRoom, GooseAIMessage, GooseChatMessage, GooseInstance } from '../services/MatrixService';

interface MatrixContextType {
  // Connection state
  isConnected: boolean;
  isReady: boolean;
  currentUser: MatrixUser | null;
  
  // Data
  friends: MatrixUser[];
  rooms: MatrixRoom[];
  gooseInstances: GooseInstance[];
  
  // Actions
  login: (username: string, password: string) => Promise<void>;
  register: (username: string, password: string) => Promise<void>;
  logout: () => Promise<void>;
  searchUsers: (query: string) => Promise<MatrixUser[]>;
  addFriend: (userId: string) => Promise<void>;
  createAISession: (name: string, inviteUserIds?: string[]) => Promise<string>;
  joinRoom: (roomId: string) => Promise<void>;
  inviteToRoom: (roomId: string, userId: string) => Promise<void>;
  sendMessage: (roomId: string, message: string) => Promise<void>;
  sendAIPrompt: (roomId: string, prompt: string, sessionId: string, model?: string) => Promise<void>;
  setAvatar: (file: File) => Promise<string>;
  removeAvatar: () => Promise<void>;
  setDisplayName: (displayName: string) => Promise<void>;
  
  // Goose-to-Goose Communication
  sendGooseMessage: (roomId: string, content: string, type?: GooseChatMessage['type'], options?: any) => Promise<string>;
  sendTaskRequest: (roomId: string, taskDescription: string, taskType: string, options?: any) => Promise<string>;
  sendTaskResponse: (roomId: string, taskId: string, response: string, status: 'completed' | 'failed', options?: any) => Promise<string>;
  sendCollaborationInvite: (roomId: string, projectDescription: string, requiredCapabilities?: string[], metadata?: Record<string, any>) => Promise<string>;
  acceptCollaborationInvite: (roomId: string, originalMessageId: string, capabilities?: string[], metadata?: Record<string, any>) => Promise<string>;
  declineCollaborationInvite: (roomId: string, originalMessageId: string, reason?: string, metadata?: Record<string, any>) => Promise<string>;
  createGooseCollaborationRoom: (name: string, inviteGooseIds?: string[], topic?: string) => Promise<string>;
  announceCapabilities: (roomId: string, capabilities: string[], status?: 'idle' | 'busy' | 'working', currentTask?: string) => Promise<string>;
  findDirectMessageRoom: (userId: string) => string | null;
  getOrCreateDirectMessageRoom: (userId: string) => Promise<string>;
  
  // Events
  onMessage: ((callback: (data: any) => void) => () => void) & ((eventName: string, callback: (data: any) => void) => () => void);
  onAIMessage: (callback: (message: GooseAIMessage) => void) => () => void;
  onGooseMessage: (callback: (message: GooseChatMessage) => void) => () => void;
  onSessionMessage: (callback: (data: any) => void) => () => void;
  onPresenceChange: (callback: (data: any) => void) => () => void;
  
  // Room history
  getRoomHistory: (roomId: string, limit?: number) => Promise<Array<{
    messageId: string;
    sender: string;
    content: string;
    timestamp: Date;
    type: 'user' | 'assistant' | 'system';
    isFromSelf: boolean;
    senderInfo: {
      userId: string;
      displayName?: string;
      avatarUrl?: string;
    };
    metadata?: Record<string, any>;
  }>>;
  getRoomHistoryAsGooseMessages: (roomId: string, limit?: number) => Promise<Array<{
    role: 'user' | 'assistant' | 'system';
    content: string;
    timestamp: Date;
    sender?: string;
    metadata?: Record<string, any>;
  }>>;

  // Debug methods
  debugGooseMessage: (roomId: string) => Promise<void>;
  getDebugInfo: () => Record<string, any>;
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
  const [gooseInstances, setGooseInstances] = useState<GooseInstance[]>([]);

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

    const handleAvatarUpdated = () => {
      updateData();
    };

    const handleDisplayNameUpdated = () => {
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
    matrixService.on('avatarUpdated', handleAvatarUpdated);
    matrixService.on('displayNameUpdated', handleDisplayNameUpdated);

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
      matrixService.off('avatarUpdated', handleAvatarUpdated);
      matrixService.off('displayNameUpdated', handleDisplayNameUpdated);
    };
  }, [matrixService]);

  const updateData = () => {
    console.log('MatrixContext - updateData called');
    const user = matrixService.getCurrentUser();
    console.log('MatrixContext - updated currentUser:', user);
    setCurrentUser(user);
    setFriends(matrixService.getFriends());
    setRooms(matrixService.getRooms());
    setGooseInstances(matrixService.getGooseInstances());
  };

  const login = async (username: string, password: string) => {
    await matrixService.login(username, password);
  };

  const register = async (username: string, password: string) => {
    await matrixService.register(username, password);
  };

  const logout = async () => {
    await matrixService.logout();
  };

  const searchUsers = async (query: string) => {
    return await matrixService.searchUsers(query);
  };

  const addFriend = async (userId: string) => {
    await matrixService.addFriend(userId);
    // Data will be updated via the membershipChange event
  };

  const createAISession = async (name: string, inviteUserIds: string[] = []) => {
    return await matrixService.createAISession(name, inviteUserIds);
  };

  const joinRoom = async (roomId: string) => {
    await matrixService.joinRoom(roomId);
    // Data will be updated via the membershipChange event
  };

  const inviteToRoom = async (roomId: string, userId: string) => {
    await matrixService.inviteToRoom(roomId, userId);
    // Data will be updated via the membershipChange event
  };

  const sendMessage = async (roomId: string, message: string) => {
    await matrixService.sendMessage(roomId, message);
  };

  const sendAIPrompt = async (roomId: string, prompt: string, sessionId: string, model?: string) => {
    await matrixService.sendAIPrompt(roomId, prompt, sessionId, model);
  };

  const setAvatar = async (file: File) => {
    return await matrixService.setAvatar(file);
  };

  const removeAvatar = async () => {
    await matrixService.removeAvatar();
  };

  const setDisplayName = async (displayName: string) => {
    await matrixService.setDisplayName(displayName);
  };

  const onMessage = (eventNameOrCallback: string | ((data: any) => void), callback?: (data: any) => void) => {
    if (typeof eventNameOrCallback === 'string' && callback) {
      // Support for specific event names like 'gooseSessionSync'
      matrixService.on(eventNameOrCallback, callback);
      return () => matrixService.off(eventNameOrCallback, callback);
    } else if (typeof eventNameOrCallback === 'function') {
      // Default behavior for 'message' event
      matrixService.on('message', eventNameOrCallback);
      return () => matrixService.off('message', eventNameOrCallback);
    } else {
      throw new Error('Invalid arguments for onMessage');
    }
  };

  const onAIMessage = (callback: (message: GooseAIMessage) => void) => {
    matrixService.on('aiMessage', callback);
    return () => matrixService.off('aiMessage', callback);
  };

  const onSessionMessage = (callback: (data: any) => void) => {
    matrixService.on('sessionMessage', callback);
    return () => matrixService.off('sessionMessage', callback);
  };

  const onPresenceChange = (callback: (data: any) => void) => {
    matrixService.on('presenceChange', callback);
    return () => matrixService.off('presenceChange', callback);
  };

  // Goose-to-Goose Communication methods
  const sendGooseMessage = async (roomId: string, content: string, type?: GooseChatMessage['type'], options?: any) => {
    return await matrixService.sendGooseMessage(roomId, content, type, options);
  };

  const sendTaskRequest = async (roomId: string, taskDescription: string, taskType: string, options?: any) => {
    return await matrixService.sendTaskRequest(roomId, taskDescription, taskType, options);
  };

  const sendTaskResponse = async (roomId: string, taskId: string, response: string, status: 'completed' | 'failed', options?: any) => {
    return await matrixService.sendTaskResponse(roomId, taskId, response, status, options);
  };

  const sendCollaborationInvite = async (roomId: string, projectDescription: string, requiredCapabilities?: string[], metadata?: Record<string, any>) => {
    return await matrixService.sendCollaborationInvite(roomId, projectDescription, requiredCapabilities, metadata);
  };

  const acceptCollaborationInvite = async (roomId: string, originalMessageId: string, capabilities?: string[], metadata?: Record<string, any>) => {
    return await matrixService.acceptCollaborationInvite(roomId, originalMessageId, capabilities, metadata);
  };

  const declineCollaborationInvite = async (roomId: string, originalMessageId: string, reason?: string, metadata?: Record<string, any>) => {
    return await matrixService.declineCollaborationInvite(roomId, originalMessageId, reason, metadata);
  };

  const createGooseCollaborationRoom = async (name: string, inviteGooseIds?: string[], topic?: string) => {
    return await matrixService.createGooseCollaborationRoom(name, inviteGooseIds, topic);
  };

  const announceCapabilities = async (roomId: string, capabilities: string[], status?: 'idle' | 'busy' | 'working', currentTask?: string) => {
    return await matrixService.announceCapabilities(roomId, capabilities, status, currentTask);
  };

  const findDirectMessageRoom = (userId: string) => {
    return matrixService.findDirectMessageRoom(userId);
  };

  const getOrCreateDirectMessageRoom = async (userId: string) => {
    return await matrixService.getOrCreateDirectMessageRoom(userId);
  };

  const onGooseMessage = (callback: (message: GooseChatMessage) => void) => {
    matrixService.on('gooseMessage', callback);
    return () => matrixService.off('gooseMessage', callback);
  };

  // Room history methods
  const getRoomHistory = async (roomId: string, limit?: number) => {
    return await matrixService.getRoomHistory(roomId, limit);
  };

  const getRoomHistoryAsGooseMessages = async (roomId: string, limit?: number) => {
    return await matrixService.getRoomHistoryAsGooseMessages(roomId, limit);
  };

  // Debug methods
  const debugGooseMessage = async (roomId: string) => {
    return await matrixService.debugGooseMessage(roomId);
  };

  const getDebugInfo = () => {
    return matrixService.getDebugInfo();
  };

  const contextValue: MatrixContextType = {
    isConnected,
    isReady,
    currentUser,
    friends,
    rooms,
    gooseInstances,
    login,
    register,
    logout,
    searchUsers,
    addFriend,
    createAISession,
    joinRoom,
    inviteToRoom,
    sendMessage,
    sendAIPrompt,
    setAvatar,
    removeAvatar,
    setDisplayName,
    // Goose-to-Goose Communication
    sendGooseMessage,
    sendTaskRequest,
    sendTaskResponse,
    sendCollaborationInvite,
    acceptCollaborationInvite,
    declineCollaborationInvite,
    createGooseCollaborationRoom,
    announceCapabilities,
    findDirectMessageRoom,
    getOrCreateDirectMessageRoom,
    // Events
    onMessage,
    onAIMessage,
    onGooseMessage,
    onSessionMessage,
    onPresenceChange,
    // Room history
    getRoomHistory,
    getRoomHistoryAsGooseMessages,
    // Debug methods
    debugGooseMessage,
    getDebugInfo,
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
