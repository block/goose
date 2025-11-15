import * as sdk from 'matrix-js-sdk';
import { EventEmitter } from 'events';
import { sessionMappingService, SessionMappingService } from './SessionMappingService';

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

export interface GooseChatMessage {
  type: 'goose.chat' | 'goose.command' | 'goose.task.request' | 'goose.task.response' | 'goose.collaboration.invite' | 'goose.collaboration.accept' | 'goose.collaboration.decline';
  messageId: string;
  content: string;
  sender: string;
  timestamp: Date;
  roomId: string;
  replyTo?: string; // Message ID this is replying to
  metadata?: {
    taskId?: string;
    taskType?: string;
    priority?: 'low' | 'medium' | 'high' | 'urgent';
    capabilities?: string[]; // What this Goose can do
    status?: 'pending' | 'in_progress' | 'completed' | 'failed';
    attachments?: Array<{
      type: 'file' | 'image' | 'code' | 'log';
      name: string;
      url?: string;
      content?: string;
    }>;
    [key: string]: any;
  };
}

export interface GooseInstance {
  userId: string;
  displayName?: string;
  avatarUrl?: string;
  presence?: 'online' | 'offline' | 'unavailable';
  capabilities?: string[]; // What this Goose can do (e.g., ['code', 'research', 'analysis'])
  version?: string;
  lastSeen?: Date;
  status?: 'idle' | 'busy' | 'working';
  currentTask?: string;
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
  private cachedCurrentUser: MatrixUser | null = null;
  private cachedFriends: MatrixUser[] | null = null;
  private cachedRooms: MatrixRoom[] | null = null;

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
   * Clear all caches
   */
  private clearAllCaches(): void {
    console.log('MatrixService - clearing all caches');
    this.cachedCurrentUser = null;
    this.cachedFriends = null;
    this.cachedRooms = null;
  }

  /**
   * Setup event listeners for Matrix events
   */
  private setupEventListeners(): void {
    if (!this.client) return;

    console.log('🔧 MatrixService: Setting up event listeners');

    this.client.on('sync', (state, prevState, data) => {
      console.log('🔄 MatrixService sync state:', state, '(was:', prevState, ')');
      this.syncState = state;
      this.emit('sync', { state, prevState, data });
      
      if (state === 'PREPARED') {
        // Clear all caches when sync is prepared to get fresh data
        this.clearAllCaches();
        console.log('✅ MatrixService: Sync prepared, emitting ready event');
        this.emit('ready');
      }
    });

    this.client.on('Room.timeline', (event, room, toStartOfTimeline) => {
      console.log('🔍 MatrixService: Room.timeline event:', {
        eventType: event.getType(),
        roomId: room.roomId,
        sender: event.getSender(),
        toStartOfTimeline
      });
      
      if (event.getType() === 'm.room.message') {
        this.handleMessage(event, room);
      }
    });

    this.client.on('RoomMember.membership', (event, member) => {
      // Clear caches when membership changes (affects friends and rooms)
      this.cachedFriends = null;
      this.cachedRooms = null;
      
      // Clear current user cache if our own membership changes
      if (member.userId === this.config.userId) {
        this.cachedCurrentUser = null;
      }
      
      // Handle Matrix room invitations for collaboration
      if (member.userId === this.config.userId && member.membership === 'invite') {
        console.log('🎯 Received Matrix room invitation:', {
          roomId: member.roomId,
          inviter: event.getSender(),
          membership: member.membership
        });
        
        // Emit a collaboration invite event for Matrix room invitations
        this.handleMatrixRoomInvitation(member.roomId, event.getSender());
      }
      
      this.emit('membershipChange', { event, member });
    });

    this.client.on('User.presence', (event, user) => {
      // Clear friends cache when any user's presence changes (affects friend presence)
      this.cachedFriends = null;
      
      // Clear current user cache if our own presence changes
      if (user.userId === this.config.userId) {
        this.cachedCurrentUser = null;
      }
      
      this.emit('presenceChange', { event, user });
    });

    // Listen for user profile updates
    this.client.on('User.avatarUrl', (event, user) => {
      if (user.userId === this.config.userId) {
        console.log('User.avatarUrl event - clearing current user cache for:', user.userId);
        this.cachedCurrentUser = null;
      } else {
        // Clear friends cache if any friend's avatar changes
        console.log('User.avatarUrl event - clearing friends cache for friend:', user.userId);
        this.cachedFriends = null;
        this.cachedRooms = null;
      }
    });

    this.client.on('User.displayName', (event, user) => {
      if (user.userId === this.config.userId) {
        console.log('User.displayName event - clearing current user cache for:', user.userId);
        this.cachedCurrentUser = null;
      } else {
        // Clear friends cache if any friend's display name changes
        console.log('User.displayName event - clearing friends cache for friend:', user.userId);
        this.cachedFriends = null;
        this.cachedRooms = null;
      }
    });
  }

