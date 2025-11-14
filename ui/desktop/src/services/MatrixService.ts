import * as sdk from 'matrix-js-sdk';
import { EventEmitter } from 'events';

export interface MatrixUser {
  userId: string;
  displayName?: string;
  avatarUrl?: string;
  presence?: 'online' | 'offline' | 'unavailable';
  lastActiveAgo?: number;
}

export interface MatrixRoom {
  roomId: string;
  name?: string;
  topic?: string;
  members: MatrixUser[];
  isDirectMessage: boolean;
  lastActivity?: Date;
}

export interface GooseAIMessage {
  type: 'ai.prompt' | 'ai.response' | 'ai.session.invite' | 'ai.session.join' | 'ai.session.leave';
  sessionId: string;
  content: string;
  model?: string;
  sender: string;
  timestamp: Date;
  metadata?: Record<string, any>;
}

export interface MatrixConfig {
  homeserverUrl: string;
  accessToken?: string;
  userId?: string;
  deviceId?: string;
}

export class MatrixService extends EventEmitter {
  private client: sdk.MatrixClient | null = null;
  private isConnected = false;
  private config: MatrixConfig;
  private syncState: 'PREPARED' | 'SYNCING' | 'ERROR' | 'STOPPED' = 'STOPPED';
  private readonly STORAGE_KEY = 'goose-matrix-credentials';

  constructor(config: MatrixConfig) {
    super();
    this.config = config;
  }

  /**
   * Save credentials to secure storage
   */
  private async saveCredentials(credentials: {
    accessToken: string;
    userId: string;
    deviceId: string;
    homeserverUrl: string;
  }): Promise<void> {
    try {
      // Use localStorage for now, but in production you'd want more secure storage
      const credentialsData = {
        accessToken: credentials.accessToken,
        userId: credentials.userId,
        deviceId: credentials.deviceId,
        homeserverUrl: credentials.homeserverUrl,
        timestamp: Date.now(),
      };
      
      localStorage.setItem(this.STORAGE_KEY, JSON.stringify(credentialsData));
      console.log('✅ Matrix credentials saved');
    } catch (error) {
      console.error('❌ Failed to save Matrix credentials:', error);
    }
  }

  /**
   * Load credentials from secure storage
   */
  private async loadCredentials(): Promise<MatrixConfig | null> {
    try {
      const stored = localStorage.getItem(this.STORAGE_KEY);
      if (!stored) {
        console.log('📭 No stored Matrix credentials found');
        return null;
      }

      const credentialsData = JSON.parse(stored);
      
      // Check if credentials are not too old (optional expiry check)
      const maxAge = 30 * 24 * 60 * 60 * 1000; // 30 days
      if (Date.now() - credentialsData.timestamp > maxAge) {
        console.log('⏰ Stored Matrix credentials are too old, clearing...');
        await this.clearCredentials();
        return null;
      }

      console.log('✅ Loaded stored Matrix credentials for:', credentialsData.userId);
      return {
        homeserverUrl: credentialsData.homeserverUrl,
        accessToken: credentialsData.accessToken,
        userId: credentialsData.userId,
        deviceId: credentialsData.deviceId,
      };
    } catch (error) {
      console.error('❌ Failed to load Matrix credentials:', error);
      return null;
    }
  }

  /**
   * Clear stored credentials
   */
  private async clearCredentials(): Promise<void> {
    try {
      localStorage.removeItem(this.STORAGE_KEY);
      console.log('🗑️ Matrix credentials cleared');
    } catch (error) {
      console.error('❌ Failed to clear Matrix credentials:', error);
    }
  }

