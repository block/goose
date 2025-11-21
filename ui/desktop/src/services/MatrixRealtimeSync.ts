/**
 * MatrixRealtimeSync - Handles real-time syncing of Matrix messages to backend sessions
 * 
 * This service listens for new Matrix messages and writes them directly to the corresponding
 * backend session, allowing the normal chat streaming mechanism to handle display.
 * This approach is much cleaner than trying to route Matrix messages directly to the UI.
 */

import { matrixService } from './MatrixService';
import { sessionMappingService } from './SessionMappingService';
import { replyHandler } from '../api';

export class MatrixRealtimeSync {
  private static instance: MatrixRealtimeSync;
  private isActive = false;
  private messageCleanup: (() => void) | null = null;
  private processedMessages = new Set<string>();

  private constructor() {}

  public static getInstance(): MatrixRealtimeSync {
    if (!MatrixRealtimeSync.instance) {
      MatrixRealtimeSync.instance = new MatrixRealtimeSync();
    }
    return MatrixRealtimeSync.instance;
  }

  /**
   * Start real-time syncing of Matrix messages to backend sessions
   */
  public start(): void {
    if (this.isActive) {
      console.log('🔄 MatrixRealtimeSync: Already active');
      return;
    }

    console.log('🚀 MatrixRealtimeSync: Starting real-time Matrix message sync to backend sessions');
    this.isActive = true;

    // Listen for all Matrix messages using EventEmitter pattern
    const messageHandler = (data: any) => {
      this.handleMatrixMessage(data);
    };
    
    matrixService.on('message', messageHandler);
    
    this.messageCleanup = () => {
      matrixService.off('message', messageHandler);
    };

    console.log('✅ MatrixRealtimeSync: Real-time sync started');
  }

  /**
   * Stop real-time syncing
   */
  public stop(): void {
    if (!this.isActive) {
      return;
    }

    console.log('🛑 MatrixRealtimeSync: Stopping real-time sync');
    this.isActive = false;

    if (this.messageCleanup) {
      this.messageCleanup();
      this.messageCleanup = null;
    }

    this.processedMessages.clear();
    console.log('✅ MatrixRealtimeSync: Real-time sync stopped');
  }