  /**
   * Generate a unique message ID
   */
  private generateMessageId(): string {
    return `goose_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
  }

  /**
   * Handle Matrix room invitations and emit a direct invitation event
   */
  private handleMatrixRoomInvitation(roomId: string, inviter: string): void {
    console.log('🎯 Processing Matrix room invitation:', {
      roomId,
      inviter
    });

    // Get inviter information
    const inviterUser = this.client?.getUser(inviter);
    const inviterName = inviterUser?.displayName || inviter.split(':')[0].substring(1);

    // Emit a direct Matrix room invitation event (not a goose message)
    // This prevents duplicate notifications
    const invitationData = {
      roomId,
      inviter,
      inviterName,
      timestamp: new Date(),
      type: 'matrix_room_invitation',
    };

    console.log('🎯 Emitting Matrix room invitation event:', invitationData);
    
    // Emit as a Matrix-specific invitation event
    this.emit('matrixRoomInvitation', invitationData);
  }

  /**
   * Check if a user is a Goose instance based on their display name or user ID
   */
  private isGooseInstance(userId: string, displayName?: string): boolean {
    const gooseIndicators = ['goose', 'bot', 'ai', 'assistant'];
    const userIdLower = userId.toLowerCase();
    const displayNameLower = displayName?.toLowerCase() || '';
    
    return gooseIndicators.some(indicator => 
      userIdLower.includes(indicator) || displayNameLower.includes(indicator)
    );
  }

  /**
   * Check if a message appears to be from a Goose instance based on content patterns
   */
  private looksLikeGooseMessage(content: string): boolean {
    // Look for common Goose patterns in message content
    const goosePatterns = [
      /🦆/,  // Goose emoji
      /🤖/,  // Robot emoji
      /\[GOOSE\]/i,
      /\[AI\]/i,
      /\[ASSISTANT\]/i,
      /^(goose|ai|assistant):/i,  // Message starting with goose:, ai:, etc.
      /@goose/i,  // @goose mentions
      /collaborative.*session/i,
      /task.*request/i,
      /collaboration.*invite/i,
      /goose-session-message:/i,  // Session messages from useSessionSharing
    ];
    
    return goosePatterns.some(pattern => pattern.test(content));
  }

  /**
   * Check if a message contains @goose mention
   */
  private containsGooseMention(content: string): boolean {
    // Case-insensitive check for @goose mentions
    const goosePattern = /@goose\b/i;
    return goosePattern.test(content);
  }

  /**
   * Handle incoming messages and emit appropriate events
   */
  private handleMessage(event: any, room: any): void {
    const content = event.getContent();
    const sender = event.getSender();
    const isFromSelf = sender === this.config.userId;
    
    // Debug: Log all incoming messages for troubleshooting
    console.log('🔍 MatrixService.handleMessage called:', {
      roomId: room.roomId,
      sender,
      configUserId: this.config.userId,
      isFromSelf,
      senderEqualsConfig: sender === this.config.userId,
      contentBody: content.body?.substring(0, 100) + '...',
      eventType: event.getType(),
      timestamp: new Date(event.getTs())
    });
    
    // Get sender information
    const senderUser = this.client?.getUser(sender);
    const senderMember = room.getMember(sender);
    
    const senderInfo = {
      userId: sender,
      displayName: senderMember?.name || senderUser?.displayName || sender.split(':')[0].substring(1),
      avatarUrl: senderMember?.getMxcAvatarUrl() || senderUser?.avatarUrl || null,
    };
    
    const messageData = {
      roomId: room.roomId,
      sender,
      content: content.body,
      timestamp: new Date(event.getTs()),
      event,
      isFromSelf,
      senderInfo,
    };
    
    let isGooseMessage = false;
    let isSessionMessage = false;
    
    // Check if this is a structured Goose message (new format)
    if (content['goose.message.type']) {
      isGooseMessage = true;
      const gooseChatMessage: GooseChatMessage = {
        type: content['goose.message.type'] as any,
        messageId: content['goose.message.id'] || this.generateMessageId(),
        content: content.body,
        sender,
        timestamp: new Date(content['goose.timestamp'] || event.getTs()),
        roomId: room.roomId,
        replyTo: content['goose.reply_to'],
        metadata: {
          taskId: content['goose.task.id'],
          taskType: content['goose.task.type'],
          priority: content['goose.priority'],
          capabilities: content['goose.capabilities'],
          status: content['goose.status'],
          attachments: content['goose.attachments'],
          isFromSelf, // Add this to metadata so UI can distinguish
          ...content['goose.metadata'],
        },
      };
      
      console.log('🦆 Received structured Goose message:', gooseChatMessage.type, 'from:', sender, isFromSelf ? '(self)' : '(other)');
      this.emit('gooseMessage', gooseChatMessage);
    }
    
    // Check if this is a legacy Goose AI message (using custom properties)
    else if (content['goose.type']) {
      isGooseMessage = true;
      const aiMessage: GooseAIMessage = {
        type: `ai.${content['goose.type']}` as any,
        sessionId: content['goose.session_id'] || room.roomId,
        content: content.body,
        model: content['goose.model'],
        sender,
        timestamp: new Date(content['goose.timestamp'] || event.getTs()),
        metadata: { ...content, isFromSelf },
      };
      
      console.log('🦆 Received legacy Goose AI message:', aiMessage.type, 'from:', sender, isFromSelf ? '(self)' : '(other)');
      this.emit('aiMessage', aiMessage);
    }
    
    // Enhanced heuristic detection for Goose messages
    else if (!isFromSelf && (
      this.isGooseInstance(sender, senderUser?.displayName) || 
      this.looksLikeGooseMessage(content.body || '')
    )) {
      isGooseMessage = true;
      // Treat as potential Goose message even without explicit markers
      const gooseChatMessage: GooseChatMessage = {
        type: 'goose.chat',
        messageId: this.generateMessageId(),
        content: content.body,
        sender,
        timestamp: new Date(event.getTs()),
        roomId: room.roomId,
        metadata: { 
          isFromSelf: false,
          detectionMethod: this.isGooseInstance(sender, senderUser?.displayName) ? 'username' : 'content',
        },
      };
      
      console.log('🦆 Detected potential Goose message from:', sender, '- detection method:', gooseChatMessage.metadata?.detectionMethod);
      this.emit('gooseMessage', gooseChatMessage);
    }
    
    // Check if this is a session-related message or contains @goose mention
    if (content.body) {
      // Check for Goose session messages (from useSessionSharing)
      if (content.body.includes('goose-session-message:') || 
          content.body.includes('goose-session-invite:') || 
          content.body.includes('goose-session-joined:')) {
        isSessionMessage = true;
        console.log('📝 Received session message from:', sender, '- processing as session sync only');
        
        // For session messages, emit ONLY the gooseSessionSync event
        // This prevents them from being processed as regular messages
        this.emit('gooseSessionSync', messageData);
        return; // Exit early to prevent further processing
      }
      
      // Check for @goose mentions (only for non-session messages and not from self)
      if (!isFromSelf && this.containsGooseMention(content.body)) {
        console.log('🦆 Detected @goose mention in message from:', sender);
        this.emit('gooseMention', {
          ...messageData,
          mentionedGoose: true,
        });
      }
    }
    
    // FIXED: Always emit regular messages for display, regardless of sender
    // This ensures that the sender's own messages appear in BaseChat
    if (!isSessionMessage && !isGooseMessage) {
      // Check if this might be in a collaborative Matrix room by looking at room members
      const roomObj = this.client?.getRoom(messageData.roomId);
      const memberCount = roomObj?.getMembers()?.length || 0;
      
      // For multi-user rooms (more than 2 people), route through session sync for non-self messages
      // But still emit regular message event for display purposes
      if (!isFromSelf && memberCount > 2) {
        console.log('🔄 Multi-user room detected - routing message through session sync to prevent local AI response from:', sender);
        this.emit('gooseSessionSync', messageData);
      }
      
      // ALWAYS emit regular message for display (both self and others)
      console.log('💬 Emitting regular message for display from:', sender, isFromSelf ? '(self)' : '(other)');
      this.emit('message', messageData);
    }
  }

  /**
   * Send a regular text message to a room
   */
  async sendMessage(roomId: string, message: string): Promise<void> {
    if (!this.client) {
      throw new Error('Client not initialized');
    }

    // Send as a regular message without any Goose markers
    // This method is for user messages, not Goose messages
    const eventContent: any = {
      msgtype: 'm.text',
      body: message,
    };

    console.log('💬 Sending regular user message to room:', roomId, 'Message:', message.substring(0, 50) + '...');
    await this.client.sendEvent(roomId, 'm.room.message', eventContent);
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

    // Create session mapping for this Matrix room
    const participants = [this.config.userId!, ...inviteUserIds];
    const mapping = sessionMappingService.createMapping(room.room_id, participants, name);
    
    console.log('📋 Created AI session with mapping:', {
      matrixRoomId: room.room_id,
      gooseSessionId: mapping.gooseSessionId,
      participants: participants.length,
      name,
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
   * Join a Matrix room by room ID
   */
  async joinRoom(roomId: string): Promise<void> {
    if (!this.client) {
      throw new Error('Client not initialized');
    }

    try {
      console.log('🚪 Attempting to join room:', roomId);
      
      // Check if we're already in the room
      const existingRoom = this.client.getRoom(roomId);
      if (existingRoom && existingRoom.getMyMembership() === 'join') {
        console.log('✅ Already joined room:', roomId);
        
        // Still ensure session mapping exists for this room
        this.ensureSessionMapping(roomId, existingRoom);
        return;
      }

      // Join the room
      await this.client.joinRoom(roomId);
      console.log('✅ Successfully joined room:', roomId);
      
      // Clear caches to refresh room data
      this.cachedRooms = null;
      this.cachedFriends = null;
      
      // Get the room after joining to create session mapping
      const joinedRoom = this.client.getRoom(roomId);
      if (joinedRoom) {
        this.ensureSessionMapping(roomId, joinedRoom);
      }
      
      // Emit join event
      this.emit('roomJoined', { roomId });
      
    } catch (error: any) {
      console.error('❌ Failed to join room:', roomId, error);
      
      // Provide more helpful error messages
      let errorMessage = 'Failed to join room';
      
      if (error.httpStatus === 403) {
        if (error.data?.errcode === 'M_FORBIDDEN') {
          errorMessage = 'You are not invited to this room or it is private.';
        } else {
          errorMessage = 'Access forbidden. You may not have permission to join this room.';
        }
      } else if (error.httpStatus === 404) {
        errorMessage = 'Room not found. The room may have been deleted or the ID is incorrect.';
      } else if (error.httpStatus === 429) {
        errorMessage = 'Too many requests. Please wait a moment and try again.';
      } else if (error.httpStatus >= 500) {
        errorMessage = 'Server error. Please try again later.';
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
   * Ensure a session mapping exists for a Matrix room
   */
  private ensureSessionMapping(roomId: string, room: any): void {
    // Check if mapping already exists
    const existingMapping = sessionMappingService.getMapping(roomId);
    if (existingMapping) {
      console.log('📋 Session mapping already exists for room:', roomId, '→', existingMapping.gooseSessionId);
      
      // Update participants if needed
      const currentParticipants = room.getMembers().map((member: any) => member.userId);
      sessionMappingService.updateParticipants(roomId, currentParticipants);
      return;
    }

    // Create new mapping if none exists
    const participants = room.getMembers().map((member: any) => member.userId);
    const roomName = room.name || `Matrix Room ${roomId.substring(1, 8)}`;
    
    const mapping = sessionMappingService.createMapping(roomId, participants, roomName);
    
    console.log('📋 Created session mapping for joined room:', {
      matrixRoomId: roomId,
      gooseSessionId: mapping.gooseSessionId,
      participants: participants.length,
      roomName,
    });
  }

  /**
   * Get all rooms the user is in
   */
  getRooms(): MatrixRoom[] {
    if (!this.client) return [];

    // Return cached rooms if available
    if (this.cachedRooms) {
      console.log('getRooms - returning cached rooms:', this.cachedRooms.length);
      return this.cachedRooms;
    }

    console.log('getRooms - fetching fresh room data');
    
    this.cachedRooms = this.client.getRooms().map(room => ({
      roomId: room.roomId,
      name: room.name,
      topic: room.currentState.getStateEvents('m.room.topic', '')?.getContent()?.topic,
      members: room.getMembers().map(member => {
        // Get MXC avatar URL and ensure it's stable
        const mxcAvatarUrl = member.getMxcAvatarUrl();
        
        return {
          userId: member.userId,
          displayName: member.name,
          avatarUrl: mxcAvatarUrl || null, // Ensure null instead of undefined
          presence: this.client?.getUser(member.userId)?.presence,
        };
      }),
      isDirectMessage: room.getMembers().length === 2,
      lastActivity: new Date(room.getLastActiveTimestamp()),
    }));

    console.log('getRooms - cached new room data:', this.cachedRooms.length);
    return this.cachedRooms;
  }

  /**
   * Get friends (users in direct message rooms)
   */
  getFriends(): MatrixUser[] {
    // Return cached friends if available
    if (this.cachedFriends) {
      console.log('getFriends - returning cached friends:', this.cachedFriends.length);
      return this.cachedFriends;
    }

    console.log('getFriends - fetching fresh friend data');
    
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

    this.cachedFriends = Array.from(friends.values());
    
    console.log('getFriends - cached new friend data:', this.cachedFriends.length);
    return this.cachedFriends;
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
   * Add a friend by creating a direct message room
   */
  async addFriend(userId: string): Promise<string> {
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
   * Create a direct message room with a user
   */
  async createDirectMessage(userId: string): Promise<string> {
    return this.addFriend(userId);
  }

  /**
   * Find the room ID for a direct message with a specific user
   */
  findDirectMessageRoom(userId: string): string | null {
    const rooms = this.getRooms();
    const dmRoom = rooms.find(room => 
      room.isDirectMessage && 
      room.members.some(member => member.userId === userId)
    );
    return dmRoom?.roomId || null;
  }

  /**
   * Get or create a direct message room with a user
   */
  async getOrCreateDirectMessageRoom(userId: string): Promise<string> {
    // First try to find existing DM room
    const existingRoomId = this.findDirectMessageRoom(userId);
    if (existingRoomId) {
      return existingRoomId;
    }
    
    // Create new DM room if none exists
    return this.createDirectMessage(userId);
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
   * Get media as blob URL for authenticated access
   */
  async getAuthenticatedMediaBlob(mxcUrl: string): Promise<string | null> {
    if (!this.client || !mxcUrl || !mxcUrl.startsWith('mxc://')) {
      return null;
    }

    try {
      console.log('getAuthenticatedMediaBlob - fetching:', mxcUrl);
      
      // Use the Matrix client's authenticated HTTP client to fetch the media
      const httpUrl = this.client.mxcUrlToHttp(mxcUrl, 64, 64, 'crop', true);
      if (!httpUrl) {
        console.error('Failed to convert MXC URL to HTTP URL');
        return null;
      }

      console.log('getAuthenticatedMediaBlob - HTTP URL:', httpUrl);

      // Get the access token and make an authenticated request
      const accessToken = this.client.getAccessToken();
      if (!accessToken) {
        console.error('No access token available');
        return null;
      }

      // Fetch with authentication header
      const response = await fetch(httpUrl, {
        headers: {
          'Authorization': `Bearer ${accessToken}`,
        },
      });

      if (!response.ok) {
        console.error('Failed to fetch media:', response.status, response.statusText);
        return null;
      }

      // Convert to blob and create object URL
      const blob = await response.blob();
      const blobUrl = URL.createObjectURL(blob);
      
      console.log('getAuthenticatedMediaBlob - created blob URL:', blobUrl);
      return blobUrl;
    } catch (error) {
      console.error('Failed to get authenticated media blob:', error);
      return null;
    }
  }

  /**
   * Get current user info
   */
  getCurrentUser(): MatrixUser | null {
    if (!this.client || !this.config.userId) return null;

    // Return cached user if available and stable
    if (this.cachedCurrentUser) {
      console.log('getCurrentUser - returning cached user:', this.cachedCurrentUser);
      return this.cachedCurrentUser;
    }

    const user = this.client.getUser(this.config.userId);
    
    // Debug logging
    console.log('getCurrentUser - raw user object:', user);
    console.log('getCurrentUser - raw avatarUrl:', user?.avatarUrl);
    
    // Only cache if we have valid data
    if (user && user.avatarUrl !== undefined) {
      this.cachedCurrentUser = {
        userId: this.config.userId,
        displayName: user.displayName,
        avatarUrl: user.avatarUrl, // Keep MXC URL as-is
        presence: user.presence,
      };
      
      console.log('getCurrentUser - cached new user data:', this.cachedCurrentUser);
      return this.cachedCurrentUser;
    }
    
    // Fallback for when user data is not yet available
    return {
      userId: this.config.userId,
      displayName: user?.displayName,
      avatarUrl: null, // Use null instead of undefined to prevent flickering
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
      console.log('setAvatar - uploading file:', file.name, file.type);
      
      // Upload the file to Matrix media repository
      const uploadResponse = await this.client.uploadContent(file, {
        name: file.name,
        type: file.type,
      });

      const avatarUrl = uploadResponse.content_uri;
      console.log('setAvatar - upload response MXC URL:', avatarUrl);

      // Set the avatar URL in the user's profile
      await this.client.setAvatarUrl(avatarUrl);
      console.log('setAvatar - avatar URL set on profile');

      // Clear cache to force refresh
      this.cachedCurrentUser = null;

      // Wait a moment for the change to propagate
      await new Promise(resolve => setTimeout(resolve, 1000));

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
      
      // Clear cache to force refresh
      this.cachedCurrentUser = null;
      
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
      
      // Clear cache to force refresh
      this.cachedCurrentUser = null;
      
      this.emit('displayNameUpdated', displayName);
    } catch (error) {
      console.error('Failed to set display name:', error);
      throw new Error('Failed to update display name');
    }
  }

  // ===== GOOSE-TO-GOOSE COMMUNICATION METHODS =====

  /**
   * Send a Goose chat message to another Goose instance
   */
  async sendGooseMessage(
    roomId: string, 
    content: string, 
    type: GooseChatMessage['type'] = 'goose.chat',
    options?: {
      replyTo?: string;
      taskId?: string;
      taskType?: string;
      priority?: 'low' | 'medium' | 'high' | 'urgent';
      capabilities?: string[];
      status?: 'pending' | 'in_progress' | 'completed' | 'failed';
      attachments?: Array<{
        type: 'file' | 'image' | 'code' | 'log';
        name: string;
        url?: string;
        content?: string;
      }>;
      metadata?: Record<string, any>;
    }
  ): Promise<string> {
    if (!this.client) {
      throw new Error('Client not initialized');
    }

    const messageId = this.generateMessageId();
    const timestamp = Date.now();

    // Create the Matrix event content
    const eventContent: any = {
      msgtype: 'm.text',
      body: content,
      format: 'org.matrix.custom.html',
      formatted_body: `<strong>🦆 Goose:</strong> ${content}`,
      
      // Goose message metadata
      'goose.message.type': type,
      'goose.message.id': messageId,
      'goose.timestamp': timestamp,
      'goose.version': '1.0',
    };

    // Add optional fields
    if (options?.replyTo) eventContent['goose.reply_to'] = options.replyTo;
    if (options?.taskId) eventContent['goose.task.id'] = options.taskId;
    if (options?.taskType) eventContent['goose.task.type'] = options.taskType;
    if (options?.priority) eventContent['goose.priority'] = options.priority;
    if (options?.capabilities) eventContent['goose.capabilities'] = options.capabilities;
    if (options?.status) eventContent['goose.status'] = options.status;
    if (options?.attachments) eventContent['goose.attachments'] = options.attachments;
    if (options?.metadata) eventContent['goose.metadata'] = options.metadata;

    await this.client.sendEvent(roomId, 'm.room.message', eventContent);
    
    console.log('🦆 Sent Goose message:', type, 'to room:', roomId);
    return messageId;
  }

  /**
   * Send a task request to another Goose instance
   */
  async sendTaskRequest(
    roomId: string,
    taskDescription: string,
    taskType: string,
    options?: {
      priority?: 'low' | 'medium' | 'high' | 'urgent';
      deadline?: Date;
      requiredCapabilities?: string[];
      attachments?: Array<{
        type: 'file' | 'image' | 'code' | 'log';
        name: string;
        url?: string;
        content?: string;
      }>;
      metadata?: Record<string, any>;
    }
  ): Promise<string> {
    const taskId = `task_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
    
    return this.sendGooseMessage(roomId, taskDescription, 'goose.task.request', {
      taskId,
      taskType,
      priority: options?.priority || 'medium',
      capabilities: options?.requiredCapabilities,
      status: 'pending',
      attachments: options?.attachments,
      metadata: {
        deadline: options?.deadline?.toISOString(),
        ...options?.metadata,
      },
    });
  }

