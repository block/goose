/**
 * SessionMappingService - Manages the relationship between Matrix room IDs and Goose session IDs
 * 
 * This service enables hybrid session management where:
 * - Matrix room IDs (e.g., "!abc123:server.com") are used for Matrix operations
 * - Goose session IDs (e.g., "20251115_143022") are used for backend API calls
 * - The mapping ensures both systems work harmoniously
 */

export interface MatrixRoomParticipant {
  userId: string;
  displayName?: string;
  avatarUrl?: string;
  joinedAt: number;
  leftAt?: number;
  membership: 'join' | 'leave' | 'invite' | 'ban' | 'knock';
  lastActivity?: number;
}

export interface MatrixRoomMetadata {
  name?: string;
  topic?: string;
  avatarUrl?: string;
  memberCount: number;
  isDirectMessage: boolean;
  isEncrypted: boolean;
  roomType?: string;
  createdAt?: number;
  lastActivity?: number;
}

export interface MatrixRoomState {
  roomId: string;
  metadata: MatrixRoomMetadata;
  participants: Map<string, MatrixRoomParticipant>;
  membershipHistory: Array<{
    userId: string;
    membership: string;
    timestamp: number;
    event: 'join' | 'leave' | 'invite' | 'kick' | 'ban';
    inviter?: string;
  }>;
  lastSyncAt: number;
}

export interface SessionMapping {
  matrixRoomId: string;
  gooseSessionId: string;
  createdAt: number;
  lastUsed: number;
  participants: string[]; // Matrix user IDs - kept for backward compatibility
  title?: string;
  // Enhanced Matrix room state tracking
  roomState?: MatrixRoomState;
  isMatrixCollaborative: boolean;
}

export class SessionMappingService {
  private static instance: SessionMappingService;
  private readonly STORAGE_KEY = 'goose-matrix-session-mappings';
  private mappings: Map<string, SessionMapping> = new Map();

  private constructor() {
    this.loadMappingsFromStorage();
  }

  public static getInstance(): SessionMappingService {
    if (!SessionMappingService.instance) {
      SessionMappingService.instance = new SessionMappingService();
    }
    return SessionMappingService.instance;
  }

  /**
   * Generate a new Goose session ID in the expected format
   * Note: This is now deprecated in favor of using actual backend session IDs
   */
  private generateGooseSessionId(): string {
    const now = new Date();
    const dateStr = now.toISOString().slice(0, 10).replace(/-/g, ''); // YYYYMMDD
    const timeStr = now.toTimeString().slice(0, 8).replace(/:/g, ''); // HHMMSS
    return `${dateStr}_${timeStr}`;
  }

  /**
   * Create a new session mapping for a Matrix room
   */
  public createMapping(matrixRoomId: string, participants: string[] = [], title?: string): SessionMapping {
    const gooseSessionId = this.generateGooseSessionId();
    const now = Date.now();

    const mapping: SessionMapping = {
      matrixRoomId,
      gooseSessionId,
      createdAt: now,
      lastUsed: now,
      participants,
      title,
      isMatrixCollaborative: false, // Will be set to true when room state is updated
    };

    this.mappings.set(matrixRoomId, mapping);
    this.saveMappingsToStorage();

    console.log('📋 SessionMappingService: Created new mapping:', {
      matrixRoomId,
      gooseSessionId,
      participants: participants.length,
      title,
    });

    return mapping;
  }

