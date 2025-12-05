// Debug script to test real-time encrypted message handling
console.log('🔍 DEBUGGING REAL-TIME ENCRYPTED MESSAGE HANDLING...');

const matrixService = window.matrixService;
if (!matrixService) {
  console.error('❌ MatrixService not found on window');
  throw new Error('MatrixService not available');
}

const client = matrixService.client;
if (!client) {
  console.error('❌ Matrix client not initialized');
  throw new Error('Matrix client not available');
}

console.log('✅ MatrixService and client available');

// Check crypto status
console.log('🔐 Crypto Status:');
console.log('  - Crypto module:', !!client.crypto);
console.log('  - Crypto ready:', client.isCryptoEnabled?.() || 'unknown');
console.log('  - Device ID:', client.getDeviceId());
console.log('  - User ID:', client.getUserId());

// Get encrypted rooms
const rooms = client.getRooms();
const encryptedRooms = rooms.filter(room => 
  room.hasEncryptionStateEvent && 
  room.hasEncryptionStateEvent() && 
  room.getMyMembership() === 'join'
);

console.log(`🔐 Found ${encryptedRooms.length} encrypted rooms:`);
encryptedRooms.forEach(room => {
  console.log(`  - ${room.name || 'Unnamed'} (${room.roomId.substring(0, 20)}...)`);
});

if (encryptedRooms.length === 0) {
  console.log('⚠️ No encrypted rooms found to test with');
  throw new Error('No encrypted rooms available');
}

// Test room (use first encrypted room)
const testRoom = encryptedRooms[0];
console.log(`🧪 Using test room: ${testRoom.name || 'Unnamed'} (${testRoom.roomId.substring(0, 20)}...)`);

// Monitor real-time events
let eventCount = 0;
const originalEmit = matrixService.emit;

console.log('📡 Setting up real-time event monitoring...');

// Monitor MatrixService events
matrixService.emit = function(eventType, ...args) {
  if (eventType === 'message') {
    eventCount++;
    const messageData = args[0];
    console.log(`📨 REAL-TIME MESSAGE #${eventCount}:`, {
      roomId: messageData.roomId?.substring(0, 20) + '...',
      sender: messageData.sender?.substring(0, 20) + '...',
      content: messageData.content?.substring(0, 50) + '...',
      isFromSelf: messageData.isFromSelf,
      timestamp: messageData.timestamp
    });
  }
  return originalEmit.apply(this, [eventType, ...args]);
};

// Monitor Matrix client events directly
const timelineHandler = (event, room, toStartOfTimeline) => {
  if (room.roomId === testRoom.roomId && !toStartOfTimeline) {
    const eventType = event.getType();
    console.log(`🔄 TIMELINE EVENT:`, {
      eventType,
      sender: event.getSender()?.substring(0, 20) + '...',
      roomId: room.roomId.substring(0, 20) + '...',
      roomName: room.name,
      isEncrypted: eventType === 'm.room.encrypted'
    });

    if (eventType === 'm.room.encrypted') {
      console.log('🔐 Encrypted event detected, checking decryption...');
      
      // Check if it can be decrypted
      setTimeout(() => {
        try {
          if (typeof event.getClearEvent === 'function') {
            const clearEvent = event.getClearEvent();
            if (clearEvent && clearEvent.content && clearEvent.content.body) {
              console.log('🔓 ✅ Successfully decrypted:', clearEvent.content.body.substring(0, 50) + '...');
            } else if (typeof event.isDecryptionFailure === 'function' && event.isDecryptionFailure()) {
              const reason = event.decryptionFailureReason || 'Unknown';
              console.log('🔐 ❌ Decryption failed:', reason);
            } else {
              console.log('🔄 Still decrypting...');
            }
          }
        } catch (error) {
          console.log('🔐 ❌ Decryption error:', error.message);
        }
      }, 1000);
    }
  }
};

const decryptionHandler = (event) => {
  if (event.getRoomId() === testRoom.roomId) {
    console.log('🔓 DECRYPTION EVENT:', {
      eventId: event.getId()?.substring(0, 20) + '...',
      roomId: event.getRoomId()?.substring(0, 20) + '...',
      sender: event.getSender()?.substring(0, 20) + '...',
      type: event.getType()
    });

    // Check the decrypted content
    try {
      if (typeof event.getClearEvent === 'function') {
        const clearEvent = event.getClearEvent();
        if (clearEvent && clearEvent.content && clearEvent.content.body) {
          console.log('🔓 ✅ Decrypted content:', clearEvent.content.body.substring(0, 50) + '...');
        }
      }
    } catch (error) {
      console.log('🔐 ❌ Error reading decrypted content:', error.message);
    }
  }
};

// Add event listeners
client.on('Room.timeline', timelineHandler);
client.on('Event.decrypted', decryptionHandler);

console.log('✅ Real-time monitoring setup complete!');
console.log('📝 Now send an encrypted message in the test room to see real-time handling');
console.log(`📍 Test room: ${testRoom.name || 'Unnamed'} (${testRoom.roomId})`);

// Cleanup function
const cleanup = () => {
  console.log('🧹 Cleaning up event listeners...');
  matrixService.emit = originalEmit;
  client.off('Room.timeline', timelineHandler);
  client.off('Event.decrypted', decryptionHandler);
  console.log('✅ Cleanup complete');
};

// Auto-cleanup after 5 minutes
setTimeout(() => {
  console.log('⏰ Auto-cleanup after 5 minutes');
  cleanup();
}, 5 * 60 * 1000);

// Return cleanup function for manual use
window.cleanupRealtimeDebug = cleanup;

console.log('💡 Use window.cleanupRealtimeDebug() to manually stop monitoring');