  /**
   * Send a task response to another Goose instance
   */
  async sendTaskResponse(
    roomId: string,
    taskId: string,
    response: string,
    status: 'completed' | 'failed',
    options?: {
      attachments?: Array<{
        type: 'file' | 'image' | 'code' | 'log';
        name: string;
        url?: string;
        content?: string;
      }>;
      metadata?: Record<string, any>;
    }
  ): Promise<string> {
    return this.sendGooseMessage(roomId, response, 'goose.task.response', {
      taskId,
      status,
      attachments: options?.attachments,
      metadata: options?.metadata,
    });
  }

  /**
   * Send a collaboration invite to another Goose instance
   */
  async sendCollaborationInvite(
    roomId: string,
    projectDescription: string,
    requiredCapabilities?: string[],
    metadata?: Record<string, any>
  ): Promise<string> {
    return this.sendGooseMessage(roomId, projectDescription, 'goose.collaboration.invite', {
      capabilities: requiredCapabilities,
      metadata,
    });
  }

  /**
   * Accept a collaboration invite
   */
  async acceptCollaborationInvite(
    roomId: string,
    originalMessageId: string,
    capabilities?: string[],
    metadata?: Record<string, any>
  ): Promise<string> {
    return this.sendGooseMessage(roomId, 'Collaboration invite accepted! 🤝', 'goose.collaboration.accept', {
      replyTo: originalMessageId,
      capabilities,
      metadata,
    });
  }

