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

  constructor(config: MatrixConfig) {
    super();
    this.config = config;
  }

  /**
   * Initialize and start the Matrix client
   */
  async initialize(): Promise<void> {
    try {
      if (this.config.accessToken) {
        // Use existing access token
        this.client = sdk.createClient({
          baseUrl: this.config.homeserverUrl,
          accessToken: this.config.accessToken,
          userId: this.config.userId,
          deviceId: this.config.deviceId,
        });
      } else {
        // Create client for login
        this.client = sdk.createClient({
          baseUrl: this.config.homeserverUrl,
        });
      }

      this.setupEventListeners();
      
      if (this.config.accessToken) {
        await this.startSync();
      }

      this.emit('initialized');
    } catch (error) {
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

      this.emit('login', response);
    } catch (error) {
      this.emit('error', error);
      throw error;
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

      this.emit('register', response);
    } catch (error) {
      this.emit('error', error);
      throw error;
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
    
    // Check if this is a Goose AI message
    if (content.msgtype?.startsWith('m.goose.ai.')) {
      const aiMessage: GooseAIMessage = {
        type: content.msgtype.replace('m.goose.', '') as any,
        sessionId: content.session_id || room.roomId,
        content: content.body,
        model: content.model,
        sender,
        timestamp: new Date(event.getTs()),
        metadata: content.metadata,
      };
      
      this.emit('aiMessage', aiMessage);
    } else {
      // Regular message
      this.emit('message', {
        roomId: room.roomId,
        sender,
        content: content.body,
        timestamp: new Date(event.getTs()),
        event,
      });
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
      msgtype: 'm.goose.ai.prompt',
      body: prompt,
      session_id: sessionId,
      model,
      timestamp: Date.now(),
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
      msgtype: 'm.goose.ai.response',
      body: response,
      session_id: sessionId,
      model,
      timestamp: Date.now(),
    });
  }

  /**
   * Create a new room for AI collaboration
   */
  async createAISession(name: string, inviteUserIds: string[] = []): Promise<string> {
    if (!this.client) {
      throw new Error('Client not initialized');
    }

    const room = await this.client.createRoom({
      name: `🤖 ${name}`,
      topic: 'Collaborative AI Session with Goose',
      preset: 'private_chat',
      invite: inviteUserIds,
      initial_state: [
        {
          type: 'm.room.power_levels',
          content: {
            events: {
              'm.goose.ai.prompt': 0,
              'm.goose.ai.response': 50, // Only elevated users can send AI responses
            },
          },
        },
      ],
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
   * Disconnect from Matrix
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
}

// Export singleton instance
export const matrixService = new MatrixService({
  homeserverUrl: 'https://matrix.org', // Default to matrix.org, can be configured
});