  /**
   * Create a new session mapping with a backend session ID
   * This creates a real backend session for the Matrix room
   */
  public async createMappingWithBackendSession(
    matrixRoomId: string, 
    participants: string[] = [], 
    title?: string,
    matrixRecipientId?: string
  ): Promise<SessionMapping> {
    try {
      // Import startAgent dynamically to avoid circular dependencies
      const { startAgent } = await import('../api');
      
      // Determine if this is a DM or group room for appropriate instructions
      const isDM = participants.length === 2;
      const roomType = isDM ? 'Direct Message' : 'Group Chat';
      
      // Create appropriate instructions based on room type
      const instructions = isDM 
        ? `You are in a direct message conversation through Matrix. This is a 1:1 chat session. Be helpful and respond naturally to the user's messages.`
        : `You are participating in a collaborative AI session through Matrix. Multiple users may be participating in this conversation. Be helpful and collaborative in your responses.`;

      // Create a backend session for this Matrix room
      const agentResponse = await startAgent({
        body: {
          working_dir: window.appConfig.get('GOOSE_WORKING_DIR') as string,
          // Create a recipe for Matrix collaboration
          recipe: {
            title: title || `Matrix ${roomType}: ${matrixRoomId.substring(1, 8)}`,
            description: `${roomType} session for Matrix room ${matrixRoomId}`,
            instructions: instructions, // Send as string, not array
          },
        },
        throwOnError: true,
      });

      const backendSession = agentResponse.data;
      if (!backendSession?.id) {
        throw new Error('Backend session creation returned no session ID');
      }
      
      console.log('📋 SessionMappingService: Backend session created successfully:', {
        sessionId: backendSession.id,
        matrixRoomId: matrixRoomId.substring(0, 20) + '...',
        title,
        isDM
      });

      // CRITICAL: Store Matrix metadata in backend session description
      try {
        // Import updateSessionDescription to store Matrix metadata
        const { updateSessionDescription } = await import('../api');
        
        // Create Matrix metadata
        const matrixMetadata = {
          roomId: matrixRoomId,
          recipientId: matrixRecipientId || participants[0] || null,
          isMatrixSession: true,
          roomName: title || `Matrix ${roomType}`,
          isDM: isDM,
          participants: participants,
          createdAt: Date.now(),
          sessionMappingVersion: '1.0'
        };

        // Store Matrix metadata in the session description
        // Format: "Matrix DM: Room Name [MATRIX_METADATA:base64encodeddata]"
        const baseDescription = `Matrix ${roomType}: ${title || matrixRoomId.substring(1, 8)}`;
        const encodedMetadata = btoa(JSON.stringify(matrixMetadata));
        const sessionDescription = `${baseDescription} [MATRIX_METADATA:${encodedMetadata}]`;
        
        await updateSessionDescription({
          path: {
            session_id: backendSession.id
          },
          body: {
            description: sessionDescription
          },
          throwOnError: false
        });

        console.log('📋 SessionMappingService: Stored Matrix metadata in backend session description:', {
          sessionId: backendSession.id,
          matrixRoomId,
          matrixRecipientId,
          isDM,
          descriptionLength: sessionDescription.length
        });

      } catch (metadataError) {
        console.warn('📋 SessionMappingService: Failed to store Matrix metadata in backend session:', metadataError);
        // Continue even if metadata storage fails - the mapping will still work
      }

      const now = Date.now();
      const mapping: SessionMapping = {
        matrixRoomId,
        gooseSessionId: backendSession.id, // Use the actual backend session ID
        createdAt: now,
        lastUsed: now,
        participants,
        title,
        isMatrixCollaborative: false, // Will be set to true when room state is updated
      };

      this.mappings.set(matrixRoomId, mapping);
      this.saveMappingsToStorage();

      console.log('📋 SessionMappingService: Created mapping with backend session:', {
        matrixRoomId,
        backendSessionId: backendSession.id,
        participants: participants.length,
        title,
      });

      return mapping;
    } catch (error) {
      console.error('📋 SessionMappingService: Failed to create backend session for Matrix room:', error);
      // Fallback to the old method if backend session creation fails
      return this.createMapping(matrixRoomId, participants, title);
    }
  }

