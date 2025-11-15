/**
 * SessionMappingService - Manages the relationship between Matrix room IDs and Goose session IDs
 * 
 * This service enables hybrid session management where:
 * - Matrix room IDs (e.g., "!abc123:server.com") are used for Matrix operations
 * - Goose session IDs (e.g., "20251115_143022") are used for backend API calls
 * - The mapping ensures both systems work harmoniously
 */

export interface SessionMapping {
  matrixRoomId: string;
  gooseSessionId: string;
  createdAt: number;
  lastUsed: number;
  participants: string[]; // Matrix user IDs
  title?: string;
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
    title?: string
  ): Promise<SessionMapping> {
    try {
      // Import startAgent dynamically to avoid circular dependencies
      const { startAgent } = await import('../api');
      
      // Create a backend session for this Matrix room
      const agentResponse = await startAgent({
        body: {
          working_dir: window.appConfig.get('GOOSE_WORKING_DIR') as string,
          // Create a recipe for Matrix collaboration
          recipe: {
            title: title || `Matrix Collaboration: ${matrixRoomId.substring(1, 8)}`,
            description: `Collaborative AI session for Matrix room ${matrixRoomId}`,
            instructions: [
              'You are participating in a collaborative AI session through Matrix.',
              'Multiple users may be participating in this conversation.',
              'Be helpful and collaborative in your responses.',
            ],
          },
        },
        throwOnError: true,
      });

      const backendSession = agentResponse.data;
      if (!backendSession?.id) {
        throw new Error('Failed to create backend session');
      }

      const now = Date.now();
      const mapping: SessionMapping = {
        matrixRoomId,
        gooseSessionId: backendSession.id, // Use the actual backend session ID
        createdAt: now,
        lastUsed: now,
        participants,
        title,
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
        this.mappings = new Map(Object.entries(data));
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
      const data = Object.fromEntries(this.mappings.entries());
      localStorage.setItem(this.STORAGE_KEY, JSON.stringify(data));
    } catch (error) {
      console.error('📋 SessionMappingService: Error saving mappings to storage:', error);
    }
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
