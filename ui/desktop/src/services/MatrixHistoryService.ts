/**
 * MatrixHistoryService - Syncs Matrix rooms to chat history
 * 
 * This service pulls Matrix rooms and creates corresponding sessions in the chat history,
 * using the hybrid approach where Matrix rooms are mapped to backend session IDs.
 */

import { matrixService } from './MatrixService';
import { sessionMappingService } from './SessionMappingService';

export interface MatrixHistorySession {
  sessionId: string;
  matrixRoomId: string;
  title: string;
  lastActivity: Date;
  messageCount: number;
  participants: string[];
  isDirectMessage: boolean;
  roomType: 'dm' | 'group' | 'collaborative';
}

export class MatrixHistoryService {
  private isInitialized = false;
  private syncInProgress = false;

  /**
   * Initialize the service and perform initial sync
   */
  async initialize(): Promise<void> {
    if (this.isInitialized) {
      return;
    }

    console.log('🔄 MatrixHistoryService: Initializing...');
    
    // Wait for Matrix service to be ready
    if (!matrixService.getConnectionStatus().connected) {
      console.log('🔄 MatrixHistoryService: Waiting for Matrix connection...');
      
      return new Promise((resolve) => {
        const checkConnection = () => {
          if (matrixService.getConnectionStatus().connected) {
            console.log('✅ MatrixHistoryService: Matrix connected, proceeding with initialization');
            this.performInitialSync().then(() => {
              this.isInitialized = true;
              resolve();
            });
          } else {
            setTimeout(checkConnection, 1000);
          }
        };
        checkConnection();
      });
    }

    await this.performInitialSync();
    this.isInitialized = true;
    console.log('✅ MatrixHistoryService: Initialized successfully');
  }

  /**
   * Perform initial sync of Matrix rooms to chat history
   */
  private async performInitialSync(): Promise<void> {
    if (this.syncInProgress) {
      console.log('⚠️ MatrixHistoryService: Sync already in progress, skipping');
      return;
    }

    this.syncInProgress = true;
    console.log('🔄 MatrixHistoryService: Starting initial sync of Matrix rooms to chat history...');

    try {
      // Get all Matrix rooms
      const matrixRooms = matrixService.getRooms();
      console.log(`📋 MatrixHistoryService: Found ${matrixRooms.length} Matrix rooms to sync`);

      let syncedCount = 0;
      let skippedCount = 0;
      let errorCount = 0;

      for (const room of matrixRooms) {
        try {
          await this.syncRoomToHistory(room);
          syncedCount++;
        } catch (error) {
          console.error(`❌ MatrixHistoryService: Failed to sync room ${room.roomId}:`, error);
          errorCount++;
        }
      }

      console.log(`✅ MatrixHistoryService: Initial sync complete - ${syncedCount} synced, ${skippedCount} skipped, ${errorCount} errors`);
    } catch (error) {
      console.error('❌ MatrixHistoryService: Initial sync failed:', error);
    } finally {
      this.syncInProgress = false;
    }
  }

  /**
   * Sync a single Matrix room to chat history
   */
  private async syncRoomToHistory(room: any): Promise<void> {
    const roomId = room.roomId;
    
    console.log(`🔄 MatrixHistoryService: Syncing room ${roomId.substring(0, 20)}... to history`);

    // Check if we already have a session mapping for this room
    let existingMapping = sessionMappingService.getMapping(roomId);
    
    if (!existingMapping) {
      console.log(`📋 MatrixHistoryService: Creating new session mapping for room ${roomId.substring(0, 20)}...`);
      
      // Determine room type and name
      const isDirectMessage = room.isDirectMessage;
      const roomType: 'dm' | 'group' | 'collaborative' = isDirectMessage 
        ? 'dm' 
        : (room.members.length > 2 ? 'collaborative' : 'group');
      
      let roomName: string;
      if (room.name) {
        roomName = room.name;
      } else if (isDirectMessage) {
        // For DMs, create a name based on the other participant
        const currentUserId = matrixService.getCurrentUser()?.userId;
        const otherParticipant = room.members.find((m: any) => m.userId !== currentUserId);
        const otherName = otherParticipant?.displayName || otherParticipant?.userId?.split(':')[0].substring(1) || 'Unknown';
        roomName = `DM with ${otherName}`;
      } else {
        roomName = `Matrix Room ${roomId.substring(1, 8)}`;
      }

      const participants = room.members.map((m: any) => m.userId);

      try {
        // Create mapping with backend session
        existingMapping = await sessionMappingService.createMappingWithBackendSession(
          roomId,
          participants,
          roomName
        );
        console.log(`✅ MatrixHistoryService: Created backend session mapping for room ${roomId.substring(0, 20)}... → ${existingMapping.gooseSessionId}`);
      } catch (error) {
        console.error(`❌ MatrixHistoryService: Failed to create backend session for room ${roomId.substring(0, 20)}...:`, error);
        
        // Fallback to regular mapping
        existingMapping = sessionMappingService.createMapping(roomId, participants, roomName);
        console.log(`📋 MatrixHistoryService: Created fallback mapping for room ${roomId.substring(0, 20)}... → ${existingMapping.gooseSessionId}`);
      }
    } else {
      console.log(`📋 MatrixHistoryService: Using existing session mapping for room ${roomId.substring(0, 20)}... → ${existingMapping.gooseSessionId}`);
    }

    // Get room message history to populate the backend session
    try {
      const roomHistory = await matrixService.getRoomHistoryAsGooseMessages(roomId, 50);
      
      if (roomHistory.length > 0) {
        console.log(`📜 MatrixHistoryService: Syncing ${roomHistory.length} messages to backend session ${existingMapping.gooseSessionId}`);
        
        // Import the API function to sync messages to backend
        const { replyHandler } = await import('../api');
        
        // Convert Matrix messages to backend format
        const backendMessages = roomHistory.map(msg => ({
          id: `matrix_${msg.timestamp.getTime()}_${Math.random().toString(36).substr(2, 9)}`,
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
            session_id: existingMapping.gooseSessionId,
            messages: backendMessages,
          },
          throwOnError: false, // Don't throw on error to prevent breaking the sync
        });
        
        console.log(`✅ MatrixHistoryService: Successfully synced ${backendMessages.length} messages to backend session`);
      } else {
        console.log(`📜 MatrixHistoryService: No messages to sync for room ${roomId.substring(0, 20)}...`);
      }
    } catch (error) {
      console.error(`❌ MatrixHistoryService: Failed to sync messages for room ${roomId.substring(0, 20)}...:`, error);
      // Don't fail the entire sync if message sync fails
    }
  }

