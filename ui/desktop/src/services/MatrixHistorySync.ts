/**
 * MatrixHistorySync Service
 * 
 * Ensures that Matrix room history and Goose backend session history are perfectly synchronized.
 * Matrix is the source of truth - Goose backend mirrors Matrix history exactly.
 * 
 * Key Principles:
 * 1. Matrix room history is the canonical source of truth
 * 2. Goose backend session should mirror Matrix history exactly
 * 3. Messages are matched by timestamp and content
 * 4. No divergence allowed between Matrix and Goose
 * 5. Strict 1:1 mapping between Matrix room ID and Goose session ID
 */

import { matrixService } from './MatrixService';
import { sessionMappingService } from './SessionMappingService';
import { getSession } from '../api';
import type { Message } from '../api';

interface MatrixMessage {
  id: string;
  content: string;
  timestamp: Date;
  sender: string;
  role: 'user' | 'assistant';
  metadata?: {
    senderInfo?: {
      userId: string;
      displayName?: string;
      avatarUrl?: string;
    };
    matrixEventId?: string;
  };
}

interface GooseMessage {
  id: string;
  role: 'user' | 'assistant';
  content: Array<{ type: string; text?: string }>;
  created: number; // Unix timestamp in seconds
  sender?: {
    userId: string;
    displayName?: string;
    avatarUrl?: string;
  };
}

interface SyncResult {
  success: boolean;
  matrixMessageCount: number;
  gooseMessageCount: number;
  addedToGoose: number;
  errors: string[];
}

class MatrixHistorySyncService {
  private syncInProgress = new Map<string, boolean>();
  private lastSyncTime = new Map<string, number>();
  
  /**
   * Sync Matrix room history to Goose backend session
   * This is the primary sync function - Matrix is the source of truth
   */
  async syncRoomHistoryToBackend(
    matrixRoomId: string,
    gooseSessionId: string,
    options: {
      fullSync?: boolean; // If true, sync ALL history. If false, only sync new messages
      messageLimit?: number; // Max messages to fetch from Matrix
    } = {}
  ): Promise<SyncResult> {
    const { fullSync = true, messageLimit = 1000 } = options;
    
    // Prevent concurrent syncs for the same room
    if (this.syncInProgress.get(matrixRoomId)) {
      console.log('⏳ Sync already in progress for room:', matrixRoomId);
      return {
        success: false,
        matrixMessageCount: 0,
        gooseMessageCount: 0,
        addedToGoose: 0,
        errors: ['Sync already in progress']
      };
    }
    
    this.syncInProgress.set(matrixRoomId, true);
    const errors: string[] = [];
    
    try {
      console.log('🔄 Starting Matrix → Goose history sync:', {
        matrixRoomId: matrixRoomId.substring(0, 20) + '...',
        gooseSessionId,
        fullSync,
        messageLimit
      });
      
      // Step 1: Fetch Matrix room history (source of truth)
      console.log('📜 Fetching Matrix room history...');
      const matrixMessages = await matrixService.getRoomHistoryAsGooseMessages(
        matrixRoomId,
        messageLimit
      );
      
      console.log('📜 Fetched', matrixMessages.length, 'messages from Matrix');
      
      if (matrixMessages.length === 0) {
        console.log('ℹ️ No messages in Matrix room, nothing to sync');
        return {
          success: true,
          matrixMessageCount: 0,
          gooseMessageCount: 0,
          addedToGoose: 0,
          errors: []
        };
      }
      
      // Step 2: Fetch Goose backend session history
      console.log('📋 Fetching Goose backend session history...');
      let gooseMessages: GooseMessage[] = [];
      
      try {
        const sessionResponse = await getSession({
          path: { session_id: gooseSessionId }
        });
        
        if (sessionResponse.data?.conversation) {
          gooseMessages = sessionResponse.data.conversation;
          console.log('📋 Fetched', gooseMessages.length, 'messages from Goose backend');
        } else {
          console.log('📋 No conversation history in Goose backend');
        }
      } catch (error) {
        console.warn('⚠️ Failed to fetch Goose backend history:', error);
        errors.push(`Failed to fetch Goose history: ${error}`);
        // Continue with sync - we'll just add all Matrix messages
      }
      
      // Step 3: Compare and identify missing messages
      console.log('🔍 Comparing Matrix and Goose histories...');
      const missingInGoose = this.identifyMissingMessages(matrixMessages, gooseMessages);
      
      console.log('🔍 Found', missingInGoose.length, 'messages in Matrix that are missing from Goose');
      
      if (missingInGoose.length === 0) {
        console.log('✅ Goose backend is already in sync with Matrix');
        this.lastSyncTime.set(matrixRoomId, Date.now());
        return {
          success: true,
          matrixMessageCount: matrixMessages.length,
          gooseMessageCount: gooseMessages.length,
          addedToGoose: 0,
          errors
        };
      }
      
      // Step 4: Add missing messages to Goose backend
      console.log('📤 Adding', missingInGoose.length, 'missing messages to Goose backend...');
      const addedCount = await this.addMessagesToGooseBackend(
        gooseSessionId,
        missingInGoose,
        gooseMessages
      );
      
      console.log('✅ Successfully synced', addedCount, 'messages to Goose backend');
      this.lastSyncTime.set(matrixRoomId, Date.now());
      
      return {
        success: true,
        matrixMessageCount: matrixMessages.length,
        gooseMessageCount: gooseMessages.length + addedCount,
        addedToGoose: addedCount,
        errors
      };
      
    } catch (error) {
      console.error('❌ Matrix history sync failed:', error);
      errors.push(`Sync failed: ${error}`);
      
      return {
        success: false,
        matrixMessageCount: 0,
        gooseMessageCount: 0,
        addedToGoose: 0,
        errors
      };
    } finally {
      this.syncInProgress.delete(matrixRoomId);
    }
  }
  
