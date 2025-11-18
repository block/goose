/**
 * UserContextService - Manages user information and context for collaborative sessions
 * 
 * This service helps Goose understand who users are when they're introduced,
 * tracks user preferences, and maintains context about participants in sessions.
 */

export interface UserProfile {
  userId: string;
  displayName?: string;
  avatarUrl?: string;
  // User context information
  preferredName?: string; // What they like to be called
  role?: string; // Their role/title (e.g., "developer", "designer", "manager")
  expertise?: string[]; // Areas of expertise
  timezone?: string;
  workingHours?: {
    start: string; // e.g., "09:00"
    end: string; // e.g., "17:00"
    days: string[]; // e.g., ["monday", "tuesday", "wednesday", "thursday", "friday"]
  };
  preferences?: {
    communicationStyle?: 'formal' | 'casual' | 'technical';
    responseLength?: 'brief' | 'detailed' | 'comprehensive';
    notificationLevel?: 'all' | 'mentions' | 'minimal';
  };
  // Context from conversations
  introducedBy?: string; // Who introduced them
  introducedAt?: Date;
  lastSeen?: Date;
  commonTopics?: string[]; // Topics they frequently discuss
  collaborationHistory?: {
    sessionId: string;
    role: 'owner' | 'collaborator';
    joinedAt: Date;
    leftAt?: Date;
  }[];
  // Custom notes about the user
  notes?: string;
}

export interface UserIntroduction {
  introducedUserId: string;
  introducedBy: string;
  sessionId: string;
  timestamp: Date;
  context: string; // The original introduction message
  extractedInfo?: {
    name?: string;
    role?: string;
    expertise?: string[];
    relationship?: string; // e.g., "colleague", "friend", "client"
  };
}

class UserContextService {
  private readonly STORAGE_KEY = 'goose-user-context';
  private userProfiles = new Map<string, UserProfile>();
  private introductions: UserIntroduction[] = [];
  private isInitialized = false;

  /**
   * Initialize the service by loading stored data
   */
  async initialize(): Promise<void> {
    if (this.isInitialized) return;

    try {
      const stored = localStorage.getItem(this.STORAGE_KEY);
      if (stored) {
        const data = JSON.parse(stored);
        
        // Load user profiles
        if (data.profiles) {
          Object.entries(data.profiles).forEach(([userId, profile]) => {
            this.userProfiles.set(userId, this.deserializeUserProfile(profile as any));
          });
        }
        
        // Load introductions
        if (data.introductions) {
          this.introductions = data.introductions.map((intro: any) => ({
            ...intro,
            timestamp: new Date(intro.timestamp),
          }));
        }
        
        console.log('✅ UserContextService: Loaded', this.userProfiles.size, 'user profiles and', this.introductions.length, 'introductions');
      }
    } catch (error) {
      console.error('❌ UserContextService: Failed to load stored data:', error);
    }

    this.isInitialized = true;
  }

  /**
   * Save data to storage
   */
  private async saveToStorage(): Promise<void> {
    try {
      const data = {
        profiles: Object.fromEntries(
          Array.from(this.userProfiles.entries()).map(([userId, profile]) => [
            userId,
            this.serializeUserProfile(profile)
          ])
        ),
        introductions: this.introductions,
        lastUpdated: Date.now(),
      };
      
      localStorage.setItem(this.STORAGE_KEY, JSON.stringify(data));
    } catch (error) {
      console.error('❌ UserContextService: Failed to save data:', error);
    }
  }

  /**
   * Serialize user profile for storage (handle dates)
   */
  private serializeUserProfile(profile: UserProfile): any {
    return {
      ...profile,
      introducedAt: profile.introducedAt?.toISOString(),
      lastSeen: profile.lastSeen?.toISOString(),
      collaborationHistory: profile.collaborationHistory?.map(history => ({
        ...history,
        joinedAt: history.joinedAt.toISOString(),
        leftAt: history.leftAt?.toISOString(),
      })),
    };
  }