  /**
   * Initialize and start the Matrix client
   */
  async initialize(): Promise<void> {
    try {
      // Try to load stored credentials first
      const storedConfig = await this.loadCredentials();
      if (storedConfig) {
        console.log('🔄 Attempting auto-login with stored credentials...');
        this.config = { ...this.config, ...storedConfig };
      }

      if (this.config.accessToken) {
        // Use existing access token (either passed in or loaded from storage)
        this.client = sdk.createClient({
          baseUrl: this.config.homeserverUrl,
          accessToken: this.config.accessToken,
          userId: this.config.userId,
          deviceId: this.config.deviceId,
        });
        
        this.setupEventListeners();
        
        try {
          await this.startSync();
          console.log('✅ Auto-login successful');
        } catch (syncError) {
          console.error('❌ Auto-login failed, clearing stored credentials:', syncError);
          await this.clearCredentials();
          // Reset config and create fresh client for manual login
          this.config = {
            homeserverUrl: this.config.homeserverUrl,
          };
          this.client = sdk.createClient({
            baseUrl: this.config.homeserverUrl,
          });
          this.setupEventListeners();
        }
      } else {
        // Create client for login
        this.client = sdk.createClient({
          baseUrl: this.config.homeserverUrl,
        });
        this.setupEventListeners();
      }

      this.emit('initialized');
    } catch (error) {
      console.error('❌ Matrix initialization failed:', error);
      this.emit('error', error);
      throw error;
    }
  }

  /**
   * Login with username/password
   */
  async login(username: string, password: string): Promise<void> {
    if (!this.client) {
      throw new Error('Client not initialized');
    }

    try {
      const response = await this.client.login('m.login.password', {
        user: username,
        password: password,
      });

      this.config.accessToken = response.access_token;
      this.config.userId = response.user_id;
      this.config.deviceId = response.device_id;

      // Recreate client with credentials
      this.client = sdk.createClient({
        baseUrl: this.config.homeserverUrl,
        accessToken: this.config.accessToken,
        userId: this.config.userId,
        deviceId: this.config.deviceId,
      });

      this.setupEventListeners();
      await this.startSync();

      // Save credentials for future auto-login
      await this.saveCredentials({
        accessToken: this.config.accessToken,
        userId: this.config.userId,
        deviceId: this.config.deviceId,
        homeserverUrl: this.config.homeserverUrl,
      });

      this.emit('login', response);
    } catch (error: any) {
      // Provide more helpful error messages
      let errorMessage = 'Login failed';
      
      if (error.httpStatus === 403) {
        if (error.data?.errcode === 'M_FORBIDDEN') {
          errorMessage = 'Invalid username or password. Please check your credentials.';
        } else {
          errorMessage = 'Access forbidden. Please check your credentials or try a different homeserver.';
        }
      } else if (error.httpStatus === 429) {
        errorMessage = 'Too many login attempts. Please wait a moment and try again.';
      } else if (error.httpStatus >= 500) {
        errorMessage = 'Server error. Please try again later or use a different homeserver.';
      } else if (error.name === 'ConnectionError' || error.code === 'NETWORK_ERROR') {
        errorMessage = 'Network error. Please check your internet connection.';
      } else if (error.data?.error) {
        errorMessage = error.data.error;
      }

      const enhancedError = new Error(errorMessage);
      this.emit('error', enhancedError);
      throw enhancedError;
    }
  }

