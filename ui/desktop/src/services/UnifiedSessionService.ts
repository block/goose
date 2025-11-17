/**
 * UnifiedSessionService - Provides a unified interface for both regular Goose sessions and Matrix sessions
 * 
 * This service combines regular backend sessions with Matrix collaborative sessions,
 * allowing them to be displayed together in the session history UI.
 */

import { Session, listSessions, getSession, deleteSession, updateSessionDescription } from '../api';
import { matrixSessionService } from './MatrixSessionService';
import { sessionMappingService } from './SessionMappingService';

export interface UnifiedSessionListResponse {
  sessions: Session[];
  matrixSessionCount: number;
  regularSessionCount: number;
}

export class UnifiedSessionService {
  private static instance: UnifiedSessionService;

  private constructor() {}

  public static getInstance(): UnifiedSessionService {
    if (!UnifiedSessionService.instance) {
      UnifiedSessionService.instance = new UnifiedSessionService();
    }
    return UnifiedSessionService.instance;
  }

  /**
   * Get all sessions (both regular and Matrix) in a unified list
   */
  public async getAllSessions(): Promise<UnifiedSessionListResponse> {
    try {
      console.log('📋 UnifiedSessionService: Fetching all sessions...');

      // Fetch regular sessions and Matrix sessions in parallel
      const [regularSessionsResponse, matrixSessions] = await Promise.all([
        listSessions<true>({ throwOnError: false }),
        matrixSessionService.getMatrixSessions(),
      ]);

      const regularSessions = regularSessionsResponse.data?.sessions || [];
      
      // Combine sessions and sort by updated_at (most recent first)
      const allSessions = [...regularSessions, ...matrixSessions].sort((a, b) => {
        const dateA = new Date(a.updated_at).getTime();
        const dateB = new Date(b.updated_at).getTime();
        return dateB - dateA; // Most recent first
      });

      console.log('📋 UnifiedSessionService: Combined sessions:', {
        regular: regularSessions.length,
        matrix: matrixSessions.length,
        total: allSessions.length,
      });

      return {
        sessions: allSessions,
        matrixSessionCount: matrixSessions.length,
        regularSessionCount: regularSessions.length,
      };
    } catch (error) {
      console.error('📋 UnifiedSessionService: Error fetching sessions:', error);
      
      // Fallback to regular sessions only if there's an error
      try {
        const regularSessionsResponse = await listSessions<true>({ throwOnError: false });
        const regularSessions = regularSessionsResponse.data?.sessions || [];
        
        return {
          sessions: regularSessions,
          matrixSessionCount: 0,
          regularSessionCount: regularSessions.length,
        };
      } catch (fallbackError) {
        console.error('📋 UnifiedSessionService: Fallback also failed:', fallbackError);
        return {
          sessions: [],
          matrixSessionCount: 0,
          regularSessionCount: 0,
        };
      }
    }
  }

  /**
   * Get a specific session by ID (handles both regular and Matrix sessions)
   */
  public async getSessionById(sessionId: string): Promise<Session | null> {
    try {
      console.log('📋 UnifiedSessionService: Fetching session:', sessionId);

      // Check if it's a Matrix session
      if (matrixSessionService.isMatrixSession(sessionId)) {
        console.log('📋 Loading Matrix session:', sessionId);
        return await matrixSessionService.loadMatrixSession(sessionId);
      } else {
        console.log('📋 Loading regular session:', sessionId);
        // Regular backend session
        const response = await getSession<true>({
          path: { session_id: sessionId },
          throwOnError: false,
        });
        
        if (response.data) {
          return response.data;
        } else {
          console.warn('📋 Regular session not found:', sessionId);
          return null;
        }
      }
    } catch (error) {
      console.error('📋 UnifiedSessionService: Error fetching session:', sessionId, error);
      return null;
    }
  }

  /**
   * Delete a session (handles both regular and Matrix sessions)
   */
  public async deleteSessionById(sessionId: string): Promise<void> {
    try {
      console.log('📋 UnifiedSessionService: Deleting session:', sessionId);

      if (matrixSessionService.isMatrixSession(sessionId)) {
        console.log('📋 Deleting Matrix session:', sessionId);
        await matrixSessionService.deleteMatrixSession(sessionId);
      } else {
        console.log('📋 Deleting regular session:', sessionId);
        await deleteSession({
          path: { session_id: sessionId },
          throwOnError: true,
        });
      }
    } catch (error) {
      console.error('📋 UnifiedSessionService: Error deleting session:', sessionId, error);
      throw error;
    }
  }

