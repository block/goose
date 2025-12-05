import * as sdk from 'matrix-js-sdk';
import { EventEmitter } from 'events';
import { sessionMappingService, SessionMappingService } from './SessionMappingService';
import { matrixInviteStateService } from './MatrixInviteStateService';

// Global Olm initialization flag
let olmInitialized = false;

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
  isSpace: boolean;
  roomType?: string;
  lastActivity?: Date;
  avatarUrl?: string;
  isPublic?: boolean;
}

export interface SpaceChild {
  roomId: string;
  name?: string;
  topic?: string;
  avatarUrl?: string;
  isSpace: boolean;
  isPublic?: boolean;
  suggested?: boolean;
  via?: string[];
  order?: string;
  memberCount?: number;
  membership?: 'join' | 'invite' | 'leave' | 'ban' | null; // User's membership status in this room
  canJoin?: boolean; // Whether the user can join this room
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
   * Handle key upload issues and conflicts
   */
  private async handleKeyUploadIssues(): Promise<void> {
    if (!this.client?.crypto) {
      console.log('🔐 Cannot handle key upload issues: crypto not available');
      return;
    }

    try {
      console.log('🔑 Checking for key upload issues...');
      
      // Test if we can upload keys without errors
      if (typeof this.client.crypto.uploadKeys === 'function') {
        try {
          await this.client.crypto.uploadKeys();
          console.log('🔑 ✅ Key upload test successful');
          return; // No issues, exit early
        } catch (uploadError: any) {
          console.warn('🔑 ⚠️ Key upload failed, attempting to resolve:', uploadError.message);
          
          // Check if it's a "key already exists" error
          if (uploadError.message && uploadError.message.includes('already exists')) {
            console.log('🔑 Detected key conflict, attempting resolution...');
            
            // Try to mark existing keys as published
            if (this.client.crypto.olmDevice && typeof this.client.crypto.olmDevice.markKeysAsPublished === 'function') {
              try {
                this.client.crypto.olmDevice.markKeysAsPublished();
                console.log('🔑 ✅ Marked existing keys as published');
                
                // Generate new keys
                if (typeof this.client.crypto.olmDevice.generateOneTimeKeys === 'function') {
                  this.client.crypto.olmDevice.generateOneTimeKeys(10);
                  console.log('🔑 ✅ Generated new one-time keys');
                }
                
                // Try uploading again
                await this.client.crypto.uploadKeys();
                console.log('🔑 ✅ Key upload successful after conflict resolution');
                
              } catch (resolutionError) {
                console.warn('🔑 ⚠️ Key conflict resolution failed:', resolutionError.message);
              }
            }
          } else {
            // Other types of upload errors
            console.warn('🔑 ⚠️ Non-conflict key upload error:', uploadError.message);
          }
        }
      }
      
    } catch (error) {
      console.error('🔑 ❌ Failed to handle key upload issues:', error);
      // Don't throw - we want to continue even if key handling fails
    }
  }

  /**
   * Verify our own device to prevent UnknownDeviceError and handle device verification popups
   */
  private async verifyOwnDevice(): Promise<void> {
    if (!this.client?.crypto || !this.config.userId || !this.config.deviceId) {
      console.log('🔐 Cannot verify own device: missing crypto, userId, or deviceId');
      return;
    }

    try {
      console.log('🔐 Verifying own device:', this.config.deviceId);
      
      // Get our own device
      const devices = await this.client.crypto.getStoredDevicesForUser(this.config.userId);
      const ownDevice = devices.find(device => device.deviceId === this.config.deviceId);
      
      if (ownDevice) {
        if (!ownDevice.isVerified()) {
          console.log('🔐 Marking own device as verified...');
          await this.client.crypto.setDeviceVerification(this.config.userId, this.config.deviceId, true);
          console.log('🔐 ✅ Own device verified successfully');
        } else {
          console.log('🔐 ✅ Own device already verified');
        }
        
        // Also ensure the device is not blocked
        if (ownDevice.isBlocked()) {
          console.log('🔐 Unblocking own device...');
          await this.client.crypto.setDeviceBlocked(this.config.userId, this.config.deviceId, false);
          console.log('🔐 ✅ Own device unblocked');
        }
      } else {
        console.log('🔐 ⚠️ Own device not found in stored devices');
        
        // Try to get device info directly and mark as verified
        try {
          await this.client.crypto.setDeviceVerification(this.config.userId, this.config.deviceId, true);
          console.log('🔐 ✅ Own device marked as verified directly');
        } catch (directError) {
          console.warn('🔐 ⚠️ Failed to verify own device directly:', directError);
        }
      }

      // Enhanced verification to prevent "New login. Was this you?" popups
      try {
        // Check current device trust status
        if (typeof this.client.crypto.checkDeviceTrust === 'function') {
          const deviceTrust = this.client.crypto.checkDeviceTrust(this.config.userId, this.config.deviceId);
          console.log('🔐 Device trust status:', {
            isVerified: deviceTrust.isVerified(),
            isCrossSigningVerified: deviceTrust.isCrossSigningVerified(),
            isTofu: deviceTrust.isTofu(),
            isLocallyVerified: deviceTrust.isLocallyVerified ? deviceTrust.isLocallyVerified() : 'unknown'
          });
          
          // If not cross-signing verified, try to mark it as such
          if (!deviceTrust.isCrossSigningVerified()) {
            console.log('🔐 Attempting to mark device as cross-signing verified...');
            try {
              // Try different methods to mark as cross-signing verified
              if (typeof this.client.crypto.setDeviceCrossSigningVerified === 'function') {
                await this.client.crypto.setDeviceCrossSigningVerified(this.config.userId, this.config.deviceId, true);
                console.log('🔐 ✅ Device marked as cross-signing verified');
              }
            } catch (crossSignError) {
              console.warn('🔐 ⚠️ Could not set cross-signing verification:', crossSignError);
            }
          }
        }

        // Handle any pending verification requests for our own device
        if (typeof this.client.crypto.getVerificationRequestsToDeviceInProgress === 'function') {
          const verificationRequests = this.client.crypto.getVerificationRequestsToDeviceInProgress(this.config.userId);
          console.log(`🔐 Found ${verificationRequests.length} pending verification requests`);
          
          for (const request of verificationRequests) {
            // If this is a self-verification request, auto-accept it
            if (request.otherUserId === this.config.userId && request.otherDeviceId === this.config.deviceId) {
              console.log('🤝 Auto-accepting self-verification request:', request.requestId);
              try {
                if (typeof request.accept === 'function') {
                  await request.accept();
                  console.log('🔐 ✅ Self-verification request accepted');
                }
              } catch (acceptError) {
                console.warn('🔐 ⚠️ Failed to accept verification request:', acceptError);
              }
            }
          }
        }

        // Try to mark device as known/trusted in device list
        try {
          // Force the device to be marked as known and trusted
          await this.client.crypto.setDeviceVerification(this.config.userId, this.config.deviceId, true);
          
          // Also try to mark it as not requiring verification
          if (typeof this.client.crypto.setDeviceBlocked === 'function') {
            await this.client.crypto.setDeviceBlocked(this.config.userId, this.config.deviceId, false);
          }
          
          console.log('🔐 ✅ Device marked as known and trusted');
        } catch (trustError) {
          console.warn('🔐 ⚠️ Failed to mark device as trusted:', trustError);
        }

        // Additional step: Try to dismiss any active verification toasts/popups programmatically
        try {
          // Check if we can access any verification UI elements and dismiss them
          if (typeof this.client.crypto.cancelVerificationRequest === 'function') {
            // This might help dismiss pending verification UI
            console.log('🔐 Checking for verification UI to dismiss...');
          }
        } catch (dismissError) {
          console.warn('🔐 ⚠️ Could not dismiss verification UI:', dismissError);
        }

      } catch (advancedError) {
        console.warn('🔐 ⚠️ Advanced device verification failed (non-critical):', advancedError);
      }
      
    } catch (error) {
      console.error('🔐 ❌ Failed to verify own device:', error);
      // Don't throw - we want to continue even if device verification fails
    }
  }

  /**
   * Fix encryption key sharing issues for better decryption
   */
  private async fixEncryptionKeySharing(): Promise<void> {
    if (!this.client?.crypto) {
      console.log('🔐 Cannot fix key sharing: crypto not available');
      return;
    }

    try {
      console.log('🔑 Fixing encryption key sharing issues...');
      
      // Get our own devices for key sharing
      const ownDevices = await this.client.crypto.getStoredDevicesForUser(this.config.userId!);
      const verifiedOwnDevices = ownDevices.filter(device => device.isVerified() && !device.isBlocked());
      
      console.log(`🔑 Found ${verifiedOwnDevices.length} verified own devices for key sharing`);
      
      // Ensure Olm sessions with our own devices
      if (verifiedOwnDevices.length > 0 && typeof this.client.crypto.ensureOlmSessionsForDevices === 'function') {
        const deviceMap = { [this.config.userId!]: verifiedOwnDevices };
        await this.client.crypto.ensureOlmSessionsForDevices(deviceMap);
        console.log('🔑 ✅ Ensured Olm sessions with own devices');
      }
      
      // Get encrypted rooms and fix key sharing
      const rooms = this.client.getRooms();
      const encryptedRooms = rooms.filter(room => 
        room.hasEncryptionStateEvent && 
        room.hasEncryptionStateEvent() && 
        room.getMyMembership() === 'join'
      );
      
      console.log(`🔑 Processing ${encryptedRooms.length} encrypted rooms for key sharing`);
      
      let roomsProcessed = 0;
      for (const room of encryptedRooms.slice(0, 5)) { // Process first 5 rooms to avoid overwhelming
        try {
          console.log(`🏠 Processing room: ${room.name || 'Unnamed'} (${room.roomId.substring(0, 20)}...)`);
          
          // Get room members and their devices
          const members = room.getMembers();
          const memberDevices = {};
          
          for (const member of members.slice(0, 10)) { // First 10 members
            try {
              const memberDeviceList = await this.client.crypto.getStoredDevicesForUser(member.userId);
              const validDevices = memberDeviceList.filter(device => 
                device.isVerified() || (!device.isBlocked() && device.isKnown())
              );
              
              if (validDevices.length > 0) {
                memberDevices[member.userId] = validDevices;
              }
            } catch (memberError) {
              console.warn(`🔑 ⚠️ Could not get devices for ${member.userId}:`, memberError.message);
            }
          }
          
          // Ensure Olm sessions for room members
          if (Object.keys(memberDevices).length > 0 && typeof this.client.crypto.ensureOlmSessionsForDevices === 'function') {
            await this.client.crypto.ensureOlmSessionsForDevices(memberDevices);
            console.log(`🔑 ✅ Ensured Olm sessions for ${Object.keys(memberDevices).length} users in room`);
          }
          
          roomsProcessed++;
        } catch (roomError) {
          console.warn(`🔑 ⚠️ Failed to process room ${room.roomId}:`, roomError.message);
        }
      }
      
      // Cancel and resend key requests to fix undecryptable messages
      try {
        if (typeof this.client.crypto.cancelAndResendAllOutgoingKeyRequests === 'function') {
          await this.client.crypto.cancelAndResendAllOutgoingKeyRequests();
          console.log('🔑 ✅ Cancelled and resent all outgoing key requests');
        }
      } catch (keyRequestError) {
        console.warn('🔑 ⚠️ Failed to resend key requests:', keyRequestError.message);
      }
      
      console.log(`🔑 ✅ Key sharing fix completed: processed ${roomsProcessed} rooms`);
      
    } catch (error) {
      console.error('🔑 ❌ Failed to fix encryption key sharing:', error);
    }
  }