  /**
   * Deserialize user profile from storage (handle dates)
   */
  private deserializeUserProfile(data: any): UserProfile {
    return {
      ...data,
      introducedAt: data.introducedAt ? new Date(data.introducedAt) : undefined,
      lastSeen: data.lastSeen ? new Date(data.lastSeen) : undefined,
      collaborationHistory: data.collaborationHistory?.map((history: any) => ({
        ...history,
        joinedAt: new Date(history.joinedAt),
        leftAt: history.leftAt ? new Date(history.leftAt) : undefined,
      })),
    };
  }

  /**
   * Process a user introduction from a message
   */
  async processIntroduction(
    message: string,
    introducedBy: string,
    sessionId: string,
    mentionedUserIds?: string[]
  ): Promise<UserIntroduction[]> {
    await this.initialize();

    const introductions: UserIntroduction[] = [];
    
    // Parse introduction patterns
    const introPatterns = [
      // "meet John" or "this is John"
      /(?:meet|this is|let me introduce|say hello to)\s+([A-Za-z][A-Za-z\s]+?)(?:\s|$|,|\.|!)/gi,
      // "John is a developer"
      /([A-Za-z][A-Za-z\s]+?)\s+is\s+(?:a|an|the)\s+([A-Za-z\s]+?)(?:\s|$|,|\.|!)/gi,
      // "@username meet John" or "hey @username, this is John"
      /@\w+[,\s]*(?:meet|this is|say hello to)\s+([A-Za-z][A-Za-z\s]+?)(?:\s|$|,|\.|!)/gi,
    ];

    const extractedNames = new Set<string>();
    
    // Extract names from patterns
    introPatterns.forEach(pattern => {
      let match;
      while ((match = pattern.exec(message)) !== null) {
        const name = match[1]?.trim();
        if (name && name.length > 1 && name.length < 50) {
          extractedNames.add(name);
        }
      }
    });

    // Also look for role/expertise information
    const rolePatterns = [
      /([A-Za-z][A-Za-z\s]+?)\s+is\s+(?:a|an|the)\s+(developer|designer|manager|engineer|analyst|consultant|architect|lead|senior|junior|intern|freelancer|contractor)/gi,
      /([A-Za-z][A-Za-z\s]+?)\s+works?\s+(?:as|in|on)\s+([A-Za-z\s]+)/gi,
      /([A-Za-z][A-Za-z\s]+?)\s+specializes?\s+in\s+([A-Za-z\s]+)/gi,
    ];

    const extractedRoles = new Map<string, string>();
    rolePatterns.forEach(pattern => {
      let match;
      while ((match = pattern.exec(message)) !== null) {
        const name = match[1]?.trim();
        const role = match[2]?.trim();
        if (name && role) {
          extractedRoles.set(name, role);
        }
      }
    });

    // Create introductions for extracted names
    for (const name of extractedNames) {
      // Try to match with mentioned user IDs if available
      let userId = `introduced:${name.toLowerCase().replace(/\s+/g, '_')}`;
      
      // If we have mentioned user IDs, try to match by display name
      if (mentionedUserIds && mentionedUserIds.length > 0) {
        // For now, use the first mentioned user ID
        // In a real implementation, you'd want to match by display name
        userId = mentionedUserIds[0];
      }

      const introduction: UserIntroduction = {
        introducedUserId: userId,
        introducedBy,
        sessionId,
        timestamp: new Date(),
        context: message,
        extractedInfo: {
          name,
          role: extractedRoles.get(name),
          expertise: this.extractExpertise(message, name),
          relationship: this.extractRelationship(message, name),
        },
      };

      introductions.push(introduction);
      this.introductions.push(introduction);

      // Create or update user profile
      await this.createOrUpdateUserProfile(userId, {
        displayName: name,
        preferredName: name,
        role: extractedRoles.get(name),
        expertise: this.extractExpertise(message, name),
        introducedBy,
        introducedAt: new Date(),
        lastSeen: new Date(),
      });

      console.log('👋 UserContextService: Processed introduction for', name, '→', userId);
    }

    if (introductions.length > 0) {
      await this.saveToStorage();
    }

    return introductions;
  }

