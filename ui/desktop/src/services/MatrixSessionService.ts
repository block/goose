/**
 * MatrixSessionService - Converts Matrix rooms to Session format for unified session management
 * 
 * This service enables Matrix collaborative sessions to appear in the regular session history
 * alongside solo Goose sessions, providing a unified experience for users.
 */

import { Session, Message, Conversation } from '../api/types.gen';
import { matrixService } from './MatrixService';
import { sessionMappingService } from './SessionMappingService';
import { llmTitleGenerationService } from './LLMTitleGenerationService';

export interface MatrixSessionInfo {
  roomId: string;
  roomName: string;
  lastActivity: Date;
  messageCount: number;
  participants: string[];
  isCollaborative: boolean;
}

export class MatrixSessionService {
  private static instance: MatrixSessionService;

  private constructor() {}

  public static getInstance(): MatrixSessionService {
    if (!MatrixSessionService.instance) {
      MatrixSessionService.instance = new MatrixSessionService();
    }
    return MatrixSessionService.instance;
  }

  /**
   * Get all Matrix rooms that have session mappings and convert them to Session format
   */
  public async getMatrixSessions(): Promise<Session[]> {
    try {
      // Only return Matrix sessions if Matrix service is connected
      const connectionStatus = matrixService.getConnectionStatus();
      if (!connectionStatus.connected) {
        console.log('📋 Matrix service not connected, skipping Matrix sessions');
        return [];
      }

      const rooms = matrixService.getRooms();
      const mappings = sessionMappingService.getAllMappings();
      const matrixSessions: Session[] = [];

      console.log('📋 Found', rooms.length, 'Matrix rooms and', mappings.length, 'session mappings');

      for (const room of rooms) {
        const mapping = sessionMappingService.getMapping(room.roomId);
        if (!mapping) {
          // Skip rooms without session mappings
          continue;
        }

        try {
          // Get room history to calculate message count and create conversation
          const history = await matrixService.getRoomHistoryAsGooseMessages(room.roomId, 100);
          
          // Generate contextual title for the room
          const roomType = room.isDirectMessage ? 'dm' : (room.members.length > 2 ? 'collaborative' : 'group');
          const generatedTitle = await llmTitleGenerationService.generateRoomTitle(room.roomId, {
            roomType,
            fallbackName: room.name || mapping.title,
            maxMessages: 15,
            includeParticipants: true,
          });
          
          // Determine working directory based on room type
          const workingDir = room.isDirectMessage 
            ? 'Direct Message' 
            : room.members.length > 2 
              ? 'Collaborative Session' 
              : 'Group Chat';
          
          // Convert Matrix room to Session format
          const session: Session = {
            id: room.roomId, // Use Matrix room ID as session ID for UI purposes
            description: generatedTitle.title,
            working_dir: workingDir,
            message_count: history.length,
            total_tokens: null, // Matrix sessions don't have token tracking yet
            created_at: new Date(mapping.createdAt).toISOString(),
            updated_at: new Date(mapping.lastUsed).toISOString(),
            conversation: this.convertMatrixHistoryToConversation(history),
            extension_data: {
              matrix: {
                roomId: room.roomId,
                participants: room.members.map(m => m.userId),
                isDirectMessage: room.isDirectMessage,
                backendSessionId: mapping.gooseSessionId,
                matrixRoomName: room.name,
                participantCount: room.members.length,
                generatedTitle: generatedTitle,
                roomType: roomType,
              }
            },
            accumulated_input_tokens: null,
            accumulated_output_tokens: null,
            accumulated_total_tokens: null,
            input_tokens: null,
            output_tokens: null,
            recipe: null,
            schedule_id: null,
          };

          matrixSessions.push(session);
          console.log('📋 Converted Matrix room to session:', {
            roomId: room.roomId,
            sessionId: session.id,
            messageCount: session.message_count,
            participants: room.members.length,
          });
        } catch (error) {
          console.warn('📋 Failed to convert Matrix room to session:', room.roomId, error);
          // Continue with other rooms even if one fails
        }
      }

      console.log('📋 Successfully converted', matrixSessions.length, 'Matrix rooms to sessions');
      return matrixSessions;
    } catch (error) {
      console.error('📋 Error getting Matrix sessions:', error);
      return [];
    }
  }