  /**
   * Update session description (handles both regular and Matrix sessions)
   */
  public async updateSessionDescriptionById(sessionId: string, newDescription: string): Promise<void> {
    try {
      console.log('📋 UnifiedSessionService: Updating session description:', sessionId, newDescription);

      if (matrixSessionService.isMatrixSession(sessionId)) {
        console.log('📋 Updating Matrix session description:', sessionId);
        await matrixSessionService.updateMatrixSessionDescription(sessionId, newDescription);
      } else {
        console.log('📋 Updating regular session description:', sessionId);
        await updateSessionDescription({
          path: { session_id: sessionId },
          body: { description: newDescription },
          throwOnError: true,
        });
      }
    } catch (error) {
      console.error('📋 UnifiedSessionService: Error updating session description:', sessionId, error);
      throw error;
    }
  }

  /**
   * Get session type for UI display purposes
   */
  public getSessionType(session: Session): 'regular' | 'matrix' | 'collaborative' {
    if (matrixSessionService.isMatrixSession(session.id)) {
      const participantCount = session.extension_data?.matrix?.participantCount || 0;
      return participantCount > 2 ? 'collaborative' : 'matrix';
    }
    return 'regular';
  }

  /**
   * Get session display info with type-specific details
   */
  public getSessionDisplayInfo(session: Session): {
    type: 'regular' | 'matrix' | 'collaborative';
    displayName: string;
    workingDir: string;
    participants?: string[];
    isCollaborative: boolean;
    hasTokenCounts: boolean;
    roomType?: 'dm' | 'group' | 'collaborative';
    titleInfo?: {
      confidence: 'high' | 'medium' | 'low';
      source: 'llm' | 'content_analysis' | 'fallback';
      generatedAt: number;
    };
  } {
    const type = this.getSessionType(session);
    const isMatrix = type === 'matrix' || type === 'collaborative';
    
    // Enhanced display info for Matrix sessions
    if (isMatrix && session.extension_data?.matrix) {
      const matrixData = session.extension_data.matrix;
      
      return {
        type,
        displayName: session.description,
        workingDir: session.working_dir, // Now uses the enhanced working dir from MatrixSessionService
        participants: matrixData.participants,
        isCollaborative: type === 'collaborative',
        hasTokenCounts: false, // Matrix sessions don't have token tracking
        roomType: matrixData.roomType || (matrixData.isDirectMessage ? 'dm' : 'group'),
        titleInfo: matrixData.generatedTitle ? {
          confidence: matrixData.generatedTitle.confidence,
          source: matrixData.generatedTitle.source,
          generatedAt: matrixData.generatedTitle.generatedAt,
        } : undefined,
      };
    }
    
    // Regular session display info
    return {
      type,
      displayName: session.description,
      workingDir: session.working_dir,
      participants: undefined,
      isCollaborative: false,
      hasTokenCounts: session.total_tokens !== null,
    };
  }

  /**
   * Create a new Matrix collaborative session
   */
  public async createMatrixSession(name: string, inviteUserIds: string[] = []): Promise<string> {
    return await matrixSessionService.createMatrixSession(name, inviteUserIds);
  }

  /**
   * Regenerate title for a Matrix session
   */
  public async regenerateSessionTitle(sessionId: string): Promise<string | null> {
    if (matrixSessionService.isMatrixSession(sessionId)) {
      return await matrixSessionService.regenerateSessionTitle(sessionId);
    } else {
      console.warn('📋 UnifiedSessionService: Cannot regenerate title for non-Matrix session:', sessionId);
      return null;
    }
  }

  /**
   * Get unified session statistics
   */
  public async getSessionStats(): Promise<{
    totalSessions: number;
    regularSessions: number;
    matrixSessions: number;
    collaborativeSessions: number;
    totalTokens: number;
    totalMessages: number;
  }> {
    try {
      const { sessions, matrixSessionCount, regularSessionCount } = await this.getAllSessions();
      const matrixStats = await matrixSessionService.getMatrixSessionStats();
      
      const totalTokens = sessions
        .filter(s => !matrixSessionService.isMatrixSession(s.id))
        .reduce((sum, s) => sum + (s.total_tokens || 0), 0);
      
      const totalMessages = sessions.reduce((sum, s) => sum + s.message_count, 0);
      
      return {
        totalSessions: sessions.length,
        regularSessions: regularSessionCount,
        matrixSessions: matrixSessionCount,
        collaborativeSessions: matrixStats.collaborativeSessions,
        totalTokens,
        totalMessages,
      };
    } catch (error) {
      console.error('📋 UnifiedSessionService: Error getting session stats:', error);
      return {
        totalSessions: 0,
        regularSessions: 0,
        matrixSessions: 0,
        collaborativeSessions: 0,
        totalTokens: 0,
        totalMessages: 0,
      };
    }
  }

  /**
   * Check if Matrix integration is available and connected
   */
  public isMatrixAvailable(): boolean {
    try {
      const matrixService = require('./MatrixService').matrixService;
      const connectionStatus = matrixService.getConnectionStatus();
      return connectionStatus.connected;
    } catch (error) {
      return false;
    }
  }
}

// Export singleton instance
export const unifiedSessionService = UnifiedSessionService.getInstance();