  /**
   * Decline a collaboration invite
   */
  async declineCollaborationInvite(
    roomId: string,
    originalMessageId: string,
    reason?: string,
    metadata?: Record<string, any>
  ): Promise<string> {
    const message = reason ? `Collaboration invite declined: ${reason}` : 'Collaboration invite declined.';
    return this.sendGooseMessage(roomId, message, 'goose.collaboration.decline', {
      replyTo: originalMessageId,
      metadata,
    });
  }

  /**
   * Get Goose instances from friends list
   */
  getGooseInstances(): GooseInstance[] {
    return this.getFriends()
      .filter(friend => this.isGooseInstance(friend.userId, friend.displayName))
      .map(friend => ({
        userId: friend.userId,
        displayName: friend.displayName,
        avatarUrl: friend.avatarUrl,
        presence: friend.presence,
        capabilities: [], // TODO: Extract from user profile or recent messages
        lastSeen: new Date(), // TODO: Get from presence data
        status: 'idle', // TODO: Determine from recent activity
      }));
  }

  /**
   * Create a Goose collaboration room
   */
  async createGooseCollaborationRoom(
    name: string, 
    inviteGooseIds: string[] = [],
    topic?: string
  ): Promise<string> {
    if (!this.client) {
      throw new Error('Client not initialized');
    }

    const room = await this.client.createRoom({
      name: `🦆 ${name}`,
      topic: topic || 'Goose-to-Goose Collaboration Room',
      preset: 'private_chat',
      invite: inviteGooseIds,
    });

    // Send a welcome message to the room
    await this.sendGooseMessage(room.room_id, `Welcome to the collaboration room: ${name}! 🦆`, 'goose.chat', {
      metadata: {
        roomType: 'collaboration',
        createdBy: this.config.userId,
      },
    });

    return room.room_id;
  }