  /**
   * Register a new account
   */
  async register(username: string, password: string): Promise<void> {
    if (!this.client) {
      throw new Error('Client not initialized');
    }

    try {
      const response = await this.client.register(username, password);
      
      this.config.accessToken = response.access_token;
      this.config.userId = response.user_id;
      this.config.deviceId = response.device_id;

      // Recreate client with credentials
      this.client = sdk.createClient({
        baseUrl: this.config.homeserverUrl,
        accessToken: this.config.accessToken,
        userId: this.config.userId,
        deviceId: this.config.deviceId,
      });

      this.setupEventListeners();
      await this.startSync();

      // Save credentials for future auto-login
      await this.saveCredentials({
        accessToken: this.config.accessToken,
        userId: this.config.userId,
        deviceId: this.config.deviceId,
        homeserverUrl: this.config.homeserverUrl,
      });

      this.emit('register', response);
    } catch (error: any) {
      // Provide more helpful error messages for registration
      let errorMessage = 'Registration failed';
      
      if (error.httpStatus === 400) {
        if (error.data?.errcode === 'M_USER_IN_USE') {
          errorMessage = 'Username is already taken. Please choose a different username.';
        } else if (error.data?.errcode === 'M_INVALID_USERNAME') {
          errorMessage = 'Invalid username format. Use only letters, numbers, and underscores.';
        } else if (error.data?.errcode === 'M_PASSWORD_TOO_SHORT') {
          errorMessage = 'Password is too short. Please use at least 12 characters.';
        } else if (error.data?.errcode === 'M_WEAK_PASSWORD') {
          errorMessage = 'Password is too weak. Please use a stronger password.';
        } else if (error.data?.error) {
          errorMessage = error.data.error;
        }
      } else if (error.httpStatus === 403) {
        if (error.data?.errcode === 'M_FORBIDDEN') {
          errorMessage = 'Registration is disabled on this homeserver. Please try a different homeserver.';
        } else {
          errorMessage = 'Registration not allowed. Please try a different homeserver.';
        }
      } else if (error.httpStatus === 429) {
        errorMessage = 'Too many registration attempts. Please wait a moment and try again.';
      } else if (error.httpStatus >= 500) {
        errorMessage = 'Server error. Please try again later or use a different homeserver.';
      } else if (error.name === 'ConnectionError' || error.code === 'NETWORK_ERROR') {
        errorMessage = 'Network error. Please check your internet connection.';
      } else if (error.data?.error) {
        errorMessage = error.data.error;
      }

      const enhancedError = new Error(errorMessage);
      this.emit('error', enhancedError);
      throw enhancedError;
    }
  }

  /**
   * Start syncing with the Matrix server
   */
  private async startSync(): Promise<void> {
    if (!this.client) {
      throw new Error('Client not initialized');
    }

    await this.client.startClient({ initialSyncLimit: 10 });
    this.syncState = 'SYNCING';
    this.isConnected = true;
    this.emit('connected');
  }

  /**
   * Setup event listeners for Matrix events
   */
  private setupEventListeners(): void {
    if (!this.client) return;

    this.client.on('sync', (state, prevState, data) => {
      this.syncState = state;
      this.emit('sync', { state, prevState, data });
      
      if (state === 'PREPARED') {
        this.emit('ready');
      }
    });

    this.client.on('Room.timeline', (event, room, toStartOfTimeline) => {
      if (event.getType() === 'm.room.message') {
        this.handleMessage(event, room);
      }
    });

    this.client.on('RoomMember.membership', (event, member) => {
      this.emit('membershipChange', { event, member });
    });

    this.client.on('User.presence', (event, user) => {
      this.emit('presenceChange', { event, user });
    });
  }

  /**
   * Handle incoming messages and emit appropriate events
   */
  private handleMessage(event: any, room: any): void {
    const content = event.getContent();
    const sender = event.getSender();
    
    // Skip messages from ourselves to avoid echo
    if (sender === this.config.userId) {
      return;
    }
    
    const messageData = {
      roomId: room.roomId,
      sender,
      content: content.body,
      timestamp: new Date(event.getTs()),
      event,
    };
    
    // Check if this is a Goose AI message (using custom properties)
    if (content['goose.type']) {
      const aiMessage: GooseAIMessage = {
        type: `ai.${content['goose.type']}` as any,
        sessionId: content['goose.session_id'] || room.roomId,
        content: content.body,
        model: content['goose.model'],
        sender,
        timestamp: new Date(content['goose.timestamp'] || event.getTs()),
        metadata: content,
      };
      
      this.emit('aiMessage', aiMessage);
    } else {
      // Regular message - emit both regular message event and check for session messages
      this.emit('message', messageData);
      
      // Also check if this is a session-related message and emit as session message
      if (content.body) {
        // Check for Goose session messages (from useSessionSharing)
        if (content.body.includes('goose-session-message:') || 
            content.body.includes('goose-session-invite:') || 
            content.body.includes('goose-session-joined:')) {
          this.emit('sessionMessage', messageData);
        }
      }
    }
  }