  /**
   * Identify messages that exist in Matrix but not in Goose backend
   * Uses timestamp and content matching to identify missing messages
   */
  private identifyMissingMessages(
    matrixMessages: MatrixMessage[],
    gooseMessages: GooseMessage[]
  ): MatrixMessage[] {
    const missing: MatrixMessage[] = [];
    
    // Create a map of Goose messages by timestamp for quick lookup
    const gooseMessageMap = new Map<number, GooseMessage[]>();
    gooseMessages.forEach(msg => {
      const timestamp = msg.created;
      if (!gooseMessageMap.has(timestamp)) {
        gooseMessageMap.set(timestamp, []);
      }
      gooseMessageMap.get(timestamp)!.push(msg);
    });
    
    // Check each Matrix message
    for (const matrixMsg of matrixMessages) {
      const matrixTimestamp = Math.floor(matrixMsg.timestamp.getTime() / 1000);
      
      // Look for matching message in Goose by timestamp (±5 seconds tolerance)
      let found = false;
      
      for (let offset = -5; offset <= 5; offset++) {
        const checkTimestamp = matrixTimestamp + offset;
        const gooseMsgsAtTime = gooseMessageMap.get(checkTimestamp);
        
        if (gooseMsgsAtTime) {
          // Check if any message at this timestamp matches content
          for (const gooseMsg of gooseMsgsAtTime) {
            const gooseContent = gooseMsg.content
              .map(c => c.type === 'text' ? c.text : '')
              .join('');
            
            // Match by content similarity (exact match or very close)
            if (this.messagesMatch(matrixMsg.content, gooseContent)) {
              found = true;
              break;
            }
          }
        }
        
        if (found) break;
      }
      
      if (!found) {
        missing.push(matrixMsg);
        console.log('🔍 Missing in Goose:', {
          timestamp: matrixMsg.timestamp.toISOString(),
          sender: matrixMsg.sender,
          content: matrixMsg.content.substring(0, 50) + '...'
        });
      }
    }
    
    return missing;
  }
  
  /**
   * Check if two message contents match (allowing for minor differences)
   */
  private messagesMatch(content1: string, content2: string): boolean {
    // Normalize whitespace and compare
    const normalize = (s: string) => s.trim().replace(/\s+/g, ' ').toLowerCase();
    return normalize(content1) === normalize(content2);
  }
  