  /**
   * Get the Goose session ID for a Matrix room ID
   */
  public getGooseSessionId(matrixRoomId: string): string | null {
    const mapping = this.mappings.get(matrixRoomId);
    if (mapping) {
      // Update last used timestamp
      mapping.lastUsed = Date.now();
      this.saveMappingsToStorage();
      return mapping.gooseSessionId;
    }
    return null;
  }

  /**
   * Get the Matrix room ID for a Goose session ID
   */
  public getMatrixRoomId(gooseSessionId: string): string | null {
    for (const [matrixRoomId, mapping] of this.mappings.entries()) {
      if (mapping.gooseSessionId === gooseSessionId) {
        // Update last used timestamp
        mapping.lastUsed = Date.now();
        this.saveMappingsToStorage();
        return matrixRoomId;
      }
    }
    return null;
  }

  /**
   * Get the complete mapping for a Matrix room ID
   */
  public getMapping(matrixRoomId: string): SessionMapping | null {
    const mapping = this.mappings.get(matrixRoomId);
    if (mapping) {
      // Update last used timestamp
      mapping.lastUsed = Date.now();
      this.saveMappingsToStorage();
    }
    return mapping || null;
  }

  /**
   * Update participants in a mapping
   */
  public updateParticipants(matrixRoomId: string, participants: string[]): void {
    const mapping = this.mappings.get(matrixRoomId);
    if (mapping) {
      mapping.participants = participants;
      mapping.lastUsed = Date.now();
      this.saveMappingsToStorage();

      console.log('📋 SessionMappingService: Updated participants:', {
        matrixRoomId,
        gooseSessionId: mapping.gooseSessionId,
        participants: participants.length,
      });
    }
  }

  /**
   * Update title in a mapping
   */
  public updateTitle(matrixRoomId: string, title: string): void {
    const mapping = this.mappings.get(matrixRoomId);
    if (mapping) {
      mapping.title = title;
      mapping.lastUsed = Date.now();
      this.saveMappingsToStorage();

      console.log('📋 SessionMappingService: Updated title:', {
        matrixRoomId,
        gooseSessionId: mapping.gooseSessionId,
        title,
      });
    }
  }

  /**
   * Check if a session ID is a Matrix room ID
   */
  public static isMatrixRoomId(sessionId: string): boolean {
    return sessionId.startsWith('!') && sessionId.includes(':');
  }

  /**
   * Check if a session ID is a Goose session ID
   */
  public static isGooseSessionId(sessionId: string): boolean {
    return /^\d{8}_\d{6}$/.test(sessionId);
  }

  /**
   * Get the appropriate session ID for backend API calls
   * Returns the mapped Goose session ID if it's a Matrix room, otherwise returns the original ID
   */
  public getBackendSessionId(sessionId: string): string | null {
    if (SessionMappingService.isMatrixRoomId(sessionId)) {
      const gooseSessionId = this.getGooseSessionId(sessionId);
      if (gooseSessionId) {
        return gooseSessionId;
      }
      console.warn('📋 SessionMappingService: No mapping found for Matrix room ID:', sessionId, '- skipping backend calls');
      // Return null to indicate that backend calls should be skipped
      return null;
    }
    return sessionId;
  }

  /**
   * Check if backend API calls should be made for this session ID
   * Returns false for Matrix sessions without mappings
   */
  public shouldMakeBackendCalls(sessionId: string): boolean {
    if (SessionMappingService.isMatrixRoomId(sessionId)) {
      const gooseSessionId = this.getGooseSessionId(sessionId);
      return gooseSessionId !== null;
    }
    return true; // Always make backend calls for regular Goose sessions
  }

  /**
   * Create a mapping for an existing Matrix room if one doesn't exist
   * This is useful for Matrix rooms loaded from history that don't have mappings yet
   */
  public ensureMappingExists(matrixRoomId: string, title?: string): SessionMapping {
    const existingMapping = this.getMapping(matrixRoomId);
    if (existingMapping) {
      return existingMapping;
    }

    // Create a new mapping for this Matrix room
    console.log('📋 SessionMappingService: Creating mapping for existing Matrix room:', matrixRoomId);
    return this.createMapping(matrixRoomId, [], title || `Matrix Room ${matrixRoomId.substring(1, 8)}`);
  }