  /**
   * Announce capabilities to a room
   */
  async announceCapabilities(
    roomId: string,
    capabilities: string[],
    status: 'idle' | 'busy' | 'working' = 'idle',
    currentTask?: string
  ): Promise<string> {
    const message = `🦆 Available capabilities: ${capabilities.join(', ')}`;
    return this.sendGooseMessage(roomId, message, 'goose.chat', {
      capabilities,
      metadata: {
        announcement: 'capabilities',
        status,
        currentTask,
      },
    });
  }

  /**
   * Debug method to test Goose message detection and sending
   */
  async debugGooseMessage(roomId: string): Promise<void> {
    console.log('🔍 DEBUG: Testing Goose message detection and sending');
    console.log('🔍 DEBUG: Current user ID:', this.config.userId);
    console.log('🔍 DEBUG: Target room ID:', roomId);
    
    // Send a test message with explicit Goose markers
    const testMessage = '🦆 DEBUG: This is a test Goose message from ' + (this.getCurrentUser()?.displayName || 'Unknown User');
    
    try {
      const messageId = await this.sendGooseMessage(roomId, testMessage, 'goose.chat', {
        metadata: {
          debug: true,
          timestamp: new Date().toISOString(),
          sender: this.config.userId,
        },
      });
      
      console.log('🔍 DEBUG: Successfully sent Goose message with ID:', messageId);
    } catch (error) {
      console.error('🔍 DEBUG: Failed to send Goose message:', error);
    }
  }