  /**
   * Extract expertise from introduction message
   */
  private extractExpertise(message: string, name: string): string[] {
    const expertise: string[] = [];
    
    const expertisePatterns = [
      new RegExp(`${name}\\s+(?:knows|works with|specializes in|is expert in)\\s+([A-Za-z\\s,]+)`, 'gi'),
      new RegExp(`${name}\\s+(?:does|handles)\\s+([A-Za-z\\s,]+)`, 'gi'),
    ];

    expertisePatterns.forEach(pattern => {
      const match = pattern.exec(message);
      if (match && match[1]) {
        const skills = match[1].split(/[,&]/).map(s => s.trim()).filter(s => s.length > 0);
        expertise.push(...skills);
      }
    });

    return expertise;
  }

  /**
   * Extract relationship from introduction message
   */
  private extractRelationship(message: string, name: string): string | undefined {
    const relationshipPatterns = [
      new RegExp(`${name}\\s+is\\s+(?:my|our)\\s+(colleague|friend|client|partner|manager|teammate)`, 'gi'),
      new RegExp(`(?:my|our)\\s+(colleague|friend|client|partner|manager|teammate)\\s+${name}`, 'gi'),
    ];

    for (const pattern of relationshipPatterns) {
      const match = pattern.exec(message);
      if (match && match[1]) {
        return match[1].toLowerCase();
      }
    }

    return undefined;
  }

  /**
   * Create or update a user profile
   */
  async createOrUpdateUserProfile(
    userId: string,
    updates: Partial<UserProfile>
  ): Promise<UserProfile> {
    await this.initialize();

    const existing = this.userProfiles.get(userId);
    const profile: UserProfile = {
      userId,
      ...existing,
      ...updates,
      lastSeen: new Date(),
    };

    // Merge arrays instead of replacing
    if (existing?.expertise && updates.expertise) {
      profile.expertise = [...new Set([...existing.expertise, ...updates.expertise])];
    }

    if (existing?.commonTopics && updates.commonTopics) {
      profile.commonTopics = [...new Set([...existing.commonTopics, ...updates.commonTopics])];
    }

    this.userProfiles.set(userId, profile);
    await this.saveToStorage();

    console.log('👤 UserContextService: Updated profile for', userId, profile.displayName || profile.preferredName);
    return profile;
  }

  /**
   * Get user profile by ID
   */
  async getUserProfile(userId: string): Promise<UserProfile | null> {
    await this.initialize();
    return this.userProfiles.get(userId) || null;
  }

  /**
   * Search for users by name
   */
  async searchUsersByName(name: string): Promise<UserProfile[]> {
    await this.initialize();
    
    const searchTerm = name.toLowerCase();
    return Array.from(this.userProfiles.values()).filter(profile => 
      profile.displayName?.toLowerCase().includes(searchTerm) ||
      profile.preferredName?.toLowerCase().includes(searchTerm) ||
      profile.userId.toLowerCase().includes(searchTerm)
    );
  }

  /**
   * Get all user profiles
   */
  async getAllUserProfiles(): Promise<UserProfile[]> {
    await this.initialize();
    return Array.from(this.userProfiles.values());
  }

  /**
   * Get introductions for a session
   */
  async getSessionIntroductions(sessionId: string): Promise<UserIntroduction[]> {
    await this.initialize();
    return this.introductions.filter(intro => intro.sessionId === sessionId);
  }

  /**
   * Update user's last seen timestamp
   */
  async updateLastSeen(userId: string): Promise<void> {
    const profile = await this.getUserProfile(userId);
    if (profile) {
      await this.createOrUpdateUserProfile(userId, { lastSeen: new Date() });
    }
  }

  /**
   * Add collaboration history entry
   */
  async addCollaborationHistory(
    userId: string,
    sessionId: string,
    role: 'owner' | 'collaborator'
  ): Promise<void> {
    const profile = await this.getUserProfile(userId);
    if (profile) {
      const history = profile.collaborationHistory || [];
      history.push({
        sessionId,
        role,
        joinedAt: new Date(),
      });
      
      await this.createOrUpdateUserProfile(userId, {
        collaborationHistory: history,
      });
    }
  }