  /**
   * Get all mappings (for debugging/management)
   */
  public getAllMappings(): SessionMapping[] {
    return Array.from(this.mappings.values());
  }

  /**
   * Remove old mappings (older than 30 days)
   */
  public cleanupOldMappings(): void {
    const thirtyDaysAgo = Date.now() - (30 * 24 * 60 * 60 * 1000);
    let removedCount = 0;

    for (const [matrixRoomId, mapping] of this.mappings.entries()) {
      if (mapping.lastUsed < thirtyDaysAgo) {
        this.mappings.delete(matrixRoomId);
        removedCount++;
      }
    }

    if (removedCount > 0) {
      this.saveMappingsToStorage();
      console.log(`📋 SessionMappingService: Cleaned up ${removedCount} old mappings`);
    }
  }

  /**
   * Load mappings from localStorage
   */
  private loadMappingsFromStorage(): void {
    try {
      const stored = localStorage.getItem(this.STORAGE_KEY);
      if (stored) {
        const data = JSON.parse(stored);
        
        // Deserialize each mapping to handle Map objects
        const deserializedEntries = Object.entries(data).map(([key, value]) => [
          key,
          this.deserializeMapping(value as any),
        ]);
        
        this.mappings = new Map(deserializedEntries);
        console.log(`📋 SessionMappingService: Loaded ${this.mappings.size} mappings from storage`);
        
        // Clean up old mappings on load
        this.cleanupOldMappings();
      }
    } catch (error) {
      console.error('📋 SessionMappingService: Error loading mappings from storage:', error);
      this.mappings = new Map();
    }
  }

  /**
   * Save mappings to localStorage
   */
  private saveMappingsToStorage(): void {
    try {
      // Serialize each mapping to handle Map objects
      const serializedEntries = Array.from(this.mappings.entries()).map(([key, value]) => [
        key,
        this.serializeMapping(value),
      ]);
      
      const data = Object.fromEntries(serializedEntries);
      localStorage.setItem(this.STORAGE_KEY, JSON.stringify(data));
    } catch (error) {
      console.error('📋 SessionMappingService: Error saving mappings to storage:', error);
    }
  }

  /**
   * Update Matrix room state for a session mapping
   */
  public updateMatrixRoomState(matrixRoomId: string, roomState: Partial<MatrixRoomState>): void {
    const mapping = this.mappings.get(matrixRoomId);
    if (mapping) {
      // Initialize room state if it doesn't exist
      if (!mapping.roomState) {
        mapping.roomState = {
          roomId: matrixRoomId,
          metadata: {
            memberCount: 0,
            isDirectMessage: false,
            isEncrypted: false,
          },
          participants: new Map(),
          membershipHistory: [],
          lastSyncAt: Date.now(),
        };
      }

      // Merge the provided room state
      mapping.roomState = {
        ...mapping.roomState,
        ...roomState,
        lastSyncAt: Date.now(),
      };

      mapping.isMatrixCollaborative = true;
      mapping.lastUsed = Date.now();
      this.saveMappingsToStorage();

      console.log('📋 SessionMappingService: Updated Matrix room state:', {
        matrixRoomId,
        memberCount: mapping.roomState.metadata.memberCount,
        participantCount: mapping.roomState.participants.size,
      });
    }
  }

