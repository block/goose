/**
 * useMatrixHistory - Hook to integrate Matrix rooms with chat history
 */

import { useState, useEffect, useCallback } from 'react';
import { matrixHistoryService, MatrixHistorySession } from '../services/MatrixHistoryService';
import { useMatrix } from '../contexts/MatrixContext';

export interface UseMatrixHistoryReturn {
  matrixSessions: MatrixHistorySession[];
  isLoading: boolean;
  isInitialized: boolean;
  error: string | null;
  refreshMatrixHistory: () => Promise<void>;
  getSessionIdForRoom: (matrixRoomId: string) => string | null;
  isMatrixSession: (sessionId: string) => boolean;
  getMatrixRoomIdForSession: (sessionId: string) => string | null;
}

export const useMatrixHistory = (): UseMatrixHistoryReturn => {
  const [matrixSessions, setMatrixSessions] = useState<MatrixHistorySession[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [isInitialized, setIsInitialized] = useState(false);
  const [error, setError] = useState<string | null>(null);
  
  const { isConnected, isReady } = useMatrix();

  // Load Matrix sessions
  const loadMatrixSessions = useCallback(async () => {
    if (!isConnected || !isReady) {
      console.log('🔄 useMatrixHistory: Matrix not ready, skipping load');
      return;
    }

    try {
      setIsLoading(true);
      setError(null);
      
      console.log('🔄 useMatrixHistory: Loading Matrix history sessions...');
      
      // Initialize the service if needed
      await matrixHistoryService.initialize();
      
      // Get all Matrix sessions
      const sessions = await matrixHistoryService.getMatrixHistorySessions();
      
      console.log(`✅ useMatrixHistory: Loaded ${sessions.length} Matrix sessions`);
      setMatrixSessions(sessions);
      setIsInitialized(true);
      
    } catch (err) {
      console.error('❌ useMatrixHistory: Failed to load Matrix sessions:', err);
      setError(err instanceof Error ? err.message : 'Failed to load Matrix history');
    } finally {
      setIsLoading(false);
    }
  }, [isConnected, isReady]);

  // Refresh Matrix history
  const refreshMatrixHistory = useCallback(async () => {
    console.log('🔄 useMatrixHistory: Refreshing Matrix history...');
    await matrixHistoryService.refreshAllRooms();
    await loadMatrixSessions();
  }, [loadMatrixSessions]);

  // Helper functions
  const getSessionIdForRoom = useCallback((matrixRoomId: string) => {
    return matrixHistoryService.getSessionIdForRoom(matrixRoomId);
  }, []);

  const isMatrixSession = useCallback((sessionId: string) => {
    return matrixHistoryService.isMatrixSession(sessionId);
  }, []);

  const getMatrixRoomIdForSession = useCallback((sessionId: string) => {
    return matrixHistoryService.getMatrixRoomIdForSession(sessionId);
  }, []);

  // Load sessions when Matrix becomes ready
  useEffect(() => {
    if (isConnected && isReady && !isInitialized) {
      console.log('🔄 useMatrixHistory: Matrix ready, loading sessions...');
      loadMatrixSessions();
    }
  }, [isConnected, isReady, isInitialized, loadMatrixSessions]);

  // Listen for Matrix events to update sessions
  useEffect(() => {
    if (!isConnected || !isReady) {
      return;
    }

    // Listen for room joined events to sync new rooms
    const handleRoomJoined = (data: { roomId: string }) => {
      console.log('🔄 useMatrixHistory: Room joined, syncing to history:', data.roomId);
      
      // Sync the new room and refresh sessions
      matrixHistoryService.syncNewRoom(data.roomId).then(() => {
        loadMatrixSessions();
      }).catch(error => {
        console.error('❌ useMatrixHistory: Failed to sync new room:', error);
      });
    };

    // Add event listeners (assuming matrixService has these events)
    // Note: You may need to add these events to MatrixService if they don't exist
    try {
      const matrixService = require('../services/MatrixService').matrixService;
      matrixService.on('roomJoined', handleRoomJoined);
      
      return () => {
        matrixService.off('roomJoined', handleRoomJoined);
      };
    } catch (error) {
      console.warn('⚠️ useMatrixHistory: Could not set up Matrix event listeners:', error);
    }
  }, [isConnected, isReady, loadMatrixSessions]);

  return {
    matrixSessions,
    isLoading,
    isInitialized,
    error,
    refreshMatrixHistory,
    getSessionIdForRoom,
    isMatrixSession,
    getMatrixRoomIdForSession,
  };
};