  /**
   * Send a regular text message to a room
   */
  async sendMessage(roomId: string, message: string): Promise<void> {
    if (!this.client) {
      throw new Error('Client not initialized');
    }

    await this.client.sendEvent(roomId, 'm.room.message', {
      msgtype: 'm.text',
      body: message,
    });
  }

  /**
   * Send an AI prompt to a collaborative session
   */
  async sendAIPrompt(roomId: string, prompt: string, sessionId: string, model?: string): Promise<void> {
    if (!this.client) {
      throw new Error('Client not initialized');
    }

    await this.client.sendEvent(roomId, 'm.room.message', {
      msgtype: 'm.text',
      body: `🤖 AI Prompt: ${prompt}`,
      format: 'org.matrix.custom.html',
      formatted_body: `<strong>🤖 AI Prompt:</strong><br/>${prompt}`,
      'goose.session_id': sessionId,
      'goose.type': 'prompt',
      'goose.model': model,
      'goose.timestamp': Date.now(),
    });
  }

  /**
   * Send an AI response to a collaborative session
   */
  async sendAIResponse(roomId: string, response: string, sessionId: string, model?: string): Promise<void> {
    if (!this.client) {
      throw new Error('Client not initialized');
    }

    await this.client.sendEvent(roomId, 'm.room.message', {
      msgtype: 'm.text',
      body: `🤖 AI Response: ${response}`,
      format: 'org.matrix.custom.html',
      formatted_body: `<strong>🤖 AI Response:</strong><br/>${response}`,
      'goose.session_id': sessionId,
      'goose.type': 'response',
      'goose.model': model,
      'goose.timestamp': Date.now(),
    });
  }

  /**
   * Create a new room for AI collaboration
   */
  async createAISession(name: string, inviteUserIds: string[] = []): Promise<string> {
    if (!this.client) {
      throw new Error('Client not initialized');
    }

    // Create a simple private room without complex power levels
    const room = await this.client.createRoom({
      name: `🤖 ${name}`,
      topic: 'Collaborative AI Session with Goose',
      preset: 'private_chat',
      invite: inviteUserIds,
      // Remove initial_state to avoid permission issues
      // We'll use regular message types instead of custom ones
    });

    return room.room_id;
  }

  /**
   * Invite a user to an existing room
   */
  async inviteToRoom(roomId: string, userId: string): Promise<void> {
    if (!this.client) {
      throw new Error('Client not initialized');
    }

    await this.client.invite(roomId, userId);
  }

  /**
   * Get all rooms the user is in
   */
  getRooms(): MatrixRoom[] {
    if (!this.client) return [];

    return this.client.getRooms().map(room => ({
      roomId: room.roomId,
      name: room.name,
      topic: room.currentState.getStateEvents('m.room.topic', '')?.getContent()?.topic,
      members: room.getMembers().map(member => ({
        userId: member.userId,
        displayName: member.name,
        avatarUrl: member.getAvatarUrl(this.config.homeserverUrl, 32, 32, 'crop'),
        presence: this.client?.getUser(member.userId)?.presence,
      })),
      isDirectMessage: room.getMembers().length === 2,
      lastActivity: new Date(room.getLastActiveTimestamp()),
    }));
  }

  /**
   * Get friends (users in direct message rooms)
   */
  getFriends(): MatrixUser[] {
    const friends = new Map<string, MatrixUser>();
    
    this.getRooms()
      .filter(room => room.isDirectMessage)
      .forEach(room => {
        room.members.forEach(member => {
          if (member.userId !== this.config.userId) {
            friends.set(member.userId, member);
          }
        });
      });

    return Array.from(friends.values());
  }