  /**
   * Add or update a participant in a Matrix room
   */
  public updateMatrixParticipant(
    matrixRoomId: string,
    participant: MatrixRoomParticipant,
    event?: 'join' | 'leave' | 'invite' | 'kick' | 'ban',
    inviter?: string
  ): void {
    const mapping = this.mappings.get(matrixRoomId);
    if (mapping) {
      // Initialize room state if needed
      if (!mapping.roomState) {
        this.updateMatrixRoomState(matrixRoomId, {});
      }

      // Update participant
      mapping.roomState!.participants.set(participant.userId, participant);

      // Add to membership history if event is provided
      if (event) {
        mapping.roomState!.membershipHistory.push({
          userId: participant.userId,
          membership: participant.membership,
          timestamp: Date.now(),
          event,
          inviter,
        });
      }

      // Update member count
      const activeMembers = Array.from(mapping.roomState!.participants.values())
        .filter(p => p.membership === 'join').length;
      mapping.roomState!.metadata.memberCount = activeMembers;

      mapping.lastUsed = Date.now();
      this.saveMappingsToStorage();

      console.log('📋 SessionMappingService: Updated Matrix participant:', {
        matrixRoomId,
        userId: participant.userId,
        membership: participant.membership,
        event,
        activeMembers,
      });
    }
  }

  /**
   * Update Matrix room metadata
   */
  public updateMatrixRoomMetadata(matrixRoomId: string, metadata: Partial<MatrixRoomMetadata>): void {
    const mapping = this.mappings.get(matrixRoomId);
    if (mapping) {
      // Initialize room state if needed
      if (!mapping.roomState) {
        this.updateMatrixRoomState(matrixRoomId, {});
      }

      // Merge metadata
      mapping.roomState!.metadata = {
        ...mapping.roomState!.metadata,
        ...metadata,
      };

      mapping.lastUsed = Date.now();
      this.saveMappingsToStorage();

      console.log('📋 SessionMappingService: Updated Matrix room metadata:', {
        matrixRoomId,
        name: metadata.name,
        topic: metadata.topic,
        memberCount: metadata.memberCount,
      });
    }
  }

  /**
   * Get Matrix room state for a session
   */
  public getMatrixRoomState(matrixRoomId: string): MatrixRoomState | null {
    const mapping = this.mappings.get(matrixRoomId);
    return mapping?.roomState || null;
  }

  /**
   * Check if a session is a Matrix collaborative session
   */
  public isMatrixCollaborativeSession(sessionId: string): boolean {
    if (SessionMappingService.isMatrixRoomId(sessionId)) {
      const mapping = this.mappings.get(sessionId);
      return mapping?.isMatrixCollaborative || false;
    }
    
    // Check if this is a Goose session ID that maps to a Matrix room
    const matrixRoomId = this.getMatrixRoomId(sessionId);
    if (matrixRoomId) {
      const mapping = this.mappings.get(matrixRoomId);
      return mapping?.isMatrixCollaborative || false;
    }
    
    return false;
  }

  /**
   * Get all Matrix collaborative sessions
   */
  public getMatrixCollaborativeSessions(): SessionMapping[] {
    return Array.from(this.mappings.values())
      .filter(mapping => mapping.isMatrixCollaborative);
  }

  /**
   * Serialize Map objects for storage (Maps don't serialize to JSON by default)
   */
  private serializeMapping(mapping: SessionMapping): any {
    if (!mapping.roomState) {
      return mapping;
    }

    return {
      ...mapping,
      roomState: {
        ...mapping.roomState,
        participants: Array.from(mapping.roomState.participants.entries()),
      },
    };
  }

  /**
   * Deserialize Map objects from storage
   */
  private deserializeMapping(data: any): SessionMapping {
    if (!data.roomState || !data.roomState.participants) {
      return data;
    }

    return {
      ...data,
      roomState: {
        ...data.roomState,
        participants: new Map(data.roomState.participants),
      },
    };
  }