  /**
   * Get room message history
   */
  async getRoomHistory(roomId: string, limit: number = 50): Promise<Array<{
    messageId: string;
    sender: string;
    content: string;
    timestamp: Date;
    type: 'user' | 'assistant' | 'system';
    isFromSelf: boolean;
    senderInfo: {
      userId: string;
      displayName?: string;
      avatarUrl?: string;
    };
    metadata?: Record<string, any>;
  }>> {
    if (!this.client) {
      throw new Error('Client not initialized');
    }

    try {
      console.log('🔍 Fetching room history for:', roomId, 'limit:', limit);
      
      const room = this.client.getRoom(roomId);
      if (!room) {
        console.error('❌ Room not found:', roomId);
        return [];
      }

      // Get timeline events from the room
      const timeline = room.getLiveTimeline();
      const events = timeline.getEvents();
      
      console.log('📜 Found', events.length, 'events in room timeline');
      
      // Filter and convert message events
      const messages = events
        .filter(event => event.getType() === 'm.room.message')
        .slice(-limit) // Get the last N messages
        .map(event => {
          const content = event.getContent();
          const sender = event.getSender();
          const isFromSelf = sender === this.config.userId;
          
          // Get sender information
          const senderMember = room.getMember(sender);
          const senderUser = this.client?.getUser(sender);
          
          const senderInfo = {
            userId: sender,
            displayName: senderMember?.name || senderUser?.displayName || sender.split(':')[0].substring(1),
            avatarUrl: senderMember?.getMxcAvatarUrl() || senderUser?.avatarUrl || null,
          };

          // Parse session messages if present
          let actualContent = content.body || '';
          let messageType: 'user' | 'assistant' | 'system' = 'user';
          let sessionData = null;

          // Check if this is a session message that needs parsing
          if (actualContent.includes('goose-session-message:')) {
            try {
              const sessionJson = actualContent.substring(actualContent.indexOf('goose-session-message:') + 'goose-session-message:'.length);
              sessionData = JSON.parse(sessionJson);
              actualContent = sessionData.content || actualContent;
              messageType = sessionData.role === 'assistant' ? 'assistant' : 'user';
              console.log('📜 Parsed session message:', sessionData.role, actualContent.substring(0, 50) + '...');
            } catch (error) {
              console.warn('Failed to parse session message:', error);
            }
          }
          // Check if this is a regular Goose/AI message
          else if (content['goose.message.type'] || content['goose.type'] || 
              this.isGooseInstance(sender, senderInfo.displayName) ||
              this.looksLikeGooseMessage(actualContent)) {
            messageType = 'assistant';
          } else if (content.msgtype === 'm.notice' || sender.includes('bot')) {
            messageType = 'system';
          }

          return {
            messageId: event.getId() || `msg_${event.getTs()}`,
            sender,
            content: actualContent,
            timestamp: new Date(event.getTs()),
            type: messageType,
            isFromSelf,
            senderInfo,
            metadata: {
              eventType: event.getType(),
              msgType: content.msgtype,
              gooseType: content['goose.message.type'] || content['goose.type'],
              gooseSessionId: content['goose.session_id'] || sessionData?.sessionId,
              gooseTaskId: content['goose.task.id'],
              sessionData,
              originalEvent: event,
              originalContent: content.body,
            },
          };
        });

      console.log('📜 Processed', messages.length, 'messages from room history');
      return messages;
      
    } catch (error) {
      console.error('❌ Failed to fetch room history:', error);
      return [];
    }
  }