  /**
   * Fix encryption key sharing issues (enhanced version)
   */
  private async fixEncryptionKeySharing(): Promise<void> {
    if (!this.client?.crypto) {
      console.log('🔐 Cannot fix key sharing: crypto not available');
      return;
    }

    try {
      console.log('🔑 Fixing encryption key sharing issues (enhanced)...');
      
      // Step 1: Aggressively verify and prepare our own devices
      console.log('🔑 Step 1: Preparing own devices for key sharing...');
      const ownDevices = await this.client.crypto.getStoredDevicesForUser(this.config.userId!);
      console.log(`🔑 Found ${ownDevices.length} own devices`);
      
      let verifiedOwnDevices = [];
      for (const device of ownDevices) {
        try {
          // Force verify all our own devices
          if (!device.isVerified()) {
            await this.client.crypto.setDeviceVerification(this.config.userId!, device.deviceId, true);
            console.log(`🔑 ✅ Force-verified own device: ${device.deviceId.substring(0, 12)}...`);
          }
          
          // Unblock any blocked own devices
          if (device.isBlocked()) {
            await this.client.crypto.setDeviceBlocked(this.config.userId!, device.deviceId, false);
            console.log(`🔑 ✅ Unblocked own device: ${device.deviceId.substring(0, 12)}...`);
          }
          
          // Add to verified list if now verified and not blocked
          if (device.isVerified() && !device.isBlocked()) {
            verifiedOwnDevices.push(device);
          }
        } catch (deviceError) {
          console.warn(`🔑 ⚠️ Failed to prepare device ${device.deviceId}:`, deviceError.message);
        }
      }
      
      console.log(`🔑 Prepared ${verifiedOwnDevices.length} verified own devices`);
      
      // Step 2: Establish Olm sessions with our own devices (critical for self-decryption)
      if (verifiedOwnDevices.length > 0 && typeof this.client.crypto.ensureOlmSessionsForDevices === 'function') {
        const ownDeviceMap = { [this.config.userId!]: verifiedOwnDevices };
        await this.client.crypto.ensureOlmSessionsForDevices(ownDeviceMap);
        console.log('🔑 ✅ Established Olm sessions with own devices');
      }
      
      // Step 3: Process encrypted rooms with enhanced key sharing
      const rooms = this.client.getRooms();
      const encryptedRooms = rooms.filter(room => 
        room.hasEncryptionStateEvent && 
        room.hasEncryptionStateEvent() && 
        room.getMyMembership() === 'join'
      );
      
      console.log(`🔑 Processing ${encryptedRooms.length} encrypted rooms for enhanced key sharing`);
      
      let roomsProcessed = 0;
      for (const room of encryptedRooms.slice(0, 5)) { // Process first 5 rooms to avoid overwhelming
        try {
          console.log(`🏠 Processing room: ${room.name || 'Unnamed'} (${room.roomId.substring(0, 20)}...)`);
          
          // Get room members and their devices
          const members = room.getMembers();
          const memberDevices = {};
          
          for (const member of members.slice(0, 10)) { // First 10 members
            try {
              const memberDeviceList = await this.client.crypto.getStoredDevicesForUser(member.userId);
              
              // Special handling for our own user ID
              if (member.userId === this.config.userId) {
                // For our own devices, use all verified devices
                const validDevices = memberDeviceList.filter(device => device.isVerified() && !device.isBlocked());
                if (validDevices.length > 0) {
                  memberDevices[member.userId] = validDevices;
                  console.log(`🔑 Added ${validDevices.length} own devices for room key sharing`);
                }
              } else {
                // For other users, use verified or known devices
                const validDevices = memberDeviceList.filter(device => 
                  device.isVerified() || (!device.isBlocked() && device.isKnown())
                );
                if (validDevices.length > 0) {
                  memberDevices[member.userId] = validDevices;
                }
              }
            } catch (memberError) {
              console.warn(`🔑 ⚠️ Could not get devices for ${member.userId}:`, memberError.message);
            }
          }
          
          // Ensure Olm sessions for room members
          if (Object.keys(memberDevices).length > 0 && typeof this.client.crypto.ensureOlmSessionsForDevices === 'function') {
            await this.client.crypto.ensureOlmSessionsForDevices(memberDevices);
            console.log(`🔑 ✅ Ensured Olm sessions for ${Object.keys(memberDevices).length} users in room`);
          }
          
          // Step 4: Try to force session refresh for this room
          try {
            if (typeof this.client.crypto.forceDiscardSession === 'function') {
              await this.client.crypto.forceDiscardSession(room.roomId);
              console.log(`🔑 ✅ Discarded stale sessions for room ${room.roomId.substring(0, 20)}...`);
            }
          } catch (discardError) {
            console.warn(`🔑 ⚠️ Could not discard sessions for room:`, discardError.message);
          }
          
          roomsProcessed++;
        } catch (roomError) {
          console.warn(`🔑 ⚠️ Failed to process room ${room.roomId}:`, roomError.message);
        }
      }
      
      // Step 5: Cancel and resend key requests multiple times for persistence
      console.log('🔑 Step 5: Aggressively refreshing key requests...');
      try {
        if (typeof this.client.crypto.cancelAndResendAllOutgoingKeyRequests === 'function') {
          // Do this multiple times to ensure it takes effect
          for (let i = 0; i < 3; i++) {
            await this.client.crypto.cancelAndResendAllOutgoingKeyRequests();
            console.log(`🔑 ✅ Key request refresh attempt ${i + 1}/3 completed`);
            
            // Small delay between attempts
            if (i < 2) {
              await new Promise(resolve => setTimeout(resolve, 1000));
            }
          }
        }
      } catch (keyRequestError) {
        console.warn('🔑 ⚠️ Failed to resend key requests:', keyRequestError.message);
      }
      
      // Step 6: Try to manually trigger decryption for recent encrypted events
      console.log('🔑 Step 6: Attempting to decrypt recent encrypted events...');
      let eventsProcessed = 0;
      
      for (const room of encryptedRooms.slice(0, 3)) { // Process first 3 rooms
        try {
          const timeline = room.getLiveTimeline();
          const events = timeline.getEvents();
          const encryptedEvents = events.filter(event => event.getType() === 'm.room.encrypted');
          const recentEncryptedEvents = encryptedEvents.slice(-5); // Last 5 encrypted events per room
          
          for (const event of recentEncryptedEvents) {
            try {
              const clearEvent = event.getClearEvent();
              if (!clearEvent && typeof event.attemptDecryption === 'function') {
                await event.attemptDecryption(this.client.crypto);
                eventsProcessed++;
              }
            } catch (eventError) {
              // Ignore individual event errors
            }
          }
        } catch (roomError) {
          console.warn(`🔑 ⚠️ Failed to process events for room:`, roomError.message);
        }
      }
      
      if (eventsProcessed > 0) {
        console.log(`🔑 ✅ Attempted decryption for ${eventsProcessed} encrypted events`);
      }
      
      console.log(`🔑 ✅ Enhanced key sharing fix completed: processed ${roomsProcessed} rooms`);
      
    } catch (error) {
      console.error('🔑 ❌ Failed to fix encryption key sharing:', error);
    }
  }

  /**
   * Comprehensive historical message recovery and decryption
   * This method implements multiple strategies to recover and decrypt historical messages
   */
  private async recoverHistoricalMessages(): Promise<void> {
    if (!this.client?.crypto) {
      console.log('🔐 Cannot recover historical messages: crypto not available');
      return;
    }

    try {
      console.log('📜 Starting comprehensive historical message recovery...');
      
      // Step 1: Request keys for all undecryptable messages
      await this.requestKeysForUndecryptableMessages();
      
      // Step 2: Check for and import key backup if available
      await this.checkAndImportKeyBackup();
      
      // Step 3: Request room keys from other devices and participants
      await this.requestRoomKeysFromParticipants();
      
      // Step 4: Force re-attempt decryption on all encrypted events
      await this.forceRetryDecryption();
      
      console.log('📜 ✅ Historical message recovery completed');
      
    } catch (error) {
      console.error('📜 ❌ Failed to recover historical messages:', error);
    }
  }

  /**
   * Request keys for all currently undecryptable messages
   */
  private async requestKeysForUndecryptableMessages(): Promise<void> {
    console.log('📜 Step 1: Requesting keys for undecryptable messages...');
    
    try {
      const rooms = this.client!.getRooms();
      const encryptedRooms = rooms.filter(room => 
        room.hasEncryptionStateEvent && 
        room.hasEncryptionStateEvent() && 
        room.getMyMembership() === 'join'
      );
      
      let keysRequested = 0;
      
      for (const room of encryptedRooms) {
        try {
          const timeline = room.getLiveTimeline();
          const events = timeline.getEvents();
          const encryptedEvents = events.filter(event => event.getType() === 'm.room.encrypted');
          
          for (const event of encryptedEvents) {
            try {
              // Check if this event is undecryptable
              const clearEvent = event.getClearEvent();
              const isDecryptionFailure = typeof event.isDecryptionFailure === 'function' && event.isDecryptionFailure();
              
              if (!clearEvent || isDecryptionFailure) {
                // This event needs key recovery
                console.log(`📜 🔑 Requesting key for undecryptable event: ${event.getId()?.substring(0, 20)}...`);
                
                // Try multiple key request methods
                if (typeof this.client!.crypto!.requestRoomKey === 'function') {
                  await this.client!.crypto!.requestRoomKey(event);
                  keysRequested++;
                } else if (typeof event.requestKey === 'function') {
                  await event.requestKey();
                  keysRequested++;
                }
              }
            } catch (eventError) {
              console.warn(`📜 ⚠️ Failed to request key for event:`, eventError);
            }
          }
        } catch (roomError) {
          console.warn(`📜 ⚠️ Failed to process room for key requests:`, roomError);
        }
      }
      
      console.log(`📜 🔑 Requested keys for ${keysRequested} undecryptable messages`);
      
    } catch (error) {
      console.error('📜 ❌ Failed to request keys for undecryptable messages:', error);
    }
  }

  /**
   * Check for and import server-side key backup
   */
  private async checkAndImportKeyBackup(): Promise<void> {
    console.log('📜 Step 2: Checking for server-side key backup...');
    
    try {
      if (!this.client?.crypto) return;
      
      // Check if key backup is available and configured
      if (typeof this.client.crypto.checkKeyBackup === 'function') {
        const backupInfo = await this.client.crypto.checkKeyBackup();
        
        if (backupInfo) {
          console.log('📜 🔑 Found key backup, attempting to restore keys...');
          
          // Try to restore keys from backup
          if (typeof this.client.crypto.restoreKeyBackup === 'function') {
            try {
              const restored = await this.client.crypto.restoreKeyBackup();
              console.log(`📜 ✅ Restored ${restored.total} keys from backup (${restored.imported} imported)`);
            } catch (restoreError) {
              console.warn('📜 ⚠️ Failed to restore from key backup:', restoreError);
            }
          }
        } else {
          console.log('📜 ℹ️ No key backup found on server');
        }
      } else {
        console.log('📜 ℹ️ Key backup methods not available');
      }
      
    } catch (error) {
      console.error('📜 ❌ Failed to check key backup:', error);
    }
  }