  /**
   * Extract Matrix metadata from a backend session description
   * Returns null if the session is not a Matrix session
   */
  public static extractMatrixMetadataFromDescription(description: string): any | null {
    try {
      // Look for the Matrix metadata pattern: [MATRIX_METADATA:base64encodeddata]
      const metadataMatch = description.match(/\[MATRIX_METADATA:([A-Za-z0-9+/=]+)\]/);
      if (!metadataMatch) {
        return null;
      }

      // Decode the base64 metadata
      const encodedMetadata = metadataMatch[1];
      const decodedMetadata = atob(encodedMetadata);
      const matrixMetadata = JSON.parse(decodedMetadata);

      console.log('📋 SessionMappingService: Extracted Matrix metadata from session description:', {
        roomId: matrixMetadata.roomId,
        recipientId: matrixMetadata.recipientId,
        isDM: matrixMetadata.isDM,
        roomName: matrixMetadata.roomName
      });

      return matrixMetadata;
    } catch (error) {
      console.error('📋 SessionMappingService: Failed to extract Matrix metadata from description:', error);
      return null;
    }
  }

  /**
   * Check if a backend session is a Matrix session based on its description
   */
  public static isBackendSessionMatrix(sessionDescription: string): boolean {
    return sessionDescription.includes('[MATRIX_METADATA:');
  }

  /**
   * Create or restore a Matrix session mapping from backend session data
   * This is used when loading sessions from the backend that contain Matrix metadata
   */
  public createMappingFromBackendSession(sessionId: string, sessionDescription: string): SessionMapping | null {
    const matrixMetadata = SessionMappingService.extractMatrixMetadataFromDescription(sessionDescription);
    if (!matrixMetadata) {
      return null;
    }

    const now = Date.now();
    const mapping: SessionMapping = {
      matrixRoomId: matrixMetadata.roomId,
      gooseSessionId: sessionId,
      createdAt: matrixMetadata.createdAt || now,
      lastUsed: now,
      participants: matrixMetadata.participants || [],
      title: matrixMetadata.roomName,
      isMatrixCollaborative: false, // Will be set to true when room state is updated
    };

    // Store the mapping
    this.mappings.set(matrixMetadata.roomId, mapping);
    this.saveMappingsToStorage();

    console.log('📋 SessionMappingService: Created mapping from backend session:', {
      matrixRoomId: matrixMetadata.roomId,
      backendSessionId: sessionId,
      title: matrixMetadata.roomName,
      isDM: matrixMetadata.isDM
    });

    return mapping;
  }

  /**
   * Get Matrix metadata for a backend session ID
   * This checks both local mappings and can extract from session description if needed
   */
  public async getMatrixMetadataForBackendSession(sessionId: string): Promise<any | null> {
    // First check if we have a local mapping
    const matrixRoomId = this.getMatrixRoomId(sessionId);
    if (matrixRoomId) {
      const mapping = this.getMapping(matrixRoomId);
      if (mapping) {
        return {
          roomId: matrixRoomId,
          recipientId: mapping.participants[0] || null,
          isMatrixSession: true,
          roomName: mapping.title,
          isDM: mapping.participants.length === 2,
          participants: mapping.participants
        };
      }
    }

    // If no local mapping, try to get the session from backend and extract metadata
    try {
      const { getSession } = await import('../api');
      const sessionResponse = await getSession({
        path: { session_id: sessionId },
        throwOnError: false
      });

      if (sessionResponse.data?.description) {
        const matrixMetadata = SessionMappingService.extractMatrixMetadataFromDescription(sessionResponse.data.description);
        if (matrixMetadata) {
          // Create a local mapping for future use
          this.createMappingFromBackendSession(sessionId, sessionResponse.data.description);
          return matrixMetadata;
        }
      }
    } catch (error) {
      console.warn('📋 SessionMappingService: Failed to fetch session from backend:', error);
    }

    return null;
  }

  /**
   * Clear all mappings (for testing/reset)
   */
  public clearAllMappings(): void {
    this.mappings.clear();
    localStorage.removeItem(this.STORAGE_KEY);
    console.log('📋 SessionMappingService: Cleared all mappings');
  }
}

// Export singleton instance
export const sessionMappingService = SessionMappingService.getInstance();