  /**
   * Convert Matrix room history to Goose chat format
   */
  async getRoomHistoryAsGooseMessages(roomId: string, limit: number = 50): Promise<Array<{
    role: 'user' | 'assistant' | 'system';
    content: string;
    timestamp: Date;
    sender?: string;
    metadata?: Record<string, any>;
  }>> {
    const history = await this.getRoomHistory(roomId, limit);
    
    return history.map(msg => ({
      role: msg.type,
      content: msg.content,
      timestamp: msg.timestamp,
      sender: msg.senderInfo.displayName || msg.sender,
      metadata: {
        ...msg.metadata,
        originalSender: msg.sender,
        senderInfo: msg.senderInfo,
        isFromSelf: msg.isFromSelf,
      },
    }));
  }

  /**
   * Get debug information about the current Matrix state
   */
  getDebugInfo(): Record<string, any> {
    return {
      isConnected: this.isConnected,
      syncState: this.syncState,
      currentUserId: this.config.userId,
      currentUser: this.getCurrentUser(),
      friendsCount: this.getFriends().length,
      roomsCount: this.getRooms().length,
      gooseInstancesCount: this.getGooseInstances().length,
      homeserver: this.config.homeserverUrl,
    };
  }
}

// Export singleton instance
export const matrixService = new MatrixService({
  homeserverUrl: 'https://matrix.tchncs.de', // Tchncs.de homeserver with open registration
});