  /**
   * Request room keys from other devices and participants
   */
  private async requestRoomKeysFromParticipants(): Promise<void> {
    console.log('📜 Step 3: Requesting room keys from other devices and participants...');
    
    try {
      const rooms = this.client!.getRooms();
      const encryptedRooms = rooms.filter(room => 
        room.hasEncryptionStateEvent && 
        room.hasEncryptionStateEvent() && 
        room.getMyMembership() === 'join'
      );
      
      for (const room of encryptedRooms.slice(0, 3)) { // Process first 3 rooms
        try {
          console.log(`📜 🔑 Requesting room keys for: ${room.name || 'Unnamed'} (${room.roomId.substring(0, 20)}...)`);
          
          // Get all participants in the room
          const members = room.getMembers();
          const participantDevices = {};
          
          for (const member of members) {
            try {
              const devices = await this.client!.crypto!.getStoredDevicesForUser(member.userId);
              const verifiedDevices = devices.filter(device => 
                device.isVerified() || (!device.isBlocked() && device.isKnown())
              );
              
              if (verifiedDevices.length > 0) {
                participantDevices[member.userId] = verifiedDevices;
              }
            } catch (memberError) {
              console.warn(`📜 ⚠️ Failed to get devices for ${member.userId}:`, memberError);
            }
          }
          
          // Request room keys from all participants
          if (Object.keys(participantDevices).length > 0) {
            try {
              // Try different methods to request room keys
              if (typeof this.client!.crypto!.requestRoomKeyFromDevices === 'function') {
                await this.client!.crypto!.requestRoomKeyFromDevices(room.roomId, participantDevices);
                console.log(`📜 ✅ Requested room keys from ${Object.keys(participantDevices).length} participants`);
              } else if (typeof this.client!.crypto!.sendRoomKeyRequest === 'function') {
                // Alternative method
                for (const [userId, devices] of Object.entries(participantDevices)) {
                  for (const device of devices as any[]) {
                    try {
                      await this.client!.crypto!.sendRoomKeyRequest({
                        room_id: room.roomId,
                        algorithm: 'm.megolm.v1.aes-sha2',
                        requesting_device_id: this.config.deviceId!,
                        request_id: `req_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
                      }, [{ userId, deviceId: device.deviceId }]);
                    } catch (deviceRequestError) {
                      console.warn(`📜 ⚠️ Failed to request keys from device ${device.deviceId}:`, deviceRequestError);
                    }
                  }
                }
                console.log(`📜 ✅ Sent individual key requests to participants`);
              }
            } catch (requestError) {
              console.warn(`📜 ⚠️ Failed to request room keys:`, requestError);
            }
          }
          
        } catch (roomError) {
          console.warn(`📜 ⚠️ Failed to request keys for room:`, roomError);
        }
      }
      
    } catch (error) {
      console.error('📜 ❌ Failed to request room keys from participants:', error);
    }
  }

  /**
   * Force retry decryption on all encrypted events
   */
  private async forceRetryDecryption(): Promise<void> {
    console.log('📜 Step 4: Force retrying decryption on all encrypted events...');
    
    try {
      const rooms = this.client!.getRooms();
      const encryptedRooms = rooms.filter(room => 
        room.hasEncryptionStateEvent && 
        room.hasEncryptionStateEvent() && 
        room.getMyMembership() === 'join'
      );
      
      let decryptionAttempts = 0;
      let successfulDecryptions = 0;
      
      for (const room of encryptedRooms.slice(0, 3)) { // Process first 3 rooms
        try {
          const timeline = room.getLiveTimeline();
          const events = timeline.getEvents();
          const encryptedEvents = events.filter(event => event.getType() === 'm.room.encrypted');
          
          console.log(`📜 🔓 Attempting to decrypt ${encryptedEvents.length} encrypted events in room: ${room.name || 'Unnamed'}`);
          
          for (const event of encryptedEvents) {
            try {
              decryptionAttempts++;
              
              // Check current decryption status
              const clearEvent = event.getClearEvent();
              const isDecryptionFailure = typeof event.isDecryptionFailure === 'function' && event.isDecryptionFailure();
              
              if (!clearEvent || isDecryptionFailure) {
                // Try to decrypt this event
                if (typeof event.attemptDecryption === 'function') {
                  await event.attemptDecryption(this.client!.crypto!);
                  
                  // Check if decryption succeeded
                  const newClearEvent = event.getClearEvent();
                  if (newClearEvent && newClearEvent.content && newClearEvent.content.body) {
                    successfulDecryptions++;
                    console.log(`📜 ✅ Successfully decrypted event: ${event.getId()?.substring(0, 20)}...`);
                    
                    // Emit an event to notify UI that a message was decrypted
                    this.emit('messageDecrypted', {
                      roomId: room.roomId,
                      eventId: event.getId(),
                      content: newClearEvent.content.body,
                    });
                  }
                }
              }
            } catch (eventError) {
              console.warn(`📜 ⚠️ Failed to decrypt event ${event.getId()}:`, eventError);
            }
          }
        } catch (roomError) {
          console.warn(`📜 ⚠️ Failed to process room for decryption retry:`, roomError);
        }
      }
      
      console.log(`📜 🔓 Decryption retry completed: ${successfulDecryptions}/${decryptionAttempts} events successfully decrypted`);
      
      if (successfulDecryptions > 0) {
        // Emit a general event that historical messages were recovered
        this.emit('historicalMessagesRecovered', {
          decryptedCount: successfulDecryptions,
          totalAttempts: decryptionAttempts,
        });
      }
      
    } catch (error) {
      console.error('📜 ❌ Failed to force retry decryption:', error);
    }
  }

  /**
   * Auto-verify all devices in encrypted rooms for development (aggressive approach)
   */
  private async autoVerifyAllDevicesForDevelopment(): Promise<void> {
    if (!this.client?.crypto) {
      console.log('🔐 Cannot auto-verify devices: crypto not available');
      return;
    }

    try {
      console.log('🔐 Auto-verifying all devices for development...');
      
      // Get all rooms and find encrypted ones
      const rooms = this.client.getRooms();
      const encryptedRooms = rooms.filter(room => room.hasEncryptionStateEvent && room.hasEncryptionStateEvent());
      
      console.log(`🔐 Found ${encryptedRooms.length} encrypted rooms to process`);
      
      let devicesVerified = 0;
      let devicesUnblocked = 0;

      for (const room of encryptedRooms) {
        const members = room.getMembers();
        
        for (const member of members) {
          try {
            const devices = await this.client.crypto.getStoredDevicesForUser(member.userId);
            
            for (const device of devices) {
              // Verify device if not already verified
              if (!device.isVerified()) {
                try {
                  await this.client.crypto.setDeviceVerification(member.userId, device.deviceId, true);
                  devicesVerified++;
                  console.log(`🔐 ✅ Auto-verified device ${device.deviceId.substring(0, 8)}... for ${member.userId}`);
                } catch (verifyError) {
                  console.warn(`🔐 ⚠️ Failed to verify device ${device.deviceId} for ${member.userId}:`, verifyError);
                }
              }
              
              // Unblock device if blocked
              if (device.isBlocked()) {
                try {
                  await this.client.crypto.setDeviceBlocked(member.userId, device.deviceId, false);
                  devicesUnblocked++;
                  console.log(`🔐 ✅ Auto-unblocked device ${device.deviceId.substring(0, 8)}... for ${member.userId}`);
                } catch (unblockError) {
                  console.warn(`🔐 ⚠️ Failed to unblock device ${device.deviceId} for ${member.userId}:`, unblockError);
                }
              }
            }
          } catch (memberError) {
            console.warn(`🔐 ⚠️ Failed to process devices for ${member.userId}:`, memberError);
          }
        }
      }

      if (devicesVerified > 0 || devicesUnblocked > 0) {
        console.log(`🔐 ✅ Auto-verification complete: ${devicesVerified} devices verified, ${devicesUnblocked} devices unblocked`);
      } else {
        console.log('🔐 ✅ All devices already verified and unblocked');
      }

    } catch (error) {
      console.error('🔐 ❌ Failed to auto-verify devices:', error);
    }
  }

  /**
   * Initialize crypto if it's not available (for late initialization)
   */
  private async initializeCryptoIfNeeded(): Promise<boolean> {
    if (this.client?.crypto) {
      console.log('🔐 Crypto already available');
      return true;
    }

    if (!this.client) {
      console.log('🔐 No client available for crypto initialization');
      return false;
    }

    console.log('🔐 Attempting to initialize crypto module...');
    
    try {
      // First ensure Olm is available
      await this.initializeOlm();
      
      // Try to initialize crypto
      if (typeof this.client.initCrypto === 'function') {
        await this.client.initCrypto();
        
        if (this.client.crypto) {
          console.log('🔐 ✅ Crypto module initialized successfully');
          
          // Configure crypto settings
          this.client.setGlobalBlacklistUnverifiedDevices(false);
          console.log('🔐 ✅ Configured to allow unverified devices');
          
          // Handle key upload issues gracefully
          await this.handleKeyUploadIssues();
          
          // Verify our own device to prevent UnknownDeviceError
          await this.verifyOwnDevice();
          
          return true;
        }
      }
      
      console.log('🔐 ⚠️ Crypto initialization failed or not available');
      return false;
      
    } catch (error) {
      console.error('🔐 ❌ Failed to initialize crypto:', error);
      return false;
    }
  }

  /**
   * Initialize Olm library for end-to-end encryption
   */
  private async initializeOlm(): Promise<void> {
    if (olmInitialized) {
      console.log('🔐 Olm already initialized');
      return;
    }

    try {
      console.log('🔐 Initializing Olm library for end-to-end encryption...');
      
      // Check if Olm is already available globally
      const globalObj = (typeof global !== 'undefined' ? global : window) as any;
      if ((window as any).Olm && globalObj.Olm) {
        console.log('🔐 Olm already available globally');
        olmInitialized = true;
        return;
      }
      
      // Try different approaches to load Olm
      let Olm: any = null;
      
      try {
        // First try: Load from public directory (most reliable for Vite dev server)
        console.log('🔐 Attempting to load Olm from public directory...');
        const response = await fetch('/olm.js'); // Use absolute path
        if (response.ok && response.headers.get('content-type')?.includes('javascript')) {
          const olmScript = await response.text();
          console.log('🔐 Successfully fetched Olm script from public directory, length:', olmScript.length);
          
          // Validate that it's actually the Olm script
          if (olmScript.includes('var Olm = (function()') || olmScript.includes('function') && olmScript.includes('Olm')) {
            console.log('🔐 Script validated as Olm JavaScript');
            
            // Execute the script in a way that makes Olm available
            const scriptElement = document.createElement('script');
            scriptElement.textContent = olmScript;
            document.head.appendChild(scriptElement);
            
            // Wait for the script to execute
            await new Promise(resolve => setTimeout(resolve, 500));
            
            if ((window as any).Olm) {
              Olm = (window as any).Olm;
              console.log('🔐 Successfully loaded Olm from public directory');
            } else {
              throw new Error('Olm not available after loading script');
            }
          } else {
            throw new Error('Fetched content does not appear to be Olm JavaScript');
          }
        } else {
          throw new Error(`Failed to fetch Olm script: ${response.status} or wrong content-type: ${response.headers.get('content-type')}`);
        }
      } catch (publicDirError) {
        console.warn('🔐 Public directory loading failed:', publicDirError);
        
        try {
          // Second try: dynamic import (fallback for production builds)
          console.log('🔐 Attempting dynamic import of Olm...');
          const olmModule = await import('@matrix-org/olm');
          
          // Handle both default export and direct export patterns
          if (olmModule.default) {
            Olm = olmModule.default;
            console.log('🔐 Using default export from dynamic import');
          } else {
            Olm = olmModule;
            console.log('🔐 Using direct export from dynamic import');
          }
          
          console.log('🔐 Dynamic import successful, Olm type:', typeof Olm);
        } catch (importError) {
          console.warn('🔐 Dynamic import failed:', importError);
          
          try {
            // Third try: require (for Node.js environments)
            console.log('🔐 Attempting require of Olm...');
            Olm = require('@matrix-org/olm');
            console.log('🔐 Require successful');
          } catch (requireError) {
            console.warn('🔐 Require failed:', requireError);
            
            // Fourth try: check if it's available on window (pre-loaded)
            if ((window as any).Olm) {
              Olm = (window as any).Olm;
              console.log('🔐 Found Olm on window object');
            } else {
              throw new Error('Unable to load Olm library through any method');
            }
          }
        }
      }
      
      if (!Olm) {
        throw new Error('Olm library not found');
      }
      
      // Initialize Olm if it has an init method
      if (typeof Olm.init === 'function') {
        console.log('🔐 Calling Olm.init()...');
        await Olm.init();
        console.log('🔐 Olm.init() completed');
      } else if (typeof Olm.default?.init === 'function') {
        console.log('🔐 Calling Olm.default.init()...');
        await Olm.default.init();
        Olm = Olm.default;
        console.log('🔐 Olm.default.init() completed');
      } else {
        console.log('🔐 No init method found, assuming Olm is ready');
      }
      
      // Make Olm available globally for matrix-js-sdk
      (globalObj as any).Olm = Olm; // Use globalObj for assignment
      (window as any).Olm = Olm;
      
      olmInitialized = true;
      console.log('🔐 ✅ Olm library initialized successfully');
      
    } catch (error) {
      console.error('🔐 ❌ Failed to initialize Olm library:', error);
      
      // For development/testing, we can continue without encryption
      console.warn('🔐 ⚠️ Continuing without end-to-end encryption support');
      console.warn('🔐 ⚠️ This means encrypted rooms will not work properly');
      
      // Don't throw the error - let the Matrix client work without encryption
      olmInitialized = true; // Mark as initialized to prevent retries
    }
  }

  /**
   * Initialize and start the Matrix client
   */
  async initialize(): Promise<void> {
    try {
      // Initialize Olm library first for encryption support
      await this.initializeOlm();
      
      // Try to load stored credentials first
      const storedConfig = await this.loadCredentials();
      if (storedConfig) {
        console.log('🔄 Attempting auto-login with stored credentials...');
        this.config = { ...this.config, ...storedConfig };
      }

      if (this.config.accessToken) {
        // Use existing access token (either passed in or loaded from storage)
        console.log('🔐 Creating client with crypto store...');
        this.client = sdk.createClient({
          baseUrl: this.config.homeserverUrl,
          accessToken: this.config.accessToken,
          userId: this.config.userId,
          deviceId: this.config.deviceId,
          // Enable crypto for encryption support with persistence
          cryptoStore: new sdk.LocalStorageCryptoStore(localStorage, 'goose-matrix-crypto'),
          // Don't specify verificationMethods to avoid TypeError
        });
        
        // Initialize crypto immediately after client creation
        console.log('🔐 Initializing crypto module...');
        try {
          if (typeof this.client.initCrypto === 'function') {
            await this.client.initCrypto();
            console.log('🔐 ✅ Crypto module initialized successfully');
            console.log('🔐 Crypto module available:', !!this.client.crypto);
            
            // Configure crypto to allow sending to unverified devices (for development)
            if (this.client.crypto) {
              this.client.setGlobalBlacklistUnverifiedDevices(false);
              console.log('🔐 ✅ Configured to allow unverified devices');
              
              // Handle key upload issues gracefully
              await this.handleKeyUploadIssues();
              
              // Verify our own device to prevent UnknownDeviceError
              await this.verifyOwnDevice();
            }
          } else {
            console.log('🔐 ℹ️ initCrypto method not available');
          }
        } catch (cryptoError) {
          console.warn('🔐 ⚠️ Failed to initialize crypto module:', cryptoError);
          // Try alternative initialization
          try {
            console.log('🔐 Trying alternative crypto initialization...');
            if (this.client.crypto) {
              console.log('🔐 Crypto module exists, trying to start it...');
            } else {
              console.log('🔐 No crypto module found, recreating client with different config...');
              // Recreate client with explicit crypto configuration
              this.client = sdk.createClient({
                baseUrl: this.config.homeserverUrl,
                accessToken: this.config.accessToken,
                userId: this.config.userId,
                deviceId: this.config.deviceId,
                cryptoStore: new sdk.LocalStorageCryptoStore(localStorage, 'goose-matrix-crypto'),
                // Try with explicit crypto callbacks
                cryptoCallbacks: {},
              });
              
              if (typeof this.client.initCrypto === 'function') {
                await this.client.initCrypto();
                console.log('🔐 ✅ Alternative crypto initialization successful');
              }
            }
          } catch (altError) {
            console.warn('🔐 ⚠️ Alternative crypto initialization also failed:', altError);
          }
        }
        
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
    console.log('🔐 LOGIN ATTEMPT: ========== STARTING LOGIN PROCESS ==========');
    console.log('🔐 LOGIN ATTEMPT: Username:', username);
    console.log('🔐 LOGIN ATTEMPT: Password length:', password?.length || 0);
    console.log('🔐 LOGIN ATTEMPT: Client initialized:', !!this.client);
    console.log('🔐 LOGIN ATTEMPT: Client type:', this.client?.constructor?.name);
    console.log('🔐 LOGIN ATTEMPT: Homeserver URL:', this.config.homeserverUrl);
    console.log('🔐 LOGIN ATTEMPT: Current config:', JSON.stringify(this.config, null, 2));
    console.log('🔐 LOGIN ATTEMPT: Client methods available:', this.client ? Object.getOwnPropertyNames(this.client).filter(name => typeof (this.client as any)[name] === 'function').slice(0, 10) : 'none');
    
    // Initialize Olm library first for encryption support
    console.log('🔐 LOGIN ATTEMPT: Initializing Olm library...');
    await this.initializeOlm();
    
    // Create a basic client for login if one doesn't exist
    if (!this.client) {
      console.log('🔐 LOGIN ATTEMPT: Creating basic client for login...');
      this.client = sdk.createClient({
        baseUrl: this.config.homeserverUrl,
      });
      this.setupEventListeners();
    }

    if (!username || !password) {
      console.error('🔐 LOGIN ERROR: Missing username or password');
      throw new Error('Username and password are required');
    }

    try {
      console.log('🔐 LOGIN ATTEMPT: About to call client.login...');
      console.log('🔐 LOGIN ATTEMPT: Login method exists:', typeof this.client.login === 'function');
      console.log('🔐 LOGIN ATTEMPT: Login parameters:', {
        method: 'm.login.password',
        user: username,
        passwordProvided: !!password
      });
      
      const response = await this.client.login('m.login.password', {
        user: username,
        password: password,
      });

      this.config.accessToken = response.access_token;
      this.config.userId = response.user_id;
      this.config.deviceId = response.device_id;

      // Recreate client with credentials
      console.log('🔐 Creating client with crypto store after login...');
      this.client = sdk.createClient({
        baseUrl: this.config.homeserverUrl,
        accessToken: this.config.accessToken,
        userId: this.config.userId,
        deviceId: this.config.deviceId,
        // Enable crypto for encryption support with persistence
        cryptoStore: new sdk.LocalStorageCryptoStore(localStorage, 'goose-matrix-crypto'),
        // Don't specify verificationMethods to avoid TypeError
      });

      // Initialize crypto immediately after client creation
      console.log('🔐 Initializing crypto module after login...');
      try {
        if (typeof this.client.initCrypto === 'function') {
          await this.client.initCrypto();
          console.log('🔐 ✅ Crypto module initialized successfully after login');
          console.log('🔐 Crypto module available after login:', !!this.client.crypto);
          
          // Configure crypto to allow sending to unverified devices (for development)
          if (this.client.crypto) {
            this.client.setGlobalBlacklistUnverifiedDevices(false);
            console.log('🔐 ✅ Configured to allow unverified devices after login');
            
            // Handle key upload issues gracefully
            await this.handleKeyUploadIssues();
            
            // Verify our own device to prevent UnknownDeviceError
            await this.verifyOwnDevice();
          }
        } else {
          console.log('🔐 ℹ️ initCrypto method not available after login');
        }
      } catch (cryptoError) {
        console.warn('🔐 ⚠️ Failed to initialize crypto module after login:', cryptoError);
        // Try alternative initialization
        try {
          console.log('🔐 Trying alternative crypto initialization after login...');
          if (this.client.crypto) {
            console.log('🔐 Crypto module exists after login, trying to start it...');
          } else {
            console.log('🔐 No crypto module found after login, recreating client...');
            // Recreate client with explicit crypto configuration
            this.client = sdk.createClient({
              baseUrl: this.config.homeserverUrl,
              accessToken: this.config.accessToken,
              userId: this.config.userId,
              deviceId: this.config.deviceId,
              cryptoStore: new sdk.LocalStorageCryptoStore(localStorage, 'goose-matrix-crypto'),
              // Try with explicit crypto callbacks
              cryptoCallbacks: {},
            });
            
            if (typeof this.client.initCrypto === 'function') {
              await this.client.initCrypto();
              console.log('🔐 ✅ Alternative crypto initialization successful after login');
            }
          }
        } catch (altError) {
          console.warn('🔐 ⚠️ Alternative crypto initialization also failed after login:', altError);
        }
      }

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
      console.error('🔐 LOGIN ERROR: Caught error during login process:', error);
      console.error('🔐 LOGIN ERROR: Error type:', typeof error);
      console.error('🔐 LOGIN ERROR: Error name:', error.name);
      console.error('🔐 LOGIN ERROR: Error message:', error.message);
      console.error('🔐 LOGIN ERROR: Error stack:', error.stack);
      console.error('🔐 LOGIN ERROR: Error httpStatus:', error.httpStatus);
      console.error('🔐 LOGIN ERROR: Error data:', error.data);
      console.error('🔐 LOGIN ERROR: Full error object:', JSON.stringify(error, null, 2));
      
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
      } else if (error.message) {
        errorMessage = `Login failed: ${error.message}`;
      }

      console.error('🔐 LOGIN ERROR: Final error message:', errorMessage);
      
      const enhancedError = new Error(errorMessage);
      this.emit('error', enhancedError);
      throw enhancedError;
    }
  }

  /**
   * Register a new account
   */
  async register(username: string, password: string): Promise<void> {
    // Initialize Olm library first for encryption support
    await this.initializeOlm();
    
    if (!this.client) {
      throw new Error('Client not initialized');
    }

    try {
      const response = await this.client.register(username, password);
      
      this.config.accessToken = response.access_token;
      this.config.userId = response.user_id;
      this.config.deviceId = response.device_id;

      // Recreate client with credentials
      console.log('🔐 Creating client with crypto store after registration...');
      this.client = sdk.createClient({
        baseUrl: this.config.homeserverUrl,
        accessToken: this.config.accessToken,
        userId: this.config.userId,
        deviceId: this.config.deviceId,
        // Enable crypto for encryption support with persistence
        cryptoStore: new sdk.LocalStorageCryptoStore(localStorage, 'goose-matrix-crypto'),
        // Don't specify verificationMethods to avoid TypeError
      });

      // Initialize crypto immediately after client creation
      console.log('🔐 Initializing crypto module after registration...');
      try {
        if (typeof this.client.initCrypto === 'function') {
          await this.client.initCrypto();
          console.log('🔐 ✅ Crypto module initialized successfully after registration');
          console.log('🔐 Crypto module available after registration:', !!this.client.crypto);
          
          // Configure crypto to allow sending to unverified devices (for development)
          if (this.client.crypto) {
            this.client.setGlobalBlacklistUnverifiedDevices(false);
            console.log('🔐 ✅ Configured to allow unverified devices after registration');
            
            // Handle key upload issues gracefully
            await this.handleKeyUploadIssues();
            
            // Verify our own device to prevent UnknownDeviceError
            await this.verifyOwnDevice();
          }
        } else {
          console.log('🔐 ℹ️ initCrypto method not available after registration');
        }
      } catch (cryptoError) {
        console.warn('🔐 ⚠️ Failed to initialize crypto module after registration:', cryptoError);
        // Try alternative initialization
        try {
          console.log('🔐 Trying alternative crypto initialization after registration...');
          if (this.client.crypto) {
            console.log('🔐 Crypto module exists after registration, trying to start it...');
          } else {
            console.log('🔐 No crypto module found after registration, recreating client...');
            // Recreate client with explicit crypto configuration
            this.client = sdk.createClient({
              baseUrl: this.config.homeserverUrl,
              accessToken: this.config.accessToken,
              userId: this.config.userId,
              deviceId: this.config.deviceId,
              cryptoStore: new sdk.LocalStorageCryptoStore(localStorage, 'goose-matrix-crypto'),
              // Try with explicit crypto callbacks
              cryptoCallbacks: {},
            });
            
            if (typeof this.client.initCrypto === 'function') {
              await this.client.initCrypto();
              console.log('🔐 ✅ Alternative crypto initialization successful after registration');
            }
          }
        } catch (altError) {
          console.warn('🔐 ⚠️ Alternative crypto initialization also failed after registration:', altError);
        }
      }

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
        setTimeout(async () => {
          this.isInitialSync = false;
          console.log('🔄 Initial sync complete - now processing new invitations');
          
          // Auto-verify all devices for development after sync is complete
          try {
            await this.autoVerifyAllDevicesForDevelopment();
          } catch (error) {
            console.warn('🔐 ⚠️ Auto-verification after sync failed (non-critical):', error);
          }
          
          // Fix encryption key sharing issues after sync is complete
          try {
            await this.fixEncryptionKeySharing();
          } catch (error) {
            console.warn('🔑 ⚠️ Key sharing fix after sync failed (non-critical):', error);
          }
          
          // Recover historical messages after sync is complete
          try {
            await this.recoverHistoricalMessages();
          } catch (error) {
            console.warn('📜 ⚠️ Historical message recovery failed (non-critical):', error);
          }
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
      
      // Handle both regular messages AND encrypted messages
      if (event.getType() === 'm.room.message' || event.getType() === 'm.room.encrypted') {
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

    // Listen for room state events to catch any room changes
    this.client.on('RoomState.events', (event, state, lastStateEvent) => {
      const eventType = event.getType();
      const roomId = event.getRoomId();
      
      console.log('🏠 RoomState.events:', {
        eventType,
        roomId: roomId?.substring(0, 20) + '...',
        stateKey: event.getStateKey(),
      });
      
      // Clear rooms cache for any room state change that might affect visibility
      // This includes: name, topic, avatar, power levels, join rules, etc.
      if (eventType === 'm.room.name' || 
          eventType === 'm.room.topic' || 
          eventType === 'm.room.avatar' ||
          eventType === 'm.room.join_rules' ||
          eventType === 'm.room.power_levels' ||
          eventType === 'm.room.canonical_alias' ||
          eventType === 'm.room.tombstone') { // Room was upgraded/replaced
        
        console.log('🏠 Room state changed, clearing rooms cache:', eventType);
        this.cachedRooms = null;
        
        // Emit specific events for UI updates
        if (eventType === 'm.room.tombstone') {
          console.log('🪦 Room tombstoned (upgraded/deleted):', roomId);
          this.emit('roomTombstoned', { roomId, event });
        }
      }
    });

    // Listen for room deletions/leaves
    this.client.on('Room.myMembership', (room, membership, prevMembership) => {
      console.log('🏠 Room.myMembership changed:', {
        roomId: room.roomId.substring(0, 20) + '...',
        roomName: room.name,
        membership,
        prevMembership,
      });
      
      // Clear rooms cache when our membership changes
      this.cachedRooms = null;
      this.cachedFriends = null;
      
      // If we left or were kicked/banned, emit event
      if (membership === 'leave' || membership === 'ban') {
        console.log('👋 Left or banned from room:', room.roomId);
        this.emit('roomLeft', { roomId: room.roomId, membership, prevMembership });
      }
    });

    // Listen for decryption events to handle newly decrypted messages
    this.client.on('Event.decrypted', (event) => {
      console.log('🔓 Event decrypted:', {
        eventType: event.getType(),
        roomId: event.getRoomId(),
        sender: event.getSender(),
        eventId: event.getId()
      });
      
      // If this is a message event that was just decrypted, re-process it
      if (event.getType() === 'm.room.encrypted') {
        const room = this.client?.getRoom(event.getRoomId());
        if (room) {
          console.log('🔓 Re-processing decrypted message for real-time display');
          this.handleMessage(event, room);
        }
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
    
    // CRITICAL FIX: Extract actual message content, handling both encrypted and unencrypted messages
    let actualMessageContent = '';
    let shouldEmitMessage = true; // Flag to control whether we should emit this message
    
    if (event.getType() === 'm.room.encrypted') {
      // Handle encrypted messages
      console.log('🔒 Processing encrypted message, crypto available:', !!this.client?.crypto);
      
      if (this.client?.crypto && typeof event.getClearEvent === 'function') {
        try {
          const clearEvent = event.getClearEvent();
          if (clearEvent && clearEvent.content && clearEvent.content.body) {
            actualMessageContent = clearEvent.content.body;
            console.log('🔓 Successfully extracted content from decrypted message:', actualMessageContent.substring(0, 50) + '...');
          } else if (typeof event.isDecryptionFailure === 'function' && event.isDecryptionFailure()) {
            const failureReason = event.decryptionFailureReason || 'Unknown encryption error';
            actualMessageContent = `🔒 [Unable to decrypt: ${failureReason}]`;
            console.log('🔒 Decryption failure, using fallback content:', failureReason);
          } else {
            // Message not yet decrypted - DON'T emit a placeholder, just wait
            console.log('🔄 Message not yet decrypted, skipping emission and attempting decryption');
            shouldEmitMessage = false;
            
            // Try to manually trigger decryption
            if (typeof event.attemptDecryption === 'function') {
              console.log('🔄 Attempting manual decryption...');
              event.attemptDecryption(this.client.crypto).then(() => {
                const newClearEvent = event.getClearEvent();
                if (newClearEvent && newClearEvent.content && newClearEvent.content.body) {
                  console.log('🔓 Manual decryption successful, re-processing message');
                  // Re-process the message after successful decryption
                  setTimeout(() => this.handleMessage(event, room), 100);
                }
              }).catch(decryptError => {
                console.warn('🔒 Manual decryption failed:', decryptError);
              });
            }
          }
        } catch (decryptError) {
          console.warn('❌ Failed to extract decrypted content:', decryptError);
          actualMessageContent = `🔒 [Encryption error: ${decryptError.message || 'Unknown'}]`;
        }
      } else {
        // Crypto not available - DON'T emit a placeholder, just try to initialize
        console.log('🔒 Crypto not available for encrypted message, attempting to initialize...');
        shouldEmitMessage = false;
        
        // Try to initialize crypto if it's not available
        this.initializeCryptoIfNeeded().then(cryptoAvailable => {
          if (cryptoAvailable) {
            console.log('🔓 Crypto initialized, re-processing encrypted message');
            // Re-process the message after crypto initialization
            setTimeout(() => this.handleMessage(event, room), 500);
          } else {
            console.log('🔒 Crypto initialization failed, message will remain encrypted');
            // Only now emit a failure message if crypto initialization completely failed
            this.handleMessage(event, room);
          }
        }).catch(initError => {
          console.warn('🔒 Crypto initialization error:', initError);
        });
      }
    } else {
      // Handle unencrypted messages
      actualMessageContent = content.body || '';
    }
    
    // If we shouldn't emit this message (e.g., waiting for decryption), return early
    if (!shouldEmitMessage) {
      console.log('🔄 Skipping message emission - waiting for decryption or crypto initialization');
      return;
    }
    
    // ADDITIONAL SAFETY CHECK: Ensure we never have undefined content
    if (actualMessageContent === undefined || actualMessageContent === null) {
      console.warn('⚠️ Content extraction resulted in undefined/null, using fallback');
      actualMessageContent = '[Message content unavailable]';
    }
    
    // Ensure content is always a string
    if (typeof actualMessageContent !== 'string') {
      console.warn('⚠️ Content is not a string, converting:', typeof actualMessageContent, actualMessageContent);
      actualMessageContent = String(actualMessageContent || '[Invalid content]');
    }
    
    // Debug: Log all incoming messages for troubleshooting
    console.log('🔍 MatrixService.handleMessage called:', {
      roomId: room.roomId,
      sender,
      configUserId: this.config.userId,
      isFromSelf,
      senderEqualsConfig: sender === this.config.userId,
      eventType: event.getType(),
      originalContentBody: content.body?.substring(0, 50) + '...',
      actualMessageContent: actualMessageContent?.substring(0, 50) + '...',
      actualMessageContentType: typeof actualMessageContent,
      actualMessageContentLength: actualMessageContent?.length || 0,
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
      content: actualMessageContent, // Use the properly extracted content
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
    
    try {
      // Send the message
      const result = await this.client.sendEvent(roomId, 'm.room.message', eventContent);
      console.log('💬 ✅ Message sent successfully:', result.event_id);
      
      // After sending, ensure key sharing for this room to prevent decryption issues
      setTimeout(async () => {
        try {
          await this.ensureKeySharingForRoom(roomId);
        } catch (keyError) {
          console.warn('🔑 ⚠️ Key sharing after message send failed (non-critical):', keyError);
        }
      }, 100); // Small delay to let the message propagate
      
    } catch (error) {
      console.error('💬 ❌ Failed to send message:', error);
      throw error;
    }
  }

  /**
   * Ensure proper key sharing for a specific room after sending a message
   */
  private async ensureKeySharingForRoom(roomId: string): Promise<void> {
    if (!this.client?.crypto) {
      console.log('🔑 Cannot ensure key sharing: crypto not available');
      return;
    }

    try {
      console.log('🔑 Ensuring key sharing for room after message send:', roomId.substring(0, 20) + '...');
      
      const room = this.client.getRoom(roomId);
      if (!room || !room.hasEncryptionStateEvent()) {
        console.log('🔑 Room is not encrypted, skipping key sharing');
        return;
      }

      // Get our own devices for key sharing
      const ownDevices = await this.client.crypto.getStoredDevicesForUser(this.config.userId!);
      const verifiedOwnDevices = ownDevices.filter(device => device.isVerified() && !device.isBlocked());
      
      if (verifiedOwnDevices.length > 0) {
        // Ensure Olm sessions with our own devices
        const ownDeviceMap = { [this.config.userId!]: verifiedOwnDevices };
        if (typeof this.client.crypto.ensureOlmSessionsForDevices === 'function') {
          await this.client.crypto.ensureOlmSessionsForDevices(ownDeviceMap);
          console.log('🔑 ✅ Ensured Olm sessions with own devices for room');
        }
      }

      // Get room members and their devices for key sharing
      const members = room.getMembers();
      const memberDevices = {};
      
      for (const member of members.slice(0, 10)) { // First 10 members
        try {
          const memberDeviceList = await this.client.crypto.getStoredDevicesForUser(member.userId);
          const validDevices = memberDeviceList.filter(device => 
            device.isVerified() || (!device.isBlocked() && device.isKnown())
          );
          
          if (validDevices.length > 0) {
            memberDevices[member.userId] = validDevices;
          }
        } catch (memberError) {
          console.warn(`🔑 ⚠️ Could not get devices for ${member.userId}:`, memberError.message);
        }
      }
      
      // Ensure Olm sessions for room members
      if (Object.keys(memberDevices).length > 0 && typeof this.client.crypto.ensureOlmSessionsForDevices === 'function') {
        await this.client.crypto.ensureOlmSessionsForDevices(memberDevices);
        console.log(`🔑 ✅ Ensured Olm sessions for ${Object.keys(memberDevices).length} users in room`);
      }

      // Force key sharing by canceling and resending key requests
      if (typeof this.client.crypto.cancelAndResendAllOutgoingKeyRequests === 'function') {
        await this.client.crypto.cancelAndResendAllOutgoingKeyRequests();
        console.log('🔑 ✅ Refreshed key requests after message send');
      }
      
    } catch (error) {
      console.error('🔑 ❌ Failed to ensure key sharing for room:', error);
    }
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
   * Get the children (rooms and sub-spaces) of a Matrix Space
   */
  async getSpaceChildren(spaceId: string): Promise<SpaceChild[]> {
    if (!this.client) {
      throw new Error('Client not initialized');
    }

    console.log('🌌 Getting children for space:', spaceId);

    try {
      const space = this.client.getRoom(spaceId);
      if (!space) {
        console.error('❌ Space not found:', spaceId);
        return [];
      }

      // Get all m.space.child state events
      const childEvents = space.currentState.getStateEvents('m.space.child');
      const children: SpaceChild[] = [];

      for (const event of childEvents) {
        const childRoomId = event.getStateKey();
        if (!childRoomId) continue;

        const content = event.getContent();
        
        // Skip if the child is deleted (empty content)
        if (!content || Object.keys(content).length === 0) {
          continue;
        }

        // Get information about the child room/space
        const childRoom = this.client.getRoom(childRoomId);
        let childInfo: SpaceChild;

        if (childRoom) {
          // We have local information about this room (we're joined or invited)
          const avatarEvent = childRoom.currentState.getStateEvents('m.room.avatar', '');
          const avatarUrl = avatarEvent?.getContent()?.url || null;
          
          const createEvent = childRoom.currentState.getStateEvents('m.room.create', '');
          const isChildSpace = createEvent?.getContent()?.type === 'm.space';
          
          const joinRulesEvent = childRoom.currentState.getStateEvents('m.room.join_rules', '');
          const isPublic = joinRulesEvent?.getContent()?.join_rule === 'public';

          // Get our membership status in this room
          const membership = childRoom.getMyMembership();
          const canJoin = membership === 'invite' || isPublic || content.suggested;

          childInfo = {
            roomId: childRoomId,
            name: childRoom.name || content.name || 'Unnamed Room',
            topic: childRoom.currentState.getStateEvents('m.room.topic', '')?.getContent()?.topic || content.topic,
            avatarUrl: avatarUrl,
            isSpace: isChildSpace,
            isPublic: isPublic,
            suggested: content.suggested || false,
            via: content.via || [],
            order: content.order,
            memberCount: childRoom.getMembers().length,
            membership: membership as 'join' | 'invite' | 'leave' | 'ban' | null,
            canJoin: canJoin,
          };

          console.log('🌌 Child room with local info:', {
            roomId: childRoomId.substring(0, 20) + '...',
            name: childInfo.name,
            membership: membership,
            canJoin: canJoin,
            isSpace: isChildSpace,
            isPublic: isPublic,
          });
        } else {
          // We don't have local info - we're not joined or invited to this room
          // Use what's available in the space child event and try to determine if we can join
          const isPublicFromContent = content.join_rule === 'public';
          const canJoin = content.suggested || isPublicFromContent;

          childInfo = {
            roomId: childRoomId,
            name: content.name || 'Unknown Room',
            topic: content.topic,
            avatarUrl: content.avatar_url,
            isSpace: content.type === 'm.space', // Check if the space child event indicates it's a space
            isPublic: isPublicFromContent,
            suggested: content.suggested || false,
            via: content.via || [],
            order: content.order,
            memberCount: 0, // We don't know the member count
            membership: null, // We're not a member
            canJoin: canJoin,
          };

          console.log('🌌 Child room without local info:', {
            roomId: childRoomId.substring(0, 20) + '...',
            name: childInfo.name,
            membership: 'null (not joined)',
            canJoin: canJoin,
            suggested: content.suggested,
            isPublic: isPublicFromContent,
          });
        }

        children.push(childInfo);
      }

      // Sort children by order, then by name
      children.sort((a, b) => {
        if (a.order && b.order) {
          return a.order.localeCompare(b.order);
        }
        if (a.order && !b.order) return -1;
        if (!a.order && b.order) return 1;
        return (a.name || '').localeCompare(b.name || '');
      });

      console.log('✅ Found', children.length, 'children in space:', spaceId);
      console.log('🌌 Space children breakdown:', {
        total: children.length,
        joined: children.filter(c => c.membership === 'join').length,
        invited: children.filter(c => c.membership === 'invite').length,
        canJoin: children.filter(c => c.canJoin && c.membership !== 'join').length,
        notAccessible: children.filter(c => !c.canJoin && c.membership !== 'join').length,
      });
      
      return children;
    } catch (error) {
      console.error('❌ Failed to get space children:', error);
      return [];
    }
  }

  /**
   * Add a room or space as a child of a Matrix Space
   */
  async addChildToSpace(spaceId: string, childRoomId: string, suggested: boolean = false, order?: string): Promise<void> {
    if (!this.client) {
      throw new Error('Client not initialized');
    }

    console.log('🌌 Adding child to space:', { spaceId, childRoomId, suggested, order });

    try {
      // Get the child room to determine via servers
      const childRoom = this.client.getRoom(childRoomId);
      let via: string[] = [];
      
      if (childRoom) {
        // Extract server names from room members for via servers
        const members = childRoom.getMembers();
        const servers = new Set<string>();
        members.forEach(member => {
          const serverName = member.userId.split(':')[1];
          if (serverName) {
            servers.add(serverName);
          }
        });
        via = Array.from(servers).slice(0, 3); // Limit to 3 servers
      }

      // Create the space child state event
      const content: any = {
        via: via.length > 0 ? via : [this.config.homeserverUrl.replace('https://', '')],
        suggested: suggested,
      };

      if (order) {
        content.order = order;
      }

      // Set the m.space.child state event
      await this.client.sendStateEvent(spaceId, 'm.space.child', content, childRoomId);

      console.log('✅ Successfully added child to space');
    } catch (error) {
      console.error('❌ Failed to add child to space:', error);
      throw new Error('Failed to add child to space');
    }
  }

  /**
   * Remove a child from a Matrix Space
   */
  async removeChildFromSpace(spaceId: string, childRoomId: string): Promise<void> {
    if (!this.client) {
      throw new Error('Client not initialized');
    }

    console.log('🌌 Removing child from space:', { spaceId, childRoomId });

    try {
      // Remove the m.space.child state event by sending empty content
      await this.client.sendStateEvent(spaceId, 'm.space.child', {}, childRoomId);
      console.log('✅ Successfully removed child from space');
    } catch (error) {
      console.error('❌ Failed to remove child from space:', error);
      throw new Error('Failed to remove child from space');
    }
  }

  /**
   * Create a new Matrix Space
   */
  async createSpace(name: string, topic: string, isPublic: boolean = false): Promise<string> {
    if (!this.client) {
      throw new Error('Client not initialized');
    }

    console.log('🌌 Creating Matrix Space:', { name, topic, isPublic });

    try {
      // Create a Matrix Space room
      const room = await this.client.createRoom({
        name: name,
        topic: topic,
        preset: isPublic ? 'public_chat' : 'private_chat',
        creation_content: {
          type: 'm.space', // This makes it a space instead of a regular room
        },
        initial_state: [
          {
            type: 'm.room.history_visibility',
            content: {
              history_visibility: isPublic ? 'world_readable' : 'invited',
            },
          },
          {
            type: 'm.room.guest_access',
            content: {
              guest_access: isPublic ? 'can_join' : 'forbidden',
            },
          },
        ],
      });

      console.log('✅ Matrix Space created successfully:', room.room_id);
      
      // Create session mapping for the space
      // This ensures the space has a 1:1 relationship with a Goose session
      try {
        const spaceRoom = this.client.getRoom(room.room_id);
        if (spaceRoom) {
          await this.ensureSessionMapping(room.room_id, spaceRoom);
          console.log('📋 Session mapping created for Matrix Space:', room.room_id);
        }
      } catch (mappingError) {
        console.error('❌ Failed to create session mapping for space:', mappingError);
        // Don't fail the space creation if mapping fails
      }
      
      // Clear rooms cache to refresh space data
      this.cachedRooms = null;

      // CRITICAL FIX: Automatically create a default room within the new space
      // This ensures the space appears in the UI immediately.
      try {
        console.log('🌌 Creating default room for new space:', room.room_id);
        const defaultRoomId = await this.createRoom('General', 'General discussion for this space', isPublic, room.room_id);
        console.log('✅ Default room created and added to space:', defaultRoomId);
      } catch (defaultRoomError) {
        console.error('❌ Failed to create default room for space:', defaultRoomError);
        // Don't fail space creation if default room creation fails
      }
      
      return room.room_id;
    } catch (error) {
      console.error('❌ Failed to create Matrix Space:', error);
      throw new Error('Failed to create space');
    }
  }

  /**
   * Create a new Matrix Room (regular room, not a space)
   */
  async createRoom(name: string, topic: string, isPublic: boolean = false, parentSpaceId?: string): Promise<string> {
    if (!this.client) {
      throw new Error('Client not initialized');
    }

    console.log('💬 Creating Matrix Room:', { name, topic, isPublic, parentSpaceId });

    try {
      // Create a regular Matrix room (not a space)
      const room = await this.client.createRoom({
        name: name,
        topic: topic,
        preset: isPublic ? 'public_chat' : 'private_chat',
        // No creation_content.type means it's a regular room
        initial_state: [
          {
            type: 'm.room.history_visibility',
            content: {
              history_visibility: isPublic ? 'world_readable' : 'invited',
            },
          },
          {
            type: 'm.room.guest_access',
            content: {
              guest_access: isPublic ? 'can_join' : 'forbidden',
            },
          },
        ],
      });

      console.log('✅ Matrix Room created successfully:', room.room_id);
      
      // Create session mapping for the room
      // This ensures the room has a 1:1 relationship with a Goose session
      try {
        const matrixRoom = this.client.getRoom(room.room_id);
        if (matrixRoom) {
          await this.ensureSessionMapping(room.room_id, matrixRoom);
          console.log('📋 Session mapping created for Matrix Room:', room.room_id);
        }
      } catch (mappingError) {
        console.error('❌ Failed to create session mapping for room:', mappingError);
        // Don't fail the room creation if mapping fails
      }
      
      // If a parent space is specified, add this room as a child
      if (parentSpaceId) {
        console.log('🌌 Adding room to parent space:', parentSpaceId);
        try {
          await this.addChildToSpace(parentSpaceId, room.room_id, false);
          console.log('✅ Room added to parent space');
        } catch (spaceError) {
          console.error('⚠️ Failed to add room to parent space (room still created):', spaceError);
          // Don't fail the entire room creation if adding to space fails
          // The room is still created successfully, just not linked to the space
        }
      }
      
      // Clear rooms cache to refresh room data
      this.cachedRooms = null;
      
      return room.room_id;
    } catch (error) {
      console.error('❌ Failed to create Matrix Room:', error);
      throw new Error('Failed to create room');
    }
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
        
        // Check for and sync existing history even if already joined
        await this.checkAndSyncRoomHistory(roomId, existingRoom);
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
      
      // Get the room after joining to create session mapping and sync history
      const joinedRoom = this.client.getRoom(roomId);
      if (joinedRoom) {
        // First ensure session mapping exists
        await this.ensureSessionMapping(roomId, joinedRoom);
        
        // Then check for and sync existing chat history
        await this.checkAndSyncRoomHistory(roomId, joinedRoom);
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
   * Leave a Matrix room by room ID
   */
  async leaveRoom(roomId: string): Promise<void> {
    if (!this.client) {
      throw new Error('Client not initialized');
    }

    try {
      console.log('🚪 Attempting to leave room:', roomId);
      
      // Check if we're in the room
      const room = this.client.getRoom(roomId);
      if (!room || room.getMyMembership() !== 'join') {
        console.log('⚠️ Not currently in room:', roomId);
        return;
      }

      // Leave the room
      await this.client.leave(roomId);
      console.log('✅ Successfully left room:', roomId);
      
      // Clear caches to refresh room data
      this.cachedRooms = null;
      this.cachedFriends = null;
      
      // Remove session mapping for this room since we're no longer in it
      try {
        sessionMappingService.removeMapping(roomId);
        console.log('📋 Removed session mapping for left room:', roomId);
      } catch (mappingError) {
        console.warn('⚠️ Failed to remove session mapping:', mappingError);
        // Don't fail the leave operation if mapping removal fails
      }
      
      // Emit leave event
      this.emit('roomLeft', { roomId, membership: 'leave', voluntary: true });
      
    } catch (error: any) {
      console.error('❌ Failed to leave room:', roomId, error);
      
      // Provide more helpful error messages
      let errorMessage = 'Failed to leave room';
      
      if (error.httpStatus === 403) {
        if (error.data?.errcode === 'M_FORBIDDEN') {
          errorMessage = 'You do not have permission to leave this room.';
        } else {
          errorMessage = 'Access forbidden. You may not have permission to leave this room.';
        }
      } else if (error.httpStatus === 404) {
        errorMessage = 'Room not found. The room may have been deleted.';
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
   * Check for existing chat history in a room and sync it to the backend session
   * This is called when joining a room to ensure all existing messages are available in Goose
   */
  private async checkAndSyncRoomHistory(roomId: string, room: any): Promise<void> {
    try {
      console.log('📜 Checking and syncing room history for:', roomId.substring(0, 20) + '...');
      
      // Get the session mapping for this room
      const mapping = sessionMappingService.getMapping(roomId);
      if (!mapping) {
        console.log('📜 No session mapping found for room, skipping history sync:', roomId.substring(0, 20) + '...');
        return;
      }

      // Check if this room has existing message history
      const roomHistory = await this.getRoomHistoryAsGooseMessages(roomId, 100); // Get up to 100 messages
      
      if (roomHistory.length === 0) {
        console.log('📜 No message history found in room:', roomId.substring(0, 20) + '...');
        return;
      }

      console.log(`📜 Found ${roomHistory.length} messages in room history, syncing to backend session:`, mapping.gooseSessionId);

      // Import the API function to sync messages to backend
      try {
        const { syncMessagesToSession } = await import('../api');
        
        // Convert Matrix messages to backend format
        const backendMessages = roomHistory.map((msg, index) => ({
          id: `matrix_${msg.timestamp.getTime()}_${index}`,
          role: msg.role,
          content: [{
            type: 'text' as const,
            text: msg.content,
          }],
          created: Math.floor(msg.timestamp.getTime() / 1000),
          // Include sender info for context
          metadata: {
            matrixSender: msg.metadata?.originalSender || msg.sender,
            matrixSenderInfo: msg.metadata?.senderInfo,
            matrixRoomId: roomId,
            isFromSelf: msg.metadata?.isFromSelf || false,
            syncedAt: Date.now(),
          }
        }));
        
        // Sync messages to backend session
        await syncMessagesToSession({
          body: {
            session_id: mapping.gooseSessionId,
            messages: backendMessages,
          },
          throwOnError: false, // Don't throw on error to prevent breaking the join process
        });
        
        console.log(`📜 ✅ Successfully synced ${backendMessages.length} messages to backend session:`, mapping.gooseSessionId);
        
        // Emit event to notify UI that history has been synced
        this.emit('roomHistorySynced', { 
          roomId, 
          sessionId: mapping.gooseSessionId, 
          messageCount: backendMessages.length 
        });
        
      } catch (apiError) {
        console.warn('📜 ⚠️ syncMessagesToSession API not available, trying alternative approach:', apiError);
        
        // Fallback: try using replyHandler if syncMessagesToSession is not available
        try {
          const { replyHandler } = await import('../api');
          
          // Convert to the format expected by replyHandler
          const backendMessages = roomHistory.map((msg, index) => ({
            id: `matrix_${msg.timestamp.getTime()}_${index}`,
            role: msg.role,
            content: [{
              type: 'text' as const,
              text: msg.content,
            }],
            created: Math.floor(msg.timestamp.getTime() / 1000),
          }));
          
          await replyHandler({
            body: {
              session_id: mapping.gooseSessionId,
              messages: backendMessages,
            },
            throwOnError: false,
          });
          
          console.log(`📜 ✅ Successfully synced ${backendMessages.length} messages using replyHandler:`, mapping.gooseSessionId);
          
          // Emit event to notify UI that history has been synced
          this.emit('roomHistorySynced', { 
            roomId, 
            sessionId: mapping.gooseSessionId, 
            messageCount: backendMessages.length 
          });
          
        } catch (fallbackError) {
          console.error('📜 ❌ Failed to sync room history using fallback method:', fallbackError);
          // Don't throw - we don't want to break the join process if history sync fails
        }
      }
      
    } catch (error) {
      console.error('📜 ❌ Failed to check and sync room history:', error);
      // Don't throw - we don't want to break the join process if history sync fails
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
   * True DMs have exactly 2 members. We'll be more flexible about room names
   * since some Matrix clients might set names for DMs.
   */
  private isDirectMessageRoom(room: any): boolean {
    const memberCount = room.getMembers().length;
    const hasExplicitName = room.name && room.name.trim() !== '';
    
    // Primary check: exactly 2 members (this is the most reliable indicator)
    const isTrueDM = memberCount === 2;
    
    // Additional context for debugging
    const roomDisplayName = room.name || room.getDefaultRoomName?.() || '(auto-generated)';
    
    console.log('🔍 DM Detection:', {
      roomId: room.roomId.substring(0, 20) + '...',
      memberCount,
      roomName: room.name || '(none)',
      roomDisplayName,
      hasExplicitName,
      isTrueDM,
      members: room.getMembers().map((m: any) => m.userId)
    });
    
    return isTrueDM;
  }

  /**
   * Check if a room is a Matrix Space
   */
  private isSpaceRoom(room: any): boolean {
    // Check for room creation event with type m.space
    const createEvent = room.currentState.getStateEvents('m.room.create', '');
    const roomType = createEvent?.getContent()?.type;
    
    return roomType === 'm.space';
  }

  /**
   * Check if a room is public (joinable by anyone)
   */
  private isPublicRoom(room: any): boolean {
    const joinRulesEvent = room.currentState.getStateEvents('m.room.join_rules', '');
    const joinRule = joinRulesEvent?.getContent()?.join_rule;
    
    return joinRule === 'public';
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
    
    // Filter to only include rooms where we are currently joined
    const joinedRooms = this.client.getRooms().filter(room => {
      const membership = room.getMyMembership();
      const isJoined = membership === 'join';
      
      if (!isJoined) {
        console.log('🚪 getRooms: Excluding room with membership:', membership, '→', room.roomId.substring(0, 20) + '...', room.name || 'Unnamed');
      }
      
      return isJoined;
    });
    
    console.log(`getRooms - filtered to ${joinedRooms.length} joined rooms (from ${this.client.getRooms().length} total)`);
    
    this.cachedRooms = joinedRooms.map(room => {
      // Get room avatar from state events
      const avatarEvent = room.currentState.getStateEvents('m.room.avatar', '');
      const avatarUrl = avatarEvent?.getContent()?.url || null;
      
      // Get room type from creation event
      const createEvent = room.currentState.getStateEvents('m.room.create', '');
      const roomType = createEvent?.getContent()?.type;
      
      // Check if this is a space
      const isSpace = this.isSpaceRoom(room);
      
      // Check if this is a public room
      const isPublic = this.isPublicRoom(room);
      
      // Debug logging for spaces
      if (isSpace) {
        console.log('🌌 getRooms: Found Matrix Space:', room.name || 'Unnamed Space', '→', room.roomId.substring(0, 20) + '...');
      }
      
      // Debug logging for avatar URLs
      if (avatarUrl) {
        console.log('🖼️ getRooms: Found avatar for room', room.roomId.substring(0, 20) + '...', '→', avatarUrl);
      }
      
      return {
        roomId: room.roomId,
        name: room.name,
        topic: room.currentState.getStateEvents('m.room.topic', '')?.getContent()?.topic,
        avatarUrl: avatarUrl, // Room avatar (cover photo)
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
        isSpace: isSpace, // Matrix Space detection
        roomType: roomType, // Store the room type
        isPublic: isPublic, // Public/private status
        lastActivity: new Date(room.getLastActiveTimestamp()),
      };
    });

    console.log('getRooms - cached new room data:', this.cachedRooms.length);
    return this.cachedRooms;
  }

  /**
   * Get pending room invitations from Matrix server
   * Returns rooms where the user has been invited but hasn't joined yet
   */
  getPendingInvitedRooms(): Array<{
    roomId: string;
    roomName?: string;
    inviter: string;
    inviterName?: string;
    timestamp: number;
  }> {
    if (!this.client) {
      console.log('📭 getPendingInvitedRooms: Client not initialized');
      return [];
    }

    const allRooms = this.client.getRooms();
    const invitedRooms = allRooms.filter(room => room.getMyMembership() === 'invite');
    
    console.log(`📬 getPendingInvitedRooms: Found ${invitedRooms.length} pending invites from Matrix server`);
    
    const pendingInvites = invitedRooms.map(room => {
      // Get the invite event to find who invited us
      const inviteEvent = room.currentState.getStateEvents('m.room.member', this.config.userId!);
      const inviter = inviteEvent?.getSender() || 'unknown';
      
      // Get inviter information
      const inviterUser = this.client?.getUser(inviter);
      const inviterMember = room.getMember(inviter);
      const inviterName = inviterMember?.name || inviterUser?.displayName || inviter.split(':')[0].substring(1);
      
      // Get timestamp from the invite event
      const timestamp = inviteEvent?.getTs() || Date.now();
      
      return {
        roomId: room.roomId,
        roomName: room.name || undefined,
        inviter,
        inviterName,
        timestamp,
      };
    });

    // Sync with matrixInviteStateService to ensure local storage is up to date
    pendingInvites.forEach(invite => {
      matrixInviteStateService.recordInvite(invite.roomId, invite.inviter, invite.inviterName);
    });

    return pendingInvites;
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

  /**
   * Update room name
   */
  async setRoomName(roomId: string, name: string): Promise<void> {
    if (!this.client) {
      throw new Error('Client not initialized');
    }

    try {
      await this.client.setRoomName(roomId, name);
      
      // Clear rooms cache to force refresh
      this.cachedRooms = null;
      
      this.emit('roomNameUpdated', { roomId, name });
      console.log('✅ Room name updated:', roomId, '→', name);
    } catch (error) {
      console.error('Failed to set room name:', error);
      throw new Error('Failed to update room name');
    }
  }

  /**
   * Update room topic
   */
  async setRoomTopic(roomId: string, topic: string): Promise<void> {
    if (!this.client) {
      throw new Error('Client not initialized');
    }

    try {
      await this.client.setRoomTopic(roomId, topic);
      
      // Clear rooms cache to force refresh
      this.cachedRooms = null;
      
      this.emit('roomTopicUpdated', { roomId, topic });
      console.log('✅ Room topic updated:', roomId, '→', topic);
    } catch (error) {
      console.error('Failed to set room topic:', error);
      throw new Error('Failed to update room topic');
    }
  }

  /**
   * Upload and set room avatar (cover photo)
   */
  async setRoomAvatar(roomId: string, file: File): Promise<string> {
    if (!this.client) {
      throw new Error('Client not initialized');
    }

    try {
      console.log('setRoomAvatar - uploading file for room:', roomId, {
        fileName: file.name,
        fileType: file.type,
        fileSize: file.size,
        fileSizeKB: (file.size / 1024).toFixed(2) + ' KB'
      });
      
      // Upload the file to Matrix media repository
      const uploadResponse = await this.client.uploadContent(file, {
        name: file.name,
        type: file.type,
      });

      console.log('setRoomAvatar - full upload response:', JSON.stringify(uploadResponse, null, 2));
      
      const avatarUrl = uploadResponse.content_uri;
      console.log('setRoomAvatar - extracted MXC URL:', avatarUrl);
      
      // Validate the MXC URL format
      if (!avatarUrl || !avatarUrl.startsWith('mxc://')) {
        throw new Error(`Invalid MXC URL returned from upload: ${avatarUrl}`);
      }

      // Set the avatar URL for the room
      await this.client.sendStateEvent(roomId, 'm.room.avatar', {
        url: avatarUrl,
      }, '');
      
      console.log('setRoomAvatar - room avatar URL set');

      // Clear rooms cache to force refresh
      this.cachedRooms = null;

      // Emit room avatar updated event
      this.emit('roomAvatarUpdated', { roomId, avatarUrl });

      return avatarUrl;
    } catch (error) {
      console.error('Failed to set room avatar:', error);
      throw new Error('Failed to upload and set room avatar');
    }
  }

  /**
   * Remove room avatar
   */
  async removeRoomAvatar(roomId: string): Promise<void> {
    if (!this.client) {
      throw new Error('Client not initialized');
    }

    try {
      await this.client.sendStateEvent(roomId, 'm.room.avatar', {
        url: '',
      }, '');
      
      // Clear rooms cache to force refresh
      this.cachedRooms = null;
      
      this.emit('roomAvatarUpdated', { roomId, avatarUrl: null });
      console.log('✅ Room avatar removed:', roomId);
    } catch (error) {
      console.error('Failed to remove room avatar:', error);
      throw new Error('Failed to remove room avatar');
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
      console.log('🔍 Current user ID:', this.config.userId);
      
      const room = this.client.getRoom(roomId);
      if (!room) {
        console.error('❌ Room not found:', roomId);
        return [];
      }

      // Get timeline events from the room
      const timeline = room.getLiveTimeline();
      const events = timeline.getEvents();
      
      console.log('📜 Found', events.length, 'events in room timeline');
      
      // Debug: Log all event types to see what we're getting
      const eventTypes = events.map(event => event.getType());
      const eventTypeCounts = eventTypes.reduce((acc, type) => {
        acc[type] = (acc[type] || 0) + 1;
        return acc;
      }, {} as Record<string, number>);
      console.log('📜 Event type breakdown:', eventTypeCounts);
      
      // Filter and convert message events (including encrypted ones)
      const messageEvents = events.filter(event => {
        const eventType = event.getType();
        return eventType === 'm.room.message' || eventType === 'm.room.encrypted';
      });
      console.log('📜 Found', messageEvents.length, 'message events (including encrypted)');
      
      // If no message events found, let's examine a few events to see what we have
      if (messageEvents.length === 0 && events.length > 0) {
        console.log('📜 No message events found. Examining first few events:');
        events.slice(0, Math.min(5, events.length)).forEach((event, index) => {
          const content = event.getContent();
          console.log(`📜 Event ${index + 1}:`, {
            type: event.getType(),
            sender: event.getSender(),
            content: content,
            hasBody: !!content.body,
            msgtype: content.msgtype,
            timestamp: new Date(event.getTs()).toISOString()
          });
        });
      }
      
      const messages = messageEvents
        .slice(-limit) // Get the last N messages
        .map((event, index) => {
          const content = event.getContent();
          const sender = event.getSender();
          const isFromSelf = sender === this.config.userId;
          
          console.log(`📜 Processing message ${index + 1}/${Math.min(messageEvents.length, limit)}:`, {
            sender: sender?.substring(0, 30) + '...',
            isFromSelf,
            contentPreview: content.body?.substring(0, 50) + '...',
            timestamp: new Date(event.getTs()).toISOString()
          });
          
          // Get sender information
          const senderMember = room.getMember(sender);
          const senderUser = this.client?.getUser(sender);
          
          const senderInfo = {
            userId: sender,
            displayName: senderMember?.name || senderUser?.displayName || sender.split(':')[0].substring(1),
            avatarUrl: senderMember?.getMxcAvatarUrl() || senderUser?.avatarUrl || null,
          };

          // Handle encrypted messages
          let actualContent = '';
          let messageType: 'user' | 'assistant' | 'system' = 'user';
          let sessionData = null;
          
          if (event.getType() === 'm.room.encrypted') {
            // Check if crypto is available
            if (!this.client?.crypto) {
              actualContent = '🔒 [Encrypted message - encryption not supported by this client]';
              console.log('📜 🔒 Encrypted message found but crypto not initialized');
            } else {
              // Try to get the decrypted content
              try {
                // Check if getClearEvent method exists
                if (typeof event.getClearEvent === 'function') {
                  const clearEvent = event.getClearEvent();
                  
                  if (clearEvent && clearEvent.content && clearEvent.content.body) {
                    actualContent = clearEvent.content.body;
                    console.log('📜 🔓 Successfully decrypted message:', actualContent.substring(0, 50) + '...');
                  } else if (typeof event.isDecryptionFailure === 'function' && event.isDecryptionFailure()) {
                    // Specific decryption failure
                    const failureReason = event.decryptionFailureReason || 'Unknown encryption error';
                    actualContent = `🔒 [Unable to decrypt: ${failureReason}]`;
                    console.log('📜 🔒 Decryption failure:', failureReason);
                  } else {
                    // Event not yet decrypted but no failure
                    actualContent = '🔄 [Decrypting message...]';
                    console.log('📜 🔄 Message not yet decrypted, may decrypt later');
                  }
                } else {
                  // getClearEvent method not available
                  actualContent = '🔒 [Encrypted message - decryption methods not available]';
                  console.log('📜 🔒 getClearEvent method not available on event');
                }
              } catch (decryptError) {
                console.warn('📜 ❌ Failed to decrypt message:', decryptError);
                actualContent = `🔒 [Encryption error: ${decryptError.message || 'Unknown'}]`;
              }
            }
          } else {
            // Regular unencrypted message
            actualContent = content.body || '';
          }

          // Check if this is a session message that needs parsing
          if (actualContent.includes('goose-session-message:')) {
            try {
              const sessionJson = actualContent.substring(actualContent.indexOf('goose-session-message:') + 'goose-session-message:'.length);
              sessionData = JSON.parse(sessionJson);
              actualContent = sessionData.content || actualContent;
              
              // Better role detection for session messages
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
          // Check if this is a regular Goose/AI message (but NOT from self unless it has explicit Goose markers)
          else if (content['goose.message.type'] || content['goose.type']) {
            // Explicit Goose message markers - always treat as assistant
            messageType = 'assistant';
            console.log('📜 Explicit Goose message detected:', content['goose.message.type'] || content['goose.type']);
          }
          // Check if message is from a known Goose instance (not self)
          else if (!isFromSelf && this.isGooseInstance(sender, senderInfo.displayName)) {
            messageType = 'assistant';
            console.log('📜 Message from Goose instance detected:', sender);
          }
          // Check if message content looks like a Goose message (not self)
          else if (!isFromSelf && this.looksLikeGooseMessage(actualContent)) {
            messageType = 'assistant';
            console.log('📜 Message content looks like Goose message');
          }
          // System messages
          else if (content.msgtype === 'm.notice' || sender.includes('bot')) {
            messageType = 'system';
            console.log('📜 System message detected');
          }
          // CRITICAL FIX: Messages from self that don't have explicit Goose markers should be 'user'
          // This ensures user's own messages are properly categorized as 'user' type
          else {
            // Default to 'user' for all other messages, including messages from self
            messageType = 'user';
            
            // Log for debugging
            if (isFromSelf) {
              console.log('📜 ✅ Message from self categorized as user:', actualContent.substring(0, 50) + '...');
            } else {
              console.log('📜 Message from other user categorized as user:', actualContent.substring(0, 50) + '...');
            }
          }

          const result = {
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
          
          console.log(`📜 Final result for message ${index + 1}:`, {
            type: result.type,
            isFromSelf: result.isFromSelf,
            sender: result.sender?.substring(0, 30) + '...',
            content: result.content?.substring(0, 50) + '...'
          });
          
          return result;
        });

      console.log('📜 Processed', messages.length, 'messages from room history');
      console.log('📜 Message type breakdown:', {
        user: messages.filter(m => m.type === 'user').length,
        assistant: messages.filter(m => m.type === 'assistant').length,
        system: messages.filter(m => m.type === 'system').length,
        fromSelf: messages.filter(m => m.isFromSelf).length,
        fromOthers: messages.filter(m => !m.isFromSelf).length,
      });
      
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