  /**
   * Handle incoming Matrix messages and sync them to backend sessions
   */
  private async handleMatrixMessage(data: any): Promise<void> {
    const { content, sender, roomId, senderInfo, timestamp, event } = data;

    try {
      // Skip if no room ID or content
      if (!roomId || !content) {
        return;
      }

      // Skip messages from ourselves to prevent loops
      const currentUser = matrixService.getCurrentUser();
      if (sender === currentUser?.userId) {
        return;
      }

      // Create a unique message key for deduplication
      const messageKey = this.createMessageKey(content, sender, roomId, timestamp);
      if (this.processedMessages.has(messageKey)) {
        return; // Already processed this message
      }

      // Skip goose-session-message: prefixed messages (these are internal)
      if (content.includes('goose-session-message:') || content.includes('goose-session-invite:') || content.includes('goose-session-joined:')) {
        return;
      }

      // CRITICAL: Check Matrix room ownership before syncing
      // Only sync to the session that currently owns this Matrix room
      const globalMatrixListenerRegistry = (window as any).__gooseMatrixListenerRegistry as Map<string, string>;
      const roomOwner = globalMatrixListenerRegistry?.get(roomId);
      
      if (!roomOwner) {
        console.log('🚫 MatrixRealtimeSync: No active owner for Matrix room, skipping sync:', {
          roomId: roomId.substring(0, 20) + '...',
          sender: sender.split(':')[0].substring(1)
        });
        return;
      }

      // Get the session mapping for the OWNER session, not just any mapping
      const allMappings = sessionMappingService.getAllMappings();
      const roomMappings = allMappings.filter(m => m.matrixRoomId === roomId);
      
      // Find the mapping that corresponds to the current room owner
      const ownerMapping = roomMappings.find(m => m.gooseSessionId === roomOwner);
      
      if (!ownerMapping) {
        console.log('🚫 MatrixRealtimeSync: No mapping found for room owner, skipping sync:', {
          roomId: roomId.substring(0, 20) + '...',
          roomOwner,
          availableMappings: roomMappings.map(m => m.gooseSessionId),
          sender: sender.split(':')[0].substring(1)
        });
        return;
      }

      console.log('🔍 MatrixRealtimeSync: Matrix room ownership and mapping verified:', {
        roomId: roomId.substring(0, 20) + '...',
        roomOwner,
        totalMappings: allMappings.length,
        roomMappingsCount: roomMappings.length,
        allRoomMappingSessionIds: roomMappings.map(m => m.gooseSessionId),
        selectedOwnerSessionId: ownerMapping.gooseSessionId,
        sender: sender.split(':')[0].substring(1),
        contentPreview: content.substring(0, 50) + '...'
      });

      // CRITICAL DEBUG: Check if there are multiple mappings for this room
      if (roomMappings.length > 1) {
        console.warn('⚠️ MatrixRealtimeSync: MULTIPLE MAPPINGS DETECTED for same Matrix room:', {
          roomId: roomId.substring(0, 20) + '...',
          mappings: roomMappings.map(m => ({
            sessionId: m.gooseSessionId,
            title: m.title,
            createdAt: new Date(m.createdAt).toISOString(),
            lastUsed: new Date(m.lastUsed).toISOString()
          })),
          currentOwner: roomOwner,
          selectedMapping: ownerMapping.gooseSessionId
        });
      }

      console.log('📨 MatrixRealtimeSync: Processing Matrix message for backend sync:', {
        roomId: roomId.substring(0, 20) + '...',
        backendSessionId: ownerMapping.gooseSessionId,
        sender: sender.split(':')[0].substring(1),
        contentPreview: content.substring(0, 50) + '...'
      });

      // Mark as processed
      this.processedMessages.add(messageKey);

      // Determine message role based on content and sender
      const messageRole = this.determineMessageRole(content, sender, senderInfo);

      // Get sender information
      const senderData = this.getSenderInfo(sender, senderInfo);

      // Create the message in backend format
      const backendMessage = {
        id: `matrix_realtime_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
        role: messageRole,
        content: [{
          type: 'text' as const,
          text: content,
        }],
        created: Math.floor((timestamp?.getTime?.() || Date.now()) / 1000),
        metadata: {
          isFromMatrix: true,
          matrixRoomId: roomId,
          matrixSender: sender,
          senderInfo: senderData,
          skipLocalResponse: messageRole === 'user', // Don't trigger AI response for user messages
        }
      };

      // Sync the message to the backend session that OWNS this Matrix room
      await replyHandler({
        body: {
          session_id: ownerMapping.gooseSessionId,
          messages: [backendMessage],
        },
        throwOnError: false, // Don't throw errors to prevent breaking the sync
      });

      console.log('✅ MatrixRealtimeSync: Successfully synced Matrix message to backend session:', {
        messageId: backendMessage.id,
        role: messageRole,
        backendSessionId: ownerMapping.gooseSessionId,
        sender: senderData.displayName || senderData.userId,
        roomOwner: roomOwner
      });

    } catch (error) {
      console.error('❌ MatrixRealtimeSync: Error syncing Matrix message to backend:', error);
      // Don't throw - we want to continue processing other messages
    }
  }

  /**
   * Create a unique message key for deduplication
   */
  private createMessageKey(content: string, sender: string, roomId: string, timestamp?: Date): string {
    const time = timestamp?.getTime() || Date.now();
    const roundedTime = Math.floor(time / 1000); // Round to nearest second
    return `${roomId}-${sender}-${roundedTime}-${content.substring(0, 50)}`;
  }

  /**
   * Determine the message role based on content and sender
   */
  private determineMessageRole(content: string, sender: string, senderInfo?: any): 'user' | 'assistant' {
    // Check if sender info indicates it's a Goose instance
    const senderDisplayName = senderInfo?.displayName || sender;
    const isGooseSender = senderDisplayName.toLowerCase().includes('goose') || 
                         sender.toLowerCase().includes('goose');

    if (isGooseSender) {
      return 'assistant';
    }

    // Use content-based heuristics for role detection
    const isGooseResponse = content && (
      // Direct Goose markers
      content.includes('🦆') ||
      content.includes('🤖') ||
      // AI assistant patterns
      /I'm\s+goose,?\s+an?\s+AI\s+(agent|assistant)/i.test(content) ||
      /I'm\s+an?\s+AI\s+(agent|assistant)/i.test(content) ||
      // Tool usage patterns
      /I\s+have\s+access\s+to\s+(several\s+)?tools/i.test(content) ||
      /I\s+can\s+(use|access)\s+(tools|extensions)/i.test(content) ||
      // Code blocks (common in AI responses)
      /```[\s\S]*```/.test(content) ||
      // Long structured responses
      (content.length > 200 && /\n\n/.test(content) && /^(I|Let|Here|To|The)/i.test(content))
    );

    return isGooseResponse ? 'assistant' : 'user';
  }

  /**
   * Get sender information for the message
   */
  private getSenderInfo(sender: string, senderInfo?: any): { userId: string; displayName: string; avatarUrl?: string } {
    if (senderInfo) {
      return {
        userId: senderInfo.userId || sender,
        displayName: senderInfo.displayName || sender.split(':')[0].substring(1),
        avatarUrl: senderInfo.avatarUrl,
      };
    }

    // Fallback to extracting from Matrix ID
    return {
      userId: sender,
      displayName: sender.split(':')[0].substring(1), // Extract username from @user:server.com
    };
  }

  /**
   * Get current sync status
   */
  public getStatus(): { isActive: boolean; processedMessageCount: number } {
    return {
      isActive: this.isActive,
      processedMessageCount: this.processedMessages.size,
    };
  }

  /**
   * Clear processed message cache (useful for debugging)
   */
  public clearCache(): void {
    this.processedMessages.clear();
    console.log('🧹 MatrixRealtimeSync: Cleared processed message cache');
  }
}

// Export singleton instance
export const matrixRealtimeSync = MatrixRealtimeSync.getInstance();