  /**
   * Generate user context summary for AI
   */
  async generateUserContextSummary(sessionId: string): Promise<string> {
    await this.initialize();

    const introductions = await this.getSessionIntroductions(sessionId);
    if (introductions.length === 0) {
      return '';
    }

    const contextLines: string[] = [
      '## User Context',
      '',
      'The following users have been introduced in this session:',
      '',
    ];

    for (const intro of introductions) {
      const profile = await this.getUserProfile(intro.introducedUserId);
      const info = intro.extractedInfo;
      
      let userLine = `- **${info?.name || profile?.displayName || intro.introducedUserId}**`;
      
      if (info?.role || profile?.role) {
        userLine += ` (${info?.role || profile?.role})`;
      }
      
      if (info?.expertise || profile?.expertise) {
        const expertise = info?.expertise || profile?.expertise || [];
        if (expertise.length > 0) {
          userLine += ` - Expertise: ${expertise.join(', ')}`;
        }
      }
      
      if (info?.relationship) {
        userLine += ` - Relationship: ${info.relationship}`;
      }
      
      contextLines.push(userLine);
      
      if (profile?.notes) {
        contextLines.push(`  - Notes: ${profile.notes}`);
      }
    }

    contextLines.push('');
    contextLines.push('Use this context to personalize your interactions with these users.');

    return contextLines.join('\n');
  }

  /**
   * Check if a message contains user introductions
   */
  containsIntroductions(message: string): boolean {
    const introKeywords = [
      'meet', 'this is', 'let me introduce', 'say hello to',
      'is a', 'is an', 'works as', 'specializes in'
    ];
    
    const lowerMessage = message.toLowerCase();
    return introKeywords.some(keyword => lowerMessage.includes(keyword));
  }

  /**
   * Clear all user context data
   */
  async clearAllData(): Promise<void> {
    this.userProfiles.clear();
    this.introductions = [];
    localStorage.removeItem(this.STORAGE_KEY);
    console.log('🗑️ UserContextService: Cleared all user context data');
  }

  /**
   * Export user context data
   */
  async exportData(): Promise<any> {
    await this.initialize();
    
    return {
      profiles: Object.fromEntries(
        Array.from(this.userProfiles.entries()).map(([userId, profile]) => [
          userId,
          this.serializeUserProfile(profile)
        ])
      ),
      introductions: this.introductions,
      exportedAt: new Date().toISOString(),
    };
  }

  /**
   * Import user context data
   */
  async importData(data: any): Promise<void> {
    try {
      if (data.profiles) {
        this.userProfiles.clear();
        Object.entries(data.profiles).forEach(([userId, profile]) => {
          this.userProfiles.set(userId, this.deserializeUserProfile(profile as any));
        });
      }
      
      if (data.introductions) {
        this.introductions = data.introductions.map((intro: any) => ({
          ...intro,
          timestamp: new Date(intro.timestamp),
        }));
      }
      
      await this.saveToStorage();
      console.log('✅ UserContextService: Imported user context data');
    } catch (error) {
      console.error('❌ UserContextService: Failed to import data:', error);
      throw error;
    }
  }
}

// Export singleton instance
export const userContextService = new UserContextService();

// Expose for debugging
if (typeof window !== 'undefined') {
  (window as any).userContextService = userContextService;
  
  // Debug helpers
  (window as any).debugUserContext = async () => {
    console.log('=== USER CONTEXT DEBUG ===');
    const profiles = await userContextService.getAllUserProfiles();
    console.log('User profiles:', profiles);
    
    const data = await userContextService.exportData();
    console.log('Full context data:', data);
  };
  
  (window as any).testUserIntroduction = async (message: string) => {
    console.log('🧪 Testing user introduction:', message);
    const introductions = await userContextService.processIntroduction(
      message,
      'test-user',
      'test-session'
    );
    console.log('Detected introductions:', introductions);
    
    const summary = await userContextService.generateUserContextSummary('test-session');
    console.log('Generated summary:', summary);
  };
}
