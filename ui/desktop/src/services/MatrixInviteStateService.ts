/**
 * MatrixInviteStateService - Manages the state of Matrix collaboration invites
 * 
 * This service tracks which invites have been seen, accepted, or declined to prevent
 * duplicate notifications on reload/reconnect.
 */

export interface InviteState {
  roomId: string;
  inviter: string;
  inviterName: string;
  timestamp: number;
  status: 'pending' | 'accepted' | 'declined' | 'dismissed';
  lastSeen: number;
  autoJoined?: boolean; // If we automatically joined the room
}

export class MatrixInviteStateService {
  private static instance: MatrixInviteStateService;
  private readonly STORAGE_KEY = 'goose-matrix-invite-states';
  private inviteStates: Map<string, InviteState> = new Map();

  private constructor() {
    this.loadInviteStatesFromStorage();
  }

  public static getInstance(): MatrixInviteStateService {
    if (!MatrixInviteStateService.instance) {
      MatrixInviteStateService.instance = new MatrixInviteStateService();
    }
    return MatrixInviteStateService.instance;
  }

  /**
   * Record a new invite or update existing one
   */
  public recordInvite(roomId: string, inviter: string, inviterName: string): InviteState {
    const now = Date.now();
    const existingState = this.inviteStates.get(roomId);

    const inviteState: InviteState = {
      roomId,
      inviter,
      inviterName,
      timestamp: existingState?.timestamp || now,
      status: existingState?.status || 'pending',
      lastSeen: now,
      autoJoined: existingState?.autoJoined,
    };

    this.inviteStates.set(roomId, inviteState);
    this.saveInviteStatesToStorage();

    console.log('📋 MatrixInviteStateService: Recorded invite state:', {
      roomId,
      inviter,
      status: inviteState.status,
      isNew: !existingState,
    });

    return inviteState;
  }

  /**
   * Check if an invite should be shown (not already handled)
   */
  public shouldShowInvite(roomId: string, inviter: string): boolean {
    const state = this.inviteStates.get(roomId);
    
    if (!state) {
      // New invite, should be shown
      return true;
    }

    // Don't show if already accepted, declined, or dismissed
    if (state.status !== 'pending') {
      console.log('📋 MatrixInviteStateService: Hiding invite - already handled:', {
        roomId,
        status: state.status,
      });
      return false;
    }

    // Don't show if we've seen it recently (within last 5 minutes)
    const fiveMinutesAgo = Date.now() - (5 * 60 * 1000);
    if (state.lastSeen > fiveMinutesAgo) {
      console.log('📋 MatrixInviteStateService: Hiding invite - seen recently:', {
        roomId,
        lastSeen: new Date(state.lastSeen).toISOString(),
      });
      return false;
    }

    return true;
  }

  /**
   * Mark an invite as accepted
   */
  public acceptInvite(roomId: string): void {
    const state = this.inviteStates.get(roomId);
    if (state) {
      state.status = 'accepted';
      state.lastSeen = Date.now();
      this.saveInviteStatesToStorage();
      
      console.log('📋 MatrixInviteStateService: Marked invite as accepted:', roomId);
    }
  }

  /**
   * Mark an invite as declined
   */
  public declineInvite(roomId: string): void {
    const state = this.inviteStates.get(roomId);
    if (state) {
      state.status = 'declined';
      state.lastSeen = Date.now();
      this.saveInviteStatesToStorage();
      
      console.log('📋 MatrixInviteStateService: Marked invite as declined:', roomId);
    }
  }

  /**
   * Mark an invite as dismissed (user closed notification without action)
   */
  public dismissInvite(roomId: string): void {
    const state = this.inviteStates.get(roomId);
    if (state) {
      state.status = 'dismissed';
      state.lastSeen = Date.now();
      this.saveInviteStatesToStorage();
      
      console.log('📋 MatrixInviteStateService: Marked invite as dismissed:', roomId);
    }
  }

