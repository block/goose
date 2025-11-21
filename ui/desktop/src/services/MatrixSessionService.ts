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
  private isInitialized = false;
  private lastSyncTime = 0;
  private cachedSessions: Session[] = [];
  private syncInProgress = false;
  private readonly CACHE_DURATION = 5 * 60 * 1000; // 5 minutes cache
  private readonly SYNC_COOLDOWN = 30 * 1000; // 30 seconds between syncs

  private constructor() {}

  public static getInstance(): MatrixSessionService {
    if (!MatrixSessionService.instance) {
      MatrixSessionService.instance = new MatrixSessionService();
    }
    return MatrixSessionService.instance;
  }

  /**
   * Get all Matrix rooms and convert them to Session format (with intelligent caching)
   */
  public async getMatrixSessions(): Promise<Session[]> {
    try {
      // Check if Matrix service is connected OR syncing (both states mean we have rooms)
      const connectionStatus = matrixService.getConnectionStatus();
      console.log('🔍 MatrixSessionService.getMatrixSessions() called - connection status:', {
        connected: connectionStatus.connected,
        syncState: connectionStatus.syncState,
      });
      
      const isUsable = connectionStatus.connected || connectionStatus.syncState === 'SYNCING' || connectionStatus.syncState === 'PREPARED';
      
      if (!isUsable) {
        console.log('❌ Matrix service not ready (state:', connectionStatus.syncState, '), skipping Matrix sessions');
        return [];
      }
      
      console.log('✅ Matrix service ready (connected:', connectionStatus.connected, ', syncState:', connectionStatus.syncState, ')');


      // Check if we have valid cached sessions
      const now = Date.now();
      if (this.cachedSessions.length > 0 && (now - this.lastSyncTime) < this.CACHE_DURATION) {
        console.log('📋 Returning cached Matrix sessions (', this.cachedSessions.length, 'sessions)');
        return this.cachedSessions;
      }

      // Check if sync is already in progress
      if (this.syncInProgress) {
        console.log('📋 Matrix session sync already in progress, returning cached sessions');
        return this.cachedSessions;
      }

      // Check sync cooldown to prevent too frequent syncs
      if ((now - this.lastSyncTime) < this.SYNC_COOLDOWN) {
        console.log('📋 Matrix session sync in cooldown, returning cached sessions');
        return this.cachedSessions;
      }

      // Perform the sync
      this.syncInProgress = true;
      console.log('📋 Starting Matrix session sync...');

      const rooms = matrixService.getRooms();
      const matrixSessions: Session[] = [];

      console.log('📋 Found', rooms.length, 'Matrix rooms, checking for new mappings...');

      // Check which rooms need new mappings (optimization)
      const roomsNeedingMappings = rooms.filter(room => !sessionMappingService.getMapping(room.roomId));
      
      if (roomsNeedingMappings.length > 0) {
        console.log('📋 Creating mappings for', roomsNeedingMappings.length, 'new Matrix rooms...');
        
        // Only create mappings for rooms that don't have them
        await this.createMappingsForNewRooms(roomsNeedingMappings);
      } else {
        console.log('📋 All Matrix rooms already have mappings, skipping mapping creation');
      }

      // Now process all rooms (this is much faster since mappings exist)
      for (const room of rooms) {
        const mapping = sessionMappingService.getMapping(room.roomId);
        if (!mapping) {
          console.warn('📋 Skipping room without mapping:', room.roomId);
          continue;
        }

        try {
          // OPTIMIZATION: Don't load full history for list view - it's too slow!
          // Just get a rough message count from the room object
          // Full history will be loaded when user clicks into the session
          const history: any[] = []; // Empty for list view
          const messageCount = room.lastActivity ? 1 : 0; // Rough estimate
          
          // Sync room history to backend session if we have a backend session ID
          if (mapping.gooseSessionId && history.length > 0) {
            try {
              console.log('📜 Syncing Matrix room history to backend session:', {
                roomId: room.roomId.substring(0, 20) + '...',
                sessionId: mapping.gooseSessionId,
                messageCount: history.length
              });
              
              // Import the API function to sync messages to backend
              const { replyHandler } = await import('../api');
              
              // Convert Matrix messages to backend format
              const backendMessages = history.map((msg, index) => ({
                id: `matrix_${msg.timestamp.getTime()}_${index}`,
                role: msg.role,
                content: [{
                  type: 'text' as const,
                  text: msg.content,
                }],
                created: Math.floor(msg.timestamp.getTime() / 1000),
                // Include sender info if available
                ...(msg.sender && { 
                  sender: {
                    userId: msg.metadata?.senderInfo?.userId || msg.sender,
                    displayName: msg.metadata?.senderInfo?.displayName || msg.sender.split(':')[0].substring(1),
                    avatarUrl: msg.metadata?.senderInfo?.avatarUrl || null,
                  }
                })
              }));
              
              // Sync messages to backend session
              await replyHandler({
                body: {
                  session_id: mapping.gooseSessionId,
                  messages: backendMessages,
                },
                throwOnError: false, // Don't throw on error to prevent breaking the session list
              });
              
              console.log('✅ Successfully synced Matrix room history to backend session');
            } catch (syncError) {
              console.warn('⚠️ Failed to sync Matrix room history to backend session:', syncError);
              // Don't fail the entire session creation if sync fails
            }
          }
          
          // Enhanced room type detection for collaborative sessions
          const isCollaborativeSession = this.isCollaborativeSession(room);
          const roomType = room.isDirectMessage 
            ? 'dm' 
            : isCollaborativeSession 
              ? 'collaborative' 
              : 'group';
              
          const generatedTitle = await llmTitleGenerationService.generateRoomTitle(room.roomId, {
            roomType,
            fallbackName: room.name || mapping.title,
            maxMessages: 15,
            includeParticipants: true,
          });
          
          // Determine working directory based on room type with collaborative distinction
          const workingDir = room.isDirectMessage 
            ? 'Direct Message' 
            : isCollaborativeSession
              ? 'Collaborative AI Session'
              : room.members.length > 2 
                ? 'Group Chat'
                : 'Matrix Room';
          
          // Convert Matrix room to Session format with enhanced collaborative metadata
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
                isCollaborativeSession: isCollaborativeSession,
                backendSessionId: mapping.gooseSessionId,
                matrixRoomName: room.name,
                participantCount: room.members.length,
                generatedTitle: generatedTitle,
                roomType: roomType,
                // Enhanced collaborative session metadata
                collaborativeMetadata: isCollaborativeSession ? {
                  sessionType: 'ai_collaboration',
                  createdViaInvite: this.wasCreatedViaInvite(room),
                  hasGooseParticipant: this.hasGooseParticipant(room),
                  collaborationLevel: this.getCollaborationLevel(room),
                  workflowType: this.detectWorkflowType(room, history),
                } : undefined,
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
            hasExtensionData: !!session.extension_data?.matrix,
            extensionDataKeys: session.extension_data?.matrix ? Object.keys(session.extension_data.matrix) : [],
          });
        } catch (error) {
          console.warn('📋 Failed to convert Matrix room to session:', room.roomId, error);
          // Continue with other rooms even if one fails
        }
      }

      // Cache the results and update sync time
      this.cachedSessions = matrixSessions;
      this.lastSyncTime = now;
      this.isInitialized = true;

      console.log('📋 Successfully synced', matrixSessions.length, 'Matrix rooms to sessions (cached for', this.CACHE_DURATION / 1000 / 60, 'minutes)');
      return matrixSessions;
    } catch (error) {
      console.error('📋 Error getting Matrix sessions:', error);
      return this.cachedSessions; // Return cached sessions on error
    } finally {
      this.syncInProgress = false;
    }
  }

  /**
   * Create mappings for new Matrix rooms (batch operation for efficiency)
   */
  private async createMappingsForNewRooms(rooms: any[]): Promise<void> {
    const currentUserId = matrixService.getCurrentUser()?.userId;
    
    for (const room of rooms) {
      try {
        console.log('📋 Creating session mapping for new Matrix room:', room.roomId.substring(0, 20) + '...');
        
        // Determine room name and participants
        const participants = room.members.map((m: any) => m.userId);
        
        let roomName: string;
        if (room.name) {
          roomName = room.name;
        } else if (room.isDirectMessage) {
          // For DMs, create a name based on the other participant
          const otherParticipant = room.members.find((m: any) => m.userId !== currentUserId);
          const otherName = otherParticipant?.displayName || otherParticipant?.userId?.split(':')[0].substring(1) || 'Unknown';
          roomName = `DM with ${otherName}`;
        } else {
          roomName = `Matrix Room ${room.roomId.substring(1, 8)}`;
        }

        try {
          // Create mapping with backend session
          await sessionMappingService.createMappingWithBackendSession(
            room.roomId,
            participants,
            roomName
          );
          console.log('✅ Created backend session mapping for new Matrix room:', {
            roomId: room.roomId.substring(0, 20) + '...',
            roomName
          });
        } catch (error) {
          console.error('❌ Failed to create backend session for Matrix room, using fallback:', error);
          // Fallback to regular mapping
          sessionMappingService.createMapping(room.roomId, participants, roomName);
          console.log('📋 Created fallback mapping for new Matrix room:', {
            roomId: room.roomId.substring(0, 20) + '...',
            roomName
          });
        }
      } catch (error) {
        console.error('❌ Failed to create mapping for room:', room.roomId, error);
        // Continue with other rooms
      }
    }
  }

  /**
   * Force refresh Matrix sessions (clears cache and re-syncs)
   */
  public async forceRefresh(): Promise<Session[]> {
    console.log('📋 Force refreshing Matrix sessions...');
    this.cachedSessions = [];
    this.lastSyncTime = 0;
    this.isInitialized = false;
    return await this.getMatrixSessions();
  }

  /**
   * Invalidate cache (for when new rooms are joined or left)
   */
  public invalidateCache(): void {
    console.log('📋 Invalidating Matrix sessions cache...');
    this.cachedSessions = [];
    this.lastSyncTime = 0;
  }

  /**
   * Determine if a Matrix room is a true collaborative session
   */
  private isCollaborativeSession(room: any): boolean {
    // Criteria for collaborative sessions:
    // 1. More than 2 participants (not a DM)
    // 2. Has explicit room name (indicates intentional creation)
    // 3. Has Goose participant or AI-related content
    // 4. Created via invitation/sharing (not just a random group chat)
    
    if (room.isDirectMessage) {
      return false; // DMs are never collaborative sessions
    }
    
    if (room.members.length <= 2) {
      return false; // Need more than 2 people for collaboration
    }
    
    // Check if room has explicit name (indicates intentional creation)
    const hasExplicitName = room.name && room.name.trim() !== '';
    
    // Check if room was created for AI/Goose collaboration
    const hasAIIndicators = this.hasAICollaborationIndicators(room);
    
    // Check if room was created via invitation (not just random group)
    const wasCreatedViaInvite = this.wasCreatedViaInvite(room);
    
    // A room is collaborative if it meets at least 2 of these criteria:
    const criteria = [hasExplicitName, hasAIIndicators, wasCreatedViaInvite];
    const metCriteria = criteria.filter(Boolean).length;
    
    return metCriteria >= 2;
  }

  /**
   * Check if room has AI/Goose collaboration indicators
   */
  private hasAICollaborationIndicators(room: any): boolean {
    // Check room name for AI/collaboration keywords
    const aiKeywords = ['goose', 'ai', 'collaboration', 'session', 'project', 'work'];
    const roomName = (room.name || '').toLowerCase();
    const hasAIName = aiKeywords.some(keyword => roomName.includes(keyword));
    
    // Check if there's a Goose participant
    const hasGooseParticipant = this.hasGooseParticipant(room);
    
    // Check room topic for AI indicators
    const roomTopic = (room.topic || '').toLowerCase();
    const hasAITopic = aiKeywords.some(keyword => roomTopic.includes(keyword));
    
    return hasAIName || hasGooseParticipant || hasAITopic;
  }

  /**
   * Check if room has a Goose participant
   */
  private hasGooseParticipant(room: any): boolean {
    return room.members.some((member: any) => {
      const userId = (member.userId || '').toLowerCase();
      const displayName = (member.displayName || '').toLowerCase();
      
      // Check for Goose-related user IDs or display names
      const gooseIndicators = ['goose', 'bot', 'ai', 'assistant'];
      return gooseIndicators.some(indicator => 
        userId.includes(indicator) || displayName.includes(indicator)
      );
    });
  }

  /**
   * Check if room was created via invitation/sharing
   */
  private wasCreatedViaInvite(room: any): boolean {
    // This is a heuristic - in a full implementation, you'd check the room's creation events
    // For now, we assume rooms with explicit names and multiple participants were created intentionally
    const hasExplicitName = room.name && room.name.trim() !== '';
    const hasMultipleParticipants = room.members.length > 2;
    
    // Check if current user was invited (not the creator)
    const currentUserId = matrixService.getCurrentUser()?.userId;
    const currentUserMember = room.members.find((m: any) => m.userId === currentUserId);
    
    // If we can't determine membership details, assume it was via invite if it has a name
    return hasExplicitName && hasMultipleParticipants;
  }

  /**
   * Determine collaboration level
   */
  private getCollaborationLevel(room: any): 'light' | 'medium' | 'intensive' {
    const participantCount = room.members.length;
    const hasGoose = this.hasGooseParticipant(room);
    
    if (participantCount >= 5 || hasGoose) {
      return 'intensive'; // Large groups or AI-assisted
    } else if (participantCount >= 3) {
      return 'medium'; // Small groups
    } else {
      return 'light'; // Minimal collaboration
    }
  }

  /**
   * Detect workflow type based on room and message history
   */
  private detectWorkflowType(room: any, history: any[]): 'brainstorming' | 'project_work' | 'code_review' | 'general' {
    const roomName = (room.name || '').toLowerCase();
    const hasGoose = this.hasGooseParticipant(room);
    
    // Analyze room name for workflow indicators
    if (roomName.includes('brainstorm') || roomName.includes('idea')) {
      return 'brainstorming';
    }
    
    if (roomName.includes('project') || roomName.includes('work') || roomName.includes('task')) {
      return 'project_work';
    }
    
    if (roomName.includes('review') || roomName.includes('code') || hasGoose) {
      return 'code_review';
    }
    
    // Analyze message history for patterns (simplified)
    const messageContent = history.map(msg => msg.content.toLowerCase()).join(' ');
    
    if (messageContent.includes('code') || messageContent.includes('function') || messageContent.includes('bug')) {
      return 'code_review';
    }
    
    if (messageContent.includes('project') || messageContent.includes('task') || messageContent.includes('deadline')) {
      return 'project_work';
    }
    
    if (messageContent.includes('idea') || messageContent.includes('think') || messageContent.includes('brainstorm')) {
      return 'brainstorming';
    }
    
    return 'general';
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