  /**
   * Search for users by display name or user ID
   */
  async searchUsers(query: string): Promise<MatrixUser[]> {
    if (!this.client) {
      throw new Error('Client not initialized');
    }

    try {
      const result = await this.client.searchUserDirectory({ term: query });
      return result.results.map(user => ({
        userId: user.user_id,
        displayName: user.display_name,
        avatarUrl: user.avatar_url,
      }));
    } catch (error) {
      console.error('Error searching users:', error);
      return [];
    }
  }

  /**
   * Create a direct message room with a user
   */
  async createDirectMessage(userId: string): Promise<string> {
    if (!this.client) {
      throw new Error('Client not initialized');
    }

    const room = await this.client.createRoom({
      preset: 'private_chat',
      invite: [userId],
      is_direct: true,
    });

    return room.room_id;
  }

  /**
   * Logout and clear stored credentials
   */
  async logout(): Promise<void> {
    try {
      // Try to logout from the server if we have a client
      if (this.client) {
        try {
          await this.client.logout();
        } catch (error) {
          console.warn('Server logout failed, continuing with local logout:', error);
        }
      }
    } finally {
      // Always clear local state and credentials
      await this.disconnect();
      await this.clearCredentials();
      
      // Reset config to initial state
      this.config = {
        homeserverUrl: this.config.homeserverUrl,
      };
      
      this.emit('logout');
    }
  }

  /**
   * Disconnect from Matrix (without clearing credentials)
   */
  async disconnect(): Promise<void> {
    if (this.client) {
      this.client.stopClient();
      this.isConnected = false;
      this.syncState = 'STOPPED';
      this.emit('disconnected');
    }
  }

  /**
   * Get current connection status
   */
  getConnectionStatus(): { connected: boolean; syncState: string } {
    return {
      connected: this.isConnected,
      syncState: this.syncState,
    };
  }

  /**
   * Get current user info
   */
  getCurrentUser(): MatrixUser | null {
    if (!this.client || !this.config.userId) return null;

    const user = this.client.getUser(this.config.userId);
    return {
      userId: this.config.userId,
      displayName: user?.displayName,
      avatarUrl: user?.avatarUrl,
      presence: user?.presence,
    };
  }

  /**
   * Upload and set user avatar
   */
  async setAvatar(file: File): Promise<string> {
    if (!this.client) {
      throw new Error('Client not initialized');
    }

    try {
      // Upload the file to Matrix media repository
      const uploadResponse = await this.client.uploadContent(file, {
        name: file.name,
        type: file.type,
      });

      const avatarUrl = uploadResponse.content_uri;

      // Set the avatar URL in the user's profile
      await this.client.setAvatarUrl(avatarUrl);

      // Emit avatar updated event
      this.emit('avatarUpdated', avatarUrl);

      return avatarUrl;
    } catch (error) {
      console.error('Failed to set avatar:', error);
      throw new Error('Failed to upload and set avatar');
    }
  }

  /**
   * Remove user avatar
   */
  async removeAvatar(): Promise<void> {
    if (!this.client) {
      throw new Error('Client not initialized');
    }

    try {
      await this.client.setAvatarUrl('');
      this.emit('avatarUpdated', null);
    } catch (error) {
      console.error('Failed to remove avatar:', error);
      throw new Error('Failed to remove avatar');
    }
  }

  /**
   * Update user display name
   */
  async setDisplayName(displayName: string): Promise<void> {
    if (!this.client) {
      throw new Error('Client not initialized');
    }

    try {
      await this.client.setDisplayName(displayName);
      this.emit('displayNameUpdated', displayName);
    } catch (error) {
      console.error('Failed to set display name:', error);
      throw new Error('Failed to update display name');
    }
  }
}

// Export singleton instance
export const matrixService = new MatrixService({
  homeserverUrl: 'https://matrix.tchncs.de', // Tchncs.de homeserver with open registration
});