  /**
   * Get all Matrix rooms as history sessions
   */
  async getMatrixHistorySessions(): Promise<MatrixHistorySession[]> {
    if (!this.isInitialized) {
      await this.initialize();
    }

    const matrixRooms = matrixService.getRooms();
    const sessions: MatrixHistorySession[] = [];

    for (const room of matrixRooms) {
      try {
        const mapping = sessionMappingService.getMapping(room.roomId);
        
        if (mapping) {
          // Get message count from Matrix room history
          const roomHistory = await matrixService.getRoomHistory(room.roomId, 1);
          
          const session: MatrixHistorySession = {
            sessionId: mapping.gooseSessionId,
            matrixRoomId: room.roomId,
            title: room.name || mapping.title || `Matrix Room ${room.roomId.substring(1, 8)}`,
            lastActivity: room.lastActivity || new Date(),
            messageCount: roomHistory.length, // This is just a sample, could be improved
            participants: room.members.map((m: any) => m.userId),
            isDirectMessage: room.isDirectMessage,
            roomType: room.isDirectMessage 
              ? 'dm' 
              : (room.members.length > 2 ? 'collaborative' : 'group'),
          };
          
          sessions.push(session);
        }
      } catch (error) {
        console.error(`❌ MatrixHistoryService: Failed to process room ${room.roomId}:`, error);
      }
    }

    return sessions.sort((a, b) => b.lastActivity.getTime() - a.lastActivity.getTime());
  }

  /**
   * Sync a new Matrix room when it's joined
   */
  async syncNewRoom(roomId: string): Promise<void> {
    console.log(`🔄 MatrixHistoryService: Syncing new room ${roomId.substring(0, 20)}...`);
    
    const rooms = matrixService.getRooms();
    const room = rooms.find(r => r.roomId === roomId);
    
    if (room) {
      await this.syncRoomToHistory(room);
      console.log(`✅ MatrixHistoryService: Successfully synced new room ${roomId.substring(0, 20)}...`);
    } else {
      console.error(`❌ MatrixHistoryService: Room ${roomId} not found in Matrix rooms list`);
    }
  }

  /**
   * Force refresh all Matrix room mappings
   */
  async refreshAllRooms(): Promise<void> {
    console.log('🔄 MatrixHistoryService: Force refreshing all Matrix rooms...');
    this.isInitialized = false;
    await this.initialize();
  }

  /**
   * Get session ID for a Matrix room (for navigation)
   */
  getSessionIdForRoom(matrixRoomId: string): string | null {
    const mapping = sessionMappingService.getMapping(matrixRoomId);
    return mapping?.gooseSessionId || null;
  }

  /**
   * Check if a session ID corresponds to a Matrix room
   */
  isMatrixSession(sessionId: string): boolean {
    const allMappings = sessionMappingService.getAllMappings();
    return allMappings.some(mapping => mapping.gooseSessionId === sessionId && mapping.matrixRoomId);
  }

  /**
   * Get Matrix room ID for a session ID
   */
  getMatrixRoomIdForSession(sessionId: string): string | null {
    const allMappings = sessionMappingService.getAllMappings();
    const mapping = allMappings.find(m => m.gooseSessionId === sessionId);
    return mapping?.matrixRoomId || null;
  }
}

// Export singleton instance
export const matrixHistoryService = new MatrixHistoryService();

// Expose for debugging
if (typeof window !== 'undefined') {
  (window as any).matrixHistoryService = matrixHistoryService;
}