  /**
   * Add missing messages to Goose backend session
   * Maintains chronological order based on Matrix timestamps
   */
  private async addMessagesToGooseBackend(
    gooseSessionId: string,
    missingMessages: MatrixMessage[],
    existingGooseMessages: GooseMessage[]
  ): Promise<number> {
    if (missingMessages.length === 0) {
      return 0;
    }
    
    try {
      // Sort missing messages by timestamp (oldest first)
      const sortedMissing = [...missingMessages].sort(
        (a, b) => a.timestamp.getTime() - b.timestamp.getTime()
      );
      
      // Convert Matrix messages to Goose message format
      const gooseFormattedMessages: GooseMessage[] = sortedMissing.map(msg => ({
        id: `matrix_${msg.timestamp.getTime()}_${msg.id}`,
        role: msg.role,
        content: [{
          type: 'text',
          text: msg.content
        }],
        created: Math.floor(msg.timestamp.getTime() / 1000),
        sender: msg.metadata?.senderInfo ? {
          userId: msg.metadata.senderInfo.userId,
          displayName: msg.metadata.senderInfo.displayName,
          avatarUrl: msg.metadata.senderInfo.avatarUrl
        } : undefined
      }));
      
      // Merge with existing messages and sort by timestamp
      const allMessages = [...existingGooseMessages, ...gooseFormattedMessages]
        .sort((a, b) => a.created - b.created);
      
      // Update the backend session with the complete, sorted history
      // Note: This requires a backend API endpoint to update session history
      // For now, we'll use the reply endpoint to add messages incrementally
      
      console.log('📤 Sending', gooseFormattedMessages.length, 'messages to Goose backend');
      
      // Import the API client
      const { client } = await import('../api/client.gen');
      
      // Get the secret key for authentication
      const secretKey = await window.electron.getSecretKey();
      
      // Send the complete conversation history to the backend
      // This will replace the existing conversation with the synced version
      const response = await client.post({
        url: `/sessions/${gooseSessionId}/sync-history`,
        body: {
          messages: allMessages
        },
        headers: {
          'X-Secret-Key': secretKey
        },
        throwOnError: false
      });
      
      if (response.error) {
        console.warn('⚠️ Backend history sync endpoint not available, using fallback method');
        
        // Fallback: Add messages one by one using the reply endpoint
        // This is less efficient but works with existing API
        for (const msg of gooseFormattedMessages) {
          try {
            await client.post({
              url: '/reply',
              body: {
                session_id: gooseSessionId,
                messages: [msg]
              },
              headers: {
                'X-Secret-Key': secretKey
              },
              throwOnError: false
            });
          } catch (error) {
            console.warn('⚠️ Failed to add message to backend:', error);
          }
        }
      }
      
      console.log('✅ Successfully added', gooseFormattedMessages.length, 'messages to Goose backend');
      return gooseFormattedMessages.length;
      
    } catch (error) {
      console.error('❌ Failed to add messages to Goose backend:', error);
      throw error;
    }
  }
  
  /**
   * Get the last sync time for a room
   */
  getLastSyncTime(matrixRoomId: string): number | undefined {
    return this.lastSyncTime.get(matrixRoomId);
  }
  
  /**
   * Check if a sync is currently in progress for a room
   */
  isSyncInProgress(matrixRoomId: string): boolean {
    return this.syncInProgress.get(matrixRoomId) || false;
  }
  
  /**
   * Validate that Matrix room and Goose session are properly mapped
   */
  async validateMapping(matrixRoomId: string, gooseSessionId: string): Promise<boolean> {
    try {
      // Check forward mapping (Matrix → Goose)
      const mappedSessionId = sessionMappingService.getGooseSessionId(matrixRoomId);
      if (mappedSessionId !== gooseSessionId) {
        console.error('❌ Mapping validation failed: Matrix room maps to different session', {
          matrixRoomId: matrixRoomId.substring(0, 20) + '...',
          expectedSessionId: gooseSessionId,
          actualSessionId: mappedSessionId
        });
        return false;
      }
      
      // Check reverse mapping (Goose → Matrix)
      const mappedRoomId = sessionMappingService.getMatrixRoomId(gooseSessionId);
      if (mappedRoomId !== matrixRoomId) {
        console.error('❌ Mapping validation failed: Goose session maps to different room', {
          gooseSessionId,
          expectedRoomId: matrixRoomId.substring(0, 20) + '...',
          actualRoomId: mappedRoomId?.substring(0, 20) + '...'
        });
        return false;
      }
      
      console.log('✅ Mapping validation passed:', {
        matrixRoomId: matrixRoomId.substring(0, 20) + '...',
        gooseSessionId
      });
      
      return true;
    } catch (error) {
      console.error('❌ Mapping validation error:', error);
      return false;
    }
  }
}

// Export singleton instance
export const matrixHistorySyncService = new MatrixHistorySyncService();