  /**
   * Convert Matrix message history to Goose Conversation format
   */
  private convertMatrixHistoryToConversation(history: Array<{
    role: 'user' | 'assistant' | 'system';
    content: string;
    timestamp: Date;
    sender?: string;
    metadata?: Record<string, any>;
  }>): Conversation {
    return history.map((msg, index) => {
      const message: Message = {
        id: `matrix_msg_${index}_${msg.timestamp.getTime()}`,
        role: msg.role,
        content: [
          {
            type: 'text',
            text: msg.content,
          }
        ],
        created: Math.floor(msg.timestamp.getTime() / 1000),
        metadata: {
          userVisible: true,
          agentVisible: true,
          matrixSender: msg.sender,
          matrixMetadata: msg.metadata,
        },
      };
      return message;
    });
  }

  /**
   * Check if a session ID represents a Matrix session
   */
  public isMatrixSession(sessionId: string): boolean {
    return sessionId.startsWith('!') && sessionId.includes(':');
  }

  /**
   * Get Matrix session info for a specific room ID
   */
  public async getMatrixSessionInfo(roomId: string): Promise<MatrixSessionInfo | null> {
    try {
      const rooms = matrixService.getRooms();
      const room = rooms.find(r => r.roomId === roomId);
      
      if (!room) {
        return null;
      }

      const history = await matrixService.getRoomHistory(roomId, 50);
      
      return {
        roomId: room.roomId,
        roomName: room.name || `Room ${roomId.substring(1, 8)}`,
        lastActivity: room.lastActivity || new Date(),
        messageCount: history.length,
        participants: room.members.map(m => m.userId),
        isCollaborative: room.members.length > 2,
      };
    } catch (error) {
      console.error('📋 Error getting Matrix session info:', error);
      return null;
    }
  }

  /**
   * Load a Matrix session by room ID, ensuring it has proper mapping
   */
  public async loadMatrixSession(roomId: string): Promise<Session | null> {
    try {
      // Ensure session mapping exists
      const mapping = sessionMappingService.getMapping(roomId);
      if (!mapping) {
        console.log('📋 No mapping found for Matrix room, creating one:', roomId);
        const rooms = matrixService.getRooms();
        const room = rooms.find(r => r.roomId === roomId);
        const roomName = room?.name || `Matrix Room ${roomId.substring(1, 8)}`;
        sessionMappingService.ensureMappingExists(roomId, roomName);
      }

      // Get the Matrix session
      const matrixSessions = await this.getMatrixSessions();
      const session = matrixSessions.find(s => s.id === roomId);
      
      if (session) {
        console.log('📋 Successfully loaded Matrix session:', roomId);
        return session;
      } else {
        console.warn('📋 Matrix session not found:', roomId);
        return null;
      }
    } catch (error) {
      console.error('📋 Error loading Matrix session:', error);
      return null;
    }
  }

  /**
   * Get session statistics for Matrix sessions
   */
  public async getMatrixSessionStats(): Promise<{
    totalSessions: number;
    totalMessages: number;
    collaborativeSessions: number;
  }> {
    try {
      const sessions = await this.getMatrixSessions();
      
      return {
        totalSessions: sessions.length,
        totalMessages: sessions.reduce((sum, s) => sum + s.message_count, 0),
        collaborativeSessions: sessions.filter(s => 
          s.extension_data?.matrix?.participantCount > 2
        ).length,
      };
    } catch (error) {
      console.error('📋 Error getting Matrix session stats:', error);
      return {
        totalSessions: 0,
        totalMessages: 0,
        collaborativeSessions: 0,
      };
    }
  }

  /**
   * Create a new Matrix session (room) and ensure proper mapping
   */
  public async createMatrixSession(name: string, inviteUserIds: string[] = []): Promise<string> {
    try {
      // Create the Matrix room
      const roomId = await matrixService.createAISession(name, inviteUserIds);
      
      // The MatrixService.createAISession already creates a session mapping with backend session
      console.log('📋 Created new Matrix session:', {
        roomId,
        name,
        inviteCount: inviteUserIds.length,
      });
      
      return roomId;
    } catch (error) {
      console.error('📋 Error creating Matrix session:', error);
      throw error;
    }
  }

