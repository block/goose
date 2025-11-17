import * as sdk from 'matrix-js-sdk';
import { EventEmitter } from 'events';
import { sessionMappingService, SessionMappingService } from './SessionMappingService';
import { matrixInviteStateService } from './MatrixInviteStateService';

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
  private isInitialSync = true; // Track if we're in initial sync to prevent startup notifications

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

    // Optimized sync settings for better real-time performance
    await this.client.startClient({ 
      initialSyncLimit: 50, // Increased from 10 to get more recent messages
      pollTimeout: 30000,   // 30 second long polling for faster notifications
      // Remove filter for now - it was causing sync errors
      // We'll rely on the increased sync limits for better performance
    });
    
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
        
        // Clean up invite states for rooms we're already in
        this.cleanupJoinedRoomInvites();
        
        // Auto-rejoin stored Matrix rooms
        this.autoRejoinStoredRooms().catch(error => {
          console.error('❌ Error during auto-rejoin:', error);
        });
        
        // Mark initial sync as complete - now we can process new invitations
        setTimeout(() => {
          this.isInitialSync = false;
          console.log('🔄 Initial sync complete - now processing new invitations');
        }, 2000); // Give 2 seconds for all initial events to settle
        
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
      
      // Update Matrix room state tracking
      this.updateMatrixRoomStateFromMembership(member.roomId, member, event);
      
      // Handle Matrix room invitations for collaboration
      if (member.userId === this.config.userId && member.membership === 'invite') {
        console.log('🎯 Received Matrix room invitation:', {
          roomId: member.roomId,
          inviter: event.getSender(),
          membership: member.membership,
          isInitialSync: this.isInitialSync
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
      inviter,
      isInitialSync: this.isInitialSync
    });

    // CRITICAL FIX: Skip processing invitations during initial sync to prevent startup notifications
    if (this.isInitialSync) {
      console.log('🎯 Skipping Matrix room invitation during initial sync - will process after sync complete:', {
        roomId: roomId.substring(0, 20) + '...',
        inviter,
      });
      return;
    }

    // Check if we're already in this room (joined membership)
    const room = this.client?.getRoom(roomId);
    if (room && room.getMyMembership() === 'join') {
      console.log('🎯 Skipping Matrix room invitation - already joined this room:', {
        roomId,
        membership: room.getMyMembership(),
      });
      
      // Mark as accepted in invite state service to prevent future notifications
      matrixInviteStateService.acceptInvite(roomId);
      return;
    }

    // Get inviter information
    const inviterUser = this.client?.getUser(inviter);
    const inviterName = inviterUser?.displayName || inviter.split(':')[0].substring(1);

    // Record the invite state and check if it should be shown
    const inviteState = matrixInviteStateService.recordInvite(roomId, inviter, inviterName);
    
    if (!matrixInviteStateService.shouldShowInvite(roomId, inviter)) {
      console.log('🎯 Skipping Matrix room invitation - already handled or seen recently:', {
        roomId,
        inviter,
        status: inviteState.status,
      });
      return;
    }

    // Emit a direct Matrix room invitation event (not a goose message)
    // This prevents duplicate notifications
    const invitationData = {
      roomId,
      inviter,
      inviterName,
      timestamp: new Date(),
      type: 'matrix_room_invitation',
      inviteState, // Include the state for UI reference
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
      // REMOVED: /collaboration.*invite/i, - this was catching Matrix room invitations
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

    // Create session mapping with backend session for this Matrix room
    const participants = [this.config.userId!, ...inviteUserIds];
    const mapping = await sessionMappingService.createMappingWithBackendSession(room.room_id, participants, name);
    
    console.log('📋 Created AI session with backend mapping:', {
      matrixRoomId: room.room_id,
      backendSessionId: mapping.gooseSessionId,
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

    // Ensure userId starts with '@' as required by Matrix
    const formattedUserId = userId.startsWith('@') ? userId : `@${userId}`;
    
    console.log('🔗 Inviting user to room:', {
      roomId,
      originalUserId: userId,
      formattedUserId,
      needsFormatting: !userId.startsWith('@')
    });

    await this.client.invite(roomId, formattedUserId);
  }

  /**
   * Accept a Matrix room invitation and update invite state
   */
  async acceptMatrixInvite(roomId: string): Promise<void> {
    try {
      await this.joinRoom(roomId);
      matrixInviteStateService.acceptInvite(roomId);
      console.log('✅ Accepted Matrix invite and updated state:', roomId);
    } catch (error) {
      console.error('❌ Failed to accept Matrix invite:', error);
      throw error;
    }
  }

  /**
   * Decline a Matrix room invitation and update invite state
   */
  async declineMatrixInvite(roomId: string): Promise<void> {
    try {
      // Matrix doesn't have a direct "decline" method, but we can leave the room if we're in it
      // or just update our local state to mark it as declined
      matrixInviteStateService.declineInvite(roomId);
      console.log('✅ Declined Matrix invite and updated state:', roomId);
    } catch (error) {
      console.error('❌ Failed to decline Matrix invite:', error);
      throw error;
    }
  }

  /**
   * Dismiss a Matrix room invitation (close notification without action)
   */
  async dismissMatrixInvite(roomId: string): Promise<void> {
    matrixInviteStateService.dismissInvite(roomId);
    console.log('✅ Dismissed Matrix invite:', roomId);
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
        await this.ensureSessionMapping(roomId, existingRoom);
        return;
      }

      // Join the room
      await this.client.joinRoom(roomId);
      console.log('✅ Successfully joined room:', roomId);
      
      // Mark the invite state as accepted
      matrixInviteStateService.acceptInvite(roomId);
      
      // Clear caches to refresh room data
      this.cachedRooms = null;
      this.cachedFriends = null;
      
      // Get the room after joining to create session mapping
      const joinedRoom = this.client.getRoom(roomId);
      if (joinedRoom) {
        await this.ensureSessionMapping(roomId, joinedRoom);
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
  private async ensureSessionMapping(roomId: string, room: any): Promise<void> {
    // Check if mapping already exists
    const existingMapping = sessionMappingService.getMapping(roomId);
    if (existingMapping) {
      console.log('📋 Session mapping already exists for room:', roomId, '→', existingMapping.gooseSessionId);
      
      // Update participants if needed
      const currentParticipants = room.getMembers().map((member: any) => member.userId);
      sessionMappingService.updateParticipants(roomId, currentParticipants);
      return;
    }

    // Create new mapping with backend session for ALL rooms (including DMs)
    const participants = room.getMembers().map((member: any) => member.userId);
    const memberCount = participants.length;
    const isDM = this.isDirectMessageRoom(room);
    
    // Generate appropriate room name
    let roomName: string;
    if (room.name) {
      roomName = room.name;
    } else if (isDM) {
      // For DMs, create a name based on the other participant
      const otherParticipant = participants.find(p => p !== this.config.userId);
      const otherUser = otherParticipant ? this.client?.getUser(otherParticipant) : null;
      const otherName = otherUser?.displayName || otherParticipant?.split(':')[0].substring(1) || 'Unknown';
      roomName = `DM with ${otherName}`;
    } else {
      roomName = `Matrix Room ${roomId.substring(1, 8)}`;
    }
    
    console.log('📋 Creating session mapping for Matrix room:', {
      roomId: roomId.substring(0, 20) + '...',
      roomName,
      participants: participants.length,
      isDM,
      type: isDM ? 'Direct Message' : 'Group Chat'
    });
    
    try {
      // Always create backend session mapping for persistence
      const mapping = await sessionMappingService.createMappingWithBackendSession(roomId, participants, roomName);
      
      console.log('📋 ✅ Created backend session mapping:', {
        matrixRoomId: roomId.substring(0, 20) + '...',
        backendSessionId: mapping.gooseSessionId,
        participants: participants.length,
        roomName,
        isDM,
      });
    } catch (error) {
      console.error('📋 ❌ Failed to create backend session mapping:', error);
      // Fallback to regular mapping if backend session creation fails
      const mapping = sessionMappingService.createMapping(roomId, participants, roomName);
      console.log('📋 Created fallback mapping:', {
        matrixRoomId: roomId.substring(0, 20) + '...',
        gooseSessionId: mapping.gooseSessionId,
        participants: participants.length,
        roomName,
        isDM,
      });
    }
  }

  /**
   * Check if a room is a true direct message room
   * True DMs have exactly 2 members AND no explicit room name (auto-generated from usernames)
   */
  private isDirectMessageRoom(room: any): boolean {
    const memberCount = room.getMembers().length;
    const hasExplicitName = room.name && room.name.trim() !== '';
    
    // True DM: exactly 2 members + no explicit name (Matrix auto-generates display name)
    const isTrueDM = memberCount === 2 && !hasExplicitName;
    
    console.log('🔍 DM Detection:', {
      roomId: room.roomId.substring(0, 20) + '...',
      memberCount,
      roomName: room.name || '(auto-generated)',
      hasExplicitName,
      isTrueDM
    });
    
    return isTrueDM;
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
      isDirectMessage: this.isDirectMessageRoom(room), // Use improved DM detection
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

      // Fetch with authentication header and improved error handling
      const response = await fetch(httpUrl, {
        headers: {
          'Authorization': `Bearer ${accessToken}`,
        },
        // Add timeout and other fetch options to handle HTTP2 issues
        signal: AbortSignal.timeout(10000), // 10 second timeout
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
      // Enhanced error handling for different types of network errors
      if (error instanceof Error) {
        if (error.name === 'TimeoutError') {
          console.warn('Matrix media fetch timed out for:', mxcUrl);
        } else if (error.message.includes('HTTP2') || error.message.includes('PROTOCOL_ERROR')) {
          console.warn('HTTP2 protocol error fetching Matrix media:', mxcUrl, '- this is usually a server-side issue');
        } else if (error.message.includes('Failed to fetch')) {
          console.warn('Network error fetching Matrix media:', mxcUrl, '- check network connection');
        } else {
          console.error('Failed to get authenticated media blob:', error);
        }
      } else {
        console.error('Failed to get authenticated media blob:', error);
      }
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
              
              // FIXED: Better role detection for session messages
              if (sessionData.role === 'assistant' || sessionData.role === 'ai' || sessionData.role === 'goose') {
                messageType = 'assistant';
              } else if (sessionData.role === 'system') {
                messageType = 'system';
              } else {
                messageType = 'user';
              }
              
              console.log('📜 Parsed session message:', sessionData.role, '→', messageType, actualContent.substring(0, 50) + '...');
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
          
          // ADDITIONAL FIX: Check if message is from self but contains AI response patterns
          else if (isFromSelf && this.looksLikeGooseMessage(actualContent)) {
            // This handles cases where the user's own messages contain AI responses
            messageType = 'assistant';
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
   * Update Matrix room state tracking from membership events
   */
  private updateMatrixRoomStateFromMembership(roomId: string, member: any, event: any): void {
    if (!this.client) return;

    try {
      const room = this.client.getRoom(roomId);
      if (!room) return;

      // Get room information
      const roomName = room.name;
      const roomTopic = room.currentState.getStateEvents('m.room.topic', '')?.getContent()?.topic;
      const members = room.getMembers();
      const memberCount = members.length;
      const isDirectMessage = this.isDirectMessageRoom(room);
      const isEncrypted = room.hasEncryptionStateEvent();

      // Update room metadata
      sessionMappingService.updateMatrixRoomMetadata(roomId, {
        name: roomName,
        topic: roomTopic,
        memberCount,
        isDirectMessage,
        isEncrypted,
        lastActivity: Date.now(),
      });

      // Get member information
      const memberUser = this.client.getUser(member.userId);
      const memberInfo = {
        userId: member.userId,
        displayName: member.name || memberUser?.displayName || member.userId.split(':')[0].substring(1),
        avatarUrl: member.getMxcAvatarUrl() || memberUser?.avatarUrl || null,
        joinedAt: member.membership === 'join' ? Date.now() : (member.events?.member?.getTs() || Date.now()),
        leftAt: member.membership === 'leave' ? Date.now() : undefined,
        membership: member.membership,
        lastActivity: Date.now(),
      };

      // Determine the event type
      let eventType: 'join' | 'leave' | 'invite' | 'kick' | 'ban' = 'join';
      if (member.membership === 'leave') {
        // Check if this was a kick/ban or voluntary leave
        const prevMembership = event.getPrevContent()?.membership;
        if (prevMembership === 'join' || prevMembership === 'invite') {
          eventType = event.getSender() === member.userId ? 'leave' : 'kick';
        }
      } else if (member.membership === 'invite') {
        eventType = 'invite';
      } else if (member.membership === 'ban') {
        eventType = 'ban';
      }

      // Update participant information
      sessionMappingService.updateMatrixParticipant(
        roomId,
        memberInfo,
        eventType,
        event.getSender()
      );

      console.log('📋 Updated Matrix room state from membership event:', {
        roomId: roomId.substring(0, 20) + '...',
        userId: member.userId,
        membership: member.membership,
        eventType,
        memberCount,
      });

    } catch (error) {
      console.error('❌ Failed to update Matrix room state from membership:', error);
    }
  }

  /**
   * Auto-rejoin Matrix rooms and ensure session mappings for all joined rooms
   * This should be called after Matrix sync is prepared
   */
  private async autoRejoinStoredRooms(): Promise<void> {
    if (!this.client) return;

    console.log('🔄 Auto-rejoining stored Matrix rooms and ensuring session mappings...');
    
    try {
      // Get all session mappings (including DM rooms)
      const allMappings = sessionMappingService.getAllMappings();
      const matrixMappings = allMappings.filter(mapping => mapping.matrixRoomId);
      
      console.log(`📋 Found ${matrixMappings.length} Matrix room mappings to check`);
      
      let rejoinedCount = 0;
      let skippedCount = 0;
      let failedCount = 0;
      let mappingsCreated = 0;

      // First, handle stored mappings
      for (const mapping of matrixMappings) {
        const { matrixRoomId, roomState } = mapping;
        
        try {
          // Check current membership status
          const room = this.client.getRoom(matrixRoomId);
          const currentMembership = room?.getMyMembership();

          console.log(`🔍 Checking stored room ${matrixRoomId.substring(0, 20)}... - current membership: ${currentMembership}`);

          // If we're not currently joined but have a stored room state, try to rejoin
          if (currentMembership !== 'join' && roomState) {
            // Check if we were previously joined based on stored participant data
            const myParticipant = roomState.participants.get(this.config.userId!);
            const wasJoined = myParticipant?.membership === 'join';

            if (wasJoined) {
              console.log(`🚪 Attempting to rejoin room: ${matrixRoomId.substring(0, 20)}...`);
              
              try {
                await this.client.joinRoom(matrixRoomId);
                rejoinedCount++;
                console.log(`✅ Successfully rejoined room: ${matrixRoomId.substring(0, 20)}...`);
                
                // Mark the invite state as accepted since we successfully rejoined
                matrixInviteStateService.markAutoJoined(matrixRoomId);
              } catch (joinError: any) {
                failedCount++;
                console.warn(`❌ Failed to rejoin room ${matrixRoomId.substring(0, 20)}...:`, joinError.message);
                
                // If we can't rejoin, update the stored state to reflect this
                if (myParticipant) {
                  myParticipant.membership = 'leave';
                  myParticipant.leftAt = Date.now();
                  sessionMappingService.updateMatrixParticipant(
                    matrixRoomId,
                    myParticipant,
                    'leave'
                  );
                }
              }
            } else {
              console.log(`⏭️ Skipping ${matrixRoomId.substring(0, 20)}... - was not previously joined`);
              skippedCount++;
            }
          } else if (currentMembership === 'join') {
            console.log(`✅ Already joined room: ${matrixRoomId.substring(0, 20)}...`);
            skippedCount++;
          }
        } catch (error) {
          console.error(`❌ Error processing stored room ${matrixRoomId.substring(0, 20)}...:`, error);
          failedCount++;
        }
      }

      // Second, ensure session mappings exist for ALL currently joined rooms (including DMs)
      console.log('📋 Ensuring session mappings for all currently joined rooms...');
      const currentRooms = this.client.getRooms();
      
      for (const room of currentRooms) {
        if (room.getMyMembership() === 'join') {
          const existingMapping = sessionMappingService.getMapping(room.roomId);
          
          if (!existingMapping) {
            console.log(`📋 Creating missing session mapping for joined room: ${room.roomId.substring(0, 20)}...`);
            try {
              await this.ensureSessionMapping(room.roomId, room);
              mappingsCreated++;
            } catch (error) {
              console.error(`❌ Failed to create session mapping for ${room.roomId.substring(0, 20)}...:`, error);
            }
          }
        }
      }

      console.log(`🎯 Auto-rejoin and mapping complete: ${rejoinedCount} rejoined, ${skippedCount} skipped, ${failedCount} failed, ${mappingsCreated} new mappings created`);
      
      if (rejoinedCount > 0 || mappingsCreated > 0) {
        // Clear caches to refresh room data
        this.cachedRooms = null;
        this.cachedFriends = null;
        
        // Emit an event to notify UI components
        this.emit('roomsRejoined', { 
          rejoined: rejoinedCount, 
          mappingsCreated 
        });
      }
    } catch (error) {
      console.error('❌ Error during auto-rejoin and mapping process:', error);
    }
  }

  /**
   * Clean up invite states for rooms we're already joined to
   * This is critical for preventing duplicate invite notifications
   */
  public cleanupJoinedRoomInvites(): void {
    if (!this.client) return;

    console.log('🧹 Cleaning up invite states for joined rooms...');
    
    const allInviteStates = matrixInviteStateService.getAllInviteStates();
    const pendingInvites = allInviteStates.filter(state => state.status === 'pending');
    
    console.log(`🧹 Found ${allInviteStates.length} total invite states, ${pendingInvites.length} pending`);
    
    let cleanedCount = 0;
    let alreadyJoinedCount = 0;

    // Check all pending invites to see if we're already in those rooms
    pendingInvites.forEach(inviteState => {
      const room = this.client?.getRoom(inviteState.roomId);
      const currentMembership = room?.getMyMembership();
      
      console.log(`🧹 Checking invite state for room ${inviteState.roomId.substring(0, 20)}... - membership: ${currentMembership}`);
      
      if (room && currentMembership === 'join') {
        console.log(`🧹 Marking joined room as accepted: ${inviteState.roomId.substring(0, 20)}...`);
        matrixInviteStateService.acceptInvite(inviteState.roomId);
        cleanedCount++;
      } else if (room && (currentMembership === 'leave' || currentMembership === 'ban')) {
        // If we've left or been banned, mark as declined to prevent showing
        console.log(`🧹 Marking left/banned room as declined: ${inviteState.roomId.substring(0, 20)}... (${currentMembership})`);
        matrixInviteStateService.declineInvite(inviteState.roomId);
        cleanedCount++;
      } else if (!room) {
        // Room doesn't exist in our client, might be old or we never joined
        console.log(`🧹 Room not found in client: ${inviteState.roomId.substring(0, 20)}... - keeping as pending`);
      }
    });

    // Also check all rooms we're currently in to ensure their invite states are marked as accepted
    const currentRooms = this.client.getRooms();
    currentRooms.forEach(room => {
      if (room.getMyMembership() === 'join') {
        const inviteState = matrixInviteStateService.getInviteState(room.roomId);
        if (inviteState && inviteState.status === 'pending') {
          console.log(`🧹 Found joined room with pending invite state, marking as accepted: ${room.roomId.substring(0, 20)}...`);
          matrixInviteStateService.acceptInvite(room.roomId);
          alreadyJoinedCount++;
        }
      }
    });

    const totalCleaned = cleanedCount + alreadyJoinedCount;
    if (totalCleaned > 0) {
      console.log(`🧹 Cleaned up ${totalCleaned} invite states (${cleanedCount} from pending list, ${alreadyJoinedCount} from current rooms)`);
      
      // Log final statistics
      const stats = matrixInviteStateService.getInviteStats();
      console.log('🧹 Final invite state statistics:', stats);
    } else {
      console.log('🧹 No invite states needed cleanup');
    }
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

// Temporary: Expose for debugging (remove in production)
if (typeof window !== 'undefined') {
  (window as any).matrixService = matrixService;
  (window as any).sessionMappingService = sessionMappingService;
  (window as any).debugInviteStates = () => {
    console.log('=== DEBUGGING INVITE STATES ===');
    const allStates = matrixInviteStateService.getAllInviteStates();
    const stats = matrixInviteStateService.getInviteStats();
    console.log('All invite states:', allStates);
    console.log('Stats:', stats);
    
    console.log('\n=== CURRENT ROOM MEMBERSHIPS ===');
    const rooms = matrixService.client?.getRooms() || [];
    rooms.forEach(room => {
      const membership = room.getMyMembership();
      console.log(`Room ${room.roomId}: ${membership} (${room.name || 'Unnamed'})`);
    });
    
    console.log('\n=== INVITE STATE ANALYSIS ===');
    allStates.forEach(state => {
      const room = matrixService.client?.getRoom(state.roomId);
      const currentMembership = room?.getMyMembership() || 'unknown';
      const shouldShow = matrixInviteStateService.shouldShowInvite(state.roomId);
      console.log(`${state.roomId}: status=${state.status}, membership=${currentMembership}, shouldShow=${shouldShow}`);
    });
    
    console.log('\n=== FORCE CLEANUP ALL INVITES ===');
    console.log('This will mark all invites for joined rooms as accepted...');
    matrixService.cleanupJoinedRoomInvites();
  };
  
  (window as any).clearAllInviteStates = () => {
    console.log('🗑️ CLEARING ALL INVITE STATES');
    matrixInviteStateService.clearAllInviteStates();
    console.log('✅ All invite states cleared');
  };
  
  (window as any).findChatSession20251114 = () => {
    console.log('🔍 SEARCHING FOR "Chat Session 20251114"...');
    
    // Search in invite states
    const allStates = matrixInviteStateService.getAllInviteStates();
    const matchingStates = allStates.filter(state => 
      state.inviterName?.includes('Chat Session 20251114') ||
      state.roomId?.includes('20251114')
    );
    
    console.log('📋 Matching invite states:', matchingStates);
    
    // Search in Matrix rooms
    const rooms = matrixService.client?.getRooms() || [];
    const matchingRooms = rooms.filter(room => 
      room.name?.includes('Chat Session 20251114') ||
      room.name?.includes('20251114') ||
      room.roomId?.includes('20251114')
    );
    
    console.log('🏠 Matching Matrix rooms:', matchingRooms.map(room => ({
      roomId: room.roomId,
      name: room.name,
      membership: room.getMyMembership(),
      members: room.getMembers().length,
      lastActivity: new Date(room.getLastActiveTimestamp())
    })));
    
    // Search in session mappings
    try {
      const collaborativeSessions = sessionMappingService.getMatrixCollaborativeSessions();
      const matchingSessions = collaborativeSessions.filter(session =>
        session.title?.includes('Chat Session 20251114') ||
        session.title?.includes('20251114') ||
        session.matrixRoomId?.includes('20251114')
      );
      
      console.log('📝 Matching session mappings:', matchingSessions);
    } catch (error) {
      console.log('📝 Could not search session mappings:', error);
    }
    
    // Check each matching state in detail
    matchingStates.forEach(state => {
      const shouldShow = matrixInviteStateService.shouldShowInvite(state.roomId, state.inviter);
      const room = matrixService.client?.getRoom(state.roomId);
      const membership = room?.getMyMembership();
      
      console.log(`🔍 DETAILED ANALYSIS for ${state.roomId}:`, {
        inviteState: state,
        shouldShow,
        currentMembership: membership,
        roomExists: !!room,
        roomName: room?.name,
        lastSeen: state.lastSeen,
        timeSinceLastSeen: state.lastSeen ? Date.now() - new Date(state.lastSeen).getTime() : 'never',
      });
    });
    
    return {
      inviteStates: matchingStates,
      rooms: matchingRooms,
      totalFound: matchingStates.length + matchingRooms.length
    };
  };
  
  (window as any).fixChatSession20251114 = async () => {
    console.log('🔧 FIXING Chat Session 20251114 persistence issue...');
    
    const rooms = matrixService.client?.getRooms() || [];
    const chatSessionRooms = rooms.filter(room => 
      room.name?.includes('Chat Session 20251114') ||
      room.name?.includes('20251114')
    );
    
    console.log(`Found ${chatSessionRooms.length} Chat Session 20251114 rooms`);
    
    let joinedCount = 0;
    let declinedCount = 0;
    let mappedCount = 0;
    
    for (const room of chatSessionRooms) {
      const roomId = room.roomId;
      const membership = room.getMyMembership();
      const roomName = room.name || 'Unnamed Room';
      
      console.log(`Processing ${roomId.substring(0, 20)}... (${membership})`);
      
      if (membership === 'invite') {
        // Decline all pending invites except maybe keep one
        console.log(`❌ Declining invite: ${roomId.substring(0, 20)}...`);
        try {
          matrixInviteStateService.declineInvite(roomId);
          declinedCount++;
        } catch (error) {
          console.error(`Failed to decline ${roomId}:`, error);
        }
      } else if (membership === 'join') {
        // Ensure session mapping exists for joined rooms
        const existingMapping = sessionMappingService.getMapping(roomId);
        if (!existingMapping) {
          console.log(`📋 Creating session mapping for: ${roomId.substring(0, 20)}...`);
          try {
            const participants = room.getMembers().map(member => member.userId);
            sessionMappingService.createMapping(roomId, participants, roomName);
            mappedCount++;
          } catch (error) {
            console.error(`Failed to create mapping for ${roomId}:`, error);
          }
        } else {
          console.log(`📋 Session mapping already exists for: ${roomId.substring(0, 20)}...`);
        }
        
        // Clean up any stale invite states
        const inviteState = matrixInviteStateService.getInviteState(roomId);
        if (inviteState && inviteState.status === 'pending') {
          console.log(`🧹 Cleaning stale invite state for: ${roomId.substring(0, 20)}...`);
          matrixInviteStateService.acceptInvite(roomId);
        }
        
        joinedCount++;
      }
    }
    
    console.log(`✅ COMPLETED: ${joinedCount} joined rooms processed, ${declinedCount} invites declined, ${mappedCount} new mappings created`);
    
    // Force cleanup
    matrixService.cleanupJoinedRoomInvites();
    
    return { joined: joinedCount, declined: declinedCount, mapped: mappedCount };
  };
  
  (window as any).debugLiveNotifications = () => {
    console.log('🔍 DEBUGGING LIVE NOTIFICATIONS...');
    
    let notificationCount = 0;
    
    // Monitor matrixRoomInvitation events
    const originalEmit = matrixService.emit;
    matrixService.emit = function(event, ...args) {
      if (event === 'matrixRoomInvitation') {
        notificationCount++;
        console.log(`🚨 LIVE NOTIFICATION #${notificationCount}:`, {
          event,
          data: args[0],
          shouldShow: matrixInviteStateService.shouldShowInvite(args[0]?.roomId, args[0]?.inviter),
          inviteState: matrixInviteStateService.getInviteState(args[0]?.roomId),
          roomMembership: matrixService.client?.getRoom(args[0]?.roomId)?.getMyMembership()
        });
      }
      return originalEmit.apply(this, [event, ...args]);
    };
    
    // Also monitor RoomMember.membership events that trigger invitations
    const client = matrixService.client;
    if (client) {
      client.on('RoomMember.membership', (event, member) => {
        if (member.userId === matrixService.getCurrentUser()?.userId && member.membership === 'invite') {
          console.log(`👤 MEMBERSHIP EVENT - INVITE:`, {
            roomId: member.roomId.substring(0, 20) + '...',
            roomName: client.getRoom(member.roomId)?.name,
            inviter: event.getSender(),
            membership: member.membership,
            shouldShow: matrixInviteStateService.shouldShowInvite(member.roomId, event.getSender())
          });
        }
      });
    }
    
    console.log('✅ Monitoring live notifications. Watch console for activity.');
    
    // Return function to stop monitoring
    return () => {
      matrixService.emit = originalEmit;
      console.log('🛑 Stopped monitoring live notifications');
    };
  };
  
  (window as any).clearUINotifications = () => {
    console.log('🧹 CLEARING UI NOTIFICATION STATE...');
    
    // Force clear all invite states
    matrixInviteStateService.clearAllInviteStates();
    
    // Emit a custom event to tell UI components to clear their state
    window.dispatchEvent(new CustomEvent('clearNotifications'));
    
    // Also force a cleanup
    matrixService.cleanupJoinedRoomInvites();
    
    console.log('✅ UI notification clear event dispatched');
  };
}