  /**
   * Mark an invite as auto-joined (we automatically joined the room)
   */
  public markAutoJoined(roomId: string): void {
    const state = this.inviteStates.get(roomId);
    if (state) {
      state.status = 'accepted';
      state.autoJoined = true;
      state.lastSeen = Date.now();
      this.saveInviteStatesToStorage();
      
      console.log('📋 MatrixInviteStateService: Marked invite as auto-joined:', roomId);
    }
  }

  /**
   * Get the state of a specific invite
   */
  public getInviteState(roomId: string): InviteState | null {
    return this.inviteStates.get(roomId) || null;
  }

  /**
   * Get all invite states (for debugging/management)
   */
  public getAllInviteStates(): InviteState[] {
    return Array.from(this.inviteStates.values());
  }

  /**
   * Get pending invites (that should potentially be shown)
   */
  public getPendingInvites(): InviteState[] {
    return Array.from(this.inviteStates.values()).filter(
      state => state.status === 'pending'
    );
  }

  /**
   * Clean up old invite states (older than 30 days)
   */
  public cleanupOldInvites(): void {
    const thirtyDaysAgo = Date.now() - (30 * 24 * 60 * 60 * 1000);
    let removedCount = 0;

    for (const [roomId, state] of this.inviteStates.entries()) {
      if (state.lastSeen < thirtyDaysAgo) {
        this.inviteStates.delete(roomId);
        removedCount++;
      }
    }

    if (removedCount > 0) {
      this.saveInviteStatesToStorage();
      console.log(`📋 MatrixInviteStateService: Cleaned up ${removedCount} old invite states`);
    }
  }

  /**
   * Reset invite state (mark as pending again) - useful for testing
   */
  public resetInviteState(roomId: string): void {
    const state = this.inviteStates.get(roomId);
    if (state) {
      state.status = 'pending';
      state.lastSeen = Date.now() - (10 * 60 * 1000); // 10 minutes ago to allow showing
      this.saveInviteStatesToStorage();
      
      console.log('📋 MatrixInviteStateService: Reset invite state:', roomId);
    }
  }

  /**
   * Clear all invite states (for testing/reset)
   */
  public clearAllInviteStates(): void {
    this.inviteStates.clear();
    localStorage.removeItem(this.STORAGE_KEY);
    console.log('📋 MatrixInviteStateService: Cleared all invite states');
  }

  /**
   * Load invite states from localStorage
   */
  private loadInviteStatesFromStorage(): void {
    try {
      const stored = localStorage.getItem(this.STORAGE_KEY);
      if (stored) {
        const data = JSON.parse(stored);
        this.inviteStates = new Map(Object.entries(data));
        console.log(`📋 MatrixInviteStateService: Loaded ${this.inviteStates.size} invite states from storage`);
        
        // Clean up old invites on load
        this.cleanupOldInvites();
      }
    } catch (error) {
      console.error('📋 MatrixInviteStateService: Error loading invite states from storage:', error);
      this.inviteStates = new Map();
    }
  }

  /**
   * Save invite states to localStorage
   */
  private saveInviteStatesToStorage(): void {
    try {
      const data = Object.fromEntries(this.inviteStates.entries());
      localStorage.setItem(this.STORAGE_KEY, JSON.stringify(data));
    } catch (error) {
      console.error('📋 MatrixInviteStateService: Error saving invite states to storage:', error);
    }
  }

  /**
   * Get statistics about invite states
   */
  public getInviteStats(): {
    total: number;
    pending: number;
    accepted: number;
    declined: number;
    dismissed: number;
  } {
    const states = Array.from(this.inviteStates.values());
    
    return {
      total: states.length,
      pending: states.filter(s => s.status === 'pending').length,
      accepted: states.filter(s => s.status === 'accepted').length,
      declined: states.filter(s => s.status === 'declined').length,
      dismissed: states.filter(s => s.status === 'dismissed').length,
    };
  }
}

// Export singleton instance
export const matrixInviteStateService = MatrixInviteStateService.getInstance();