  /**
   * Delete a Matrix session (leave the room and remove mapping)
   */
  public async deleteMatrixSession(roomId: string): Promise<void> {
    try {
      // Note: Matrix SDK doesn't have a direct "leave room" method in our current setup
      // For now, we'll just remove the session mapping
      // In a full implementation, you'd want to call matrixService.leaveRoom(roomId)
      
      // Remove the session mapping
      const mapping = sessionMappingService.getMapping(roomId);
      if (mapping) {
        // Clear the specific mapping (we'd need to add this method to SessionMappingService)
        console.log('📋 Would remove Matrix session mapping for:', roomId);
        // sessionMappingService.removeMapping(roomId); // TODO: implement this method
      }
      
      console.log('📋 Matrix session deletion requested for:', roomId);
    } catch (error) {
      console.error('📋 Error deleting Matrix session:', error);
      throw error;
    }
  }

  /**
   * Update Matrix session description (room name)
   */
  public async updateMatrixSessionDescription(roomId: string, newDescription: string): Promise<void> {
    try {
      // Update the session mapping title
      sessionMappingService.updateTitle(roomId, newDescription);
      
      // Note: To fully implement this, you'd also want to update the Matrix room name
      // This would require adding a method to MatrixService like:
      // await matrixService.setRoomName(roomId, newDescription);
      
      console.log('📋 Updated Matrix session description:', {
        roomId,
        newDescription,
      });
    } catch (error) {
      console.error('📋 Error updating Matrix session description:', error);
      throw error;
    }
  }

  /**
   * Regenerate title for a Matrix session using LLM analysis
   */
  public async regenerateSessionTitle(roomId: string): Promise<string | null> {
    try {
      const rooms = matrixService.getRooms();
      const room = rooms.find(r => r.roomId === roomId);
      
      if (!room) {
        console.warn('📋 Room not found for title regeneration:', roomId);
        return null;
      }

      // Determine room type
      const roomType = room.isDirectMessage ? 'dm' : (room.members.length > 2 ? 'collaborative' : 'group');
      
      // Force regenerate title
      const generatedTitle = await llmTitleGenerationService.regenerateTitle(roomId, {
        roomType,
        maxMessages: 20,
        includeParticipants: true,
      });

      // Update the session mapping with new title
      sessionMappingService.updateTitle(roomId, generatedTitle.title);

      console.log('📋 Regenerated title for Matrix session:', {
        roomId,
        oldTitle: room.name,
        newTitle: generatedTitle.title,
        confidence: generatedTitle.confidence,
        source: generatedTitle.source,
      });

      return generatedTitle.title;
    } catch (error) {
      console.error('📋 Error regenerating session title:', error);
      return null;
    }
  }

  /**
   * Get enhanced session display info with generated titles
   */
  public async getEnhancedSessionDisplayInfo(roomId: string): Promise<{
    title: string;
    roomType: 'dm' | 'group' | 'collaborative';
    workingDir: string;
    participants: string[];
    titleInfo?: {
      confidence: 'high' | 'medium' | 'low';
      source: 'llm' | 'content_analysis' | 'fallback';
      generatedAt: number;
    };
  } | null> {
    try {
      const rooms = matrixService.getRooms();
      const room = rooms.find(r => r.roomId === roomId);
      
      if (!room) {
        return null;
      }

      const roomType = room.isDirectMessage ? 'dm' : (room.members.length > 2 ? 'collaborative' : 'group');
      const generatedTitle = await llmTitleGenerationService.generateRoomTitle(roomId, { roomType });
      
      const workingDir = room.isDirectMessage 
        ? 'Direct Message' 
        : room.members.length > 2 
          ? 'Collaborative Session' 
          : 'Group Chat';

      return {
        title: generatedTitle.title,
        roomType: generatedTitle.roomType,
        workingDir,
        participants: room.members.map(m => m.userId),
        titleInfo: {
          confidence: generatedTitle.confidence,
          source: generatedTitle.source,
          generatedAt: generatedTitle.generatedAt,
        },
      };
    } catch (error) {
      console.error('📋 Error getting enhanced session display info:', error);
      return null;
    }
  }
}

// Export singleton instance
export const matrixSessionService = MatrixSessionService.getInstance();
