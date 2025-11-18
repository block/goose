import { useCallback, useEffect, useState } from 'react';
import { userContextService, UserProfile, UserIntroduction } from '../services/UserContextService';

interface UseUserContextReturn {
  // User profiles
  getUserProfile: (userId: string) => Promise<UserProfile | null>;
  searchUsers: (name: string) => Promise<UserProfile[]>;
  getAllUsers: () => Promise<UserProfile[]>;
  
  // Introduction processing
  processIntroduction: (
    message: string,
    introducedBy: string,
    sessionId: string,
    mentionedUserIds?: string[]
  ) => Promise<UserIntroduction[]>;
  
  // Context generation
  generateContextSummary: (sessionId: string) => Promise<string>;
  containsIntroductions: (message: string) => boolean;
  
  // User management
  updateUserProfile: (userId: string, updates: Partial<UserProfile>) => Promise<UserProfile>;
  updateLastSeen: (userId: string) => Promise<void>;
  addCollaborationHistory: (userId: string, sessionId: string, role: 'owner' | 'collaborator') => Promise<void>;
  
  // State
  isInitialized: boolean;
  error: string | null;
}

export const useUserContext = (): UseUserContextReturn => {
  const [isInitialized, setIsInitialized] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Initialize the service
  useEffect(() => {
    const initializeService = async () => {
      try {
        await userContextService.initialize();
        setIsInitialized(true);
        setError(null);
      } catch (err) {
        console.error('Failed to initialize UserContextService:', err);
        setError(err instanceof Error ? err.message : 'Failed to initialize user context');
      }
    };

    initializeService();
  }, []);

  // Wrapped service methods with error handling
  const getUserProfile = useCallback(async (userId: string): Promise<UserProfile | null> => {
    try {
      return await userContextService.getUserProfile(userId);
    } catch (err) {
      console.error('Failed to get user profile:', err);
      setError(err instanceof Error ? err.message : 'Failed to get user profile');
      return null;
    }
  }, []);

  const searchUsers = useCallback(async (name: string): Promise<UserProfile[]> => {
    try {
      return await userContextService.searchUsersByName(name);
    } catch (err) {
      console.error('Failed to search users:', err);
      setError(err instanceof Error ? err.message : 'Failed to search users');
      return [];
    }
  }, []);

  const getAllUsers = useCallback(async (): Promise<UserProfile[]> => {
    try {
      return await userContextService.getAllUserProfiles();
    } catch (err) {
      console.error('Failed to get all users:', err);
      setError(err instanceof Error ? err.message : 'Failed to get all users');
      return [];
    }
  }, []);

  const processIntroduction = useCallback(async (
    message: string,
    introducedBy: string,
    sessionId: string,
    mentionedUserIds?: string[]
  ): Promise<UserIntroduction[]> => {
    try {
      return await userContextService.processIntroduction(message, introducedBy, sessionId, mentionedUserIds);
    } catch (err) {
      console.error('Failed to process introduction:', err);
      setError(err instanceof Error ? err.message : 'Failed to process introduction');
      return [];
    }
  }, []);

  const generateContextSummary = useCallback(async (sessionId: string): Promise<string> => {
    try {
      return await userContextService.generateUserContextSummary(sessionId);
    } catch (err) {
      console.error('Failed to generate context summary:', err);
      setError(err instanceof Error ? err.message : 'Failed to generate context summary');
      return '';
    }
  }, []);

  const containsIntroductions = useCallback((message: string): boolean => {
    try {
      return userContextService.containsIntroductions(message);
    } catch (err) {
      console.error('Failed to check for introductions:', err);
      setError(err instanceof Error ? err.message : 'Failed to check for introductions');
      return false;
    }
  }, []);

  const updateUserProfile = useCallback(async (
    userId: string,
    updates: Partial<UserProfile>
  ): Promise<UserProfile> => {
    try {
      const profile = await userContextService.createOrUpdateUserProfile(userId, updates);
      setError(null);
      return profile;
    } catch (err) {
      console.error('Failed to update user profile:', err);
      setError(err instanceof Error ? err.message : 'Failed to update user profile');
      throw err;
    }
  }, []);

  const updateLastSeen = useCallback(async (userId: string): Promise<void> => {
    try {
      await userContextService.updateLastSeen(userId);
    } catch (err) {
      console.error('Failed to update last seen:', err);
      setError(err instanceof Error ? err.message : 'Failed to update last seen');
    }
  }, []);

  const addCollaborationHistory = useCallback(async (
    userId: string,
    sessionId: string,
    role: 'owner' | 'collaborator'
  ): Promise<void> => {
    try {
      await userContextService.addCollaborationHistory(userId, sessionId, role);
    } catch (err) {
      console.error('Failed to add collaboration history:', err);
      setError(err instanceof Error ? err.message : 'Failed to add collaboration history');
    }
  }, []);

  return {
    // User profiles
    getUserProfile,
    searchUsers,
    getAllUsers,
    
    // Introduction processing
    processIntroduction,
    
    // Context generation
    generateContextSummary,
    containsIntroductions,
    
    // User management
    updateUserProfile,
    updateLastSeen,
    addCollaborationHistory,
    
    // State
    isInitialized,
    error,
  };
};
