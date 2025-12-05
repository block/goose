// Encryption Diagnosis Script
// Copy and paste this into your browser console

const roomId = "!aiOnzvuIzOOXnluWRv:tchncs.de";

console.log("🔐 DIAGNOSING ENCRYPTION STATE");

const matrixService = window.matrixService;
if (!matrixService || !matrixService.client) {
  console.error("❌ matrixService or client not available");
} else {
  const client = matrixService.client;
  const room = client.getRoom(roomId);
  
  if (!room) {
    console.error("❌ Room not found");
  } else {
    console.log("🔍 Room found:", room.name);
    console.log("🔍 Room encryption state:", room.hasEncryptionStateEvent());
    
    // Check if crypto is initialized
    console.log("🔐 Crypto initialized:", !!client.crypto);
    
    if (client.crypto) {
      console.log("🔐 Crypto module available");
      
      // Check device verification
      console.log("🔐 Current device ID:", client.getDeviceId());
      console.log("🔐 Current user ID:", client.getUserId());
      
      // Get room encryption info
      const encryptionEvent = room.currentState.getStateEvents('m.room.encryption', '');
      if (encryptionEvent) {
        console.log("🔐 Room encryption algorithm:", encryptionEvent.getContent().algorithm);
      }
      
      // Check encrypted events and their decryption status
      const timeline = room.getLiveTimeline();
      const events = timeline.getEvents();
      const encryptedEvents = events.filter(e => e.getType() === 'm.room.encrypted');
      
      console.log(`🔐 Found ${encryptedEvents.length} encrypted events`);
      
      encryptedEvents.forEach((event, index) => {
        console.log(`🔐 Encrypted Event ${index + 1}:`);
        console.log(`  Event ID: ${event.getId()}`);
        console.log(`  Sender: ${event.getSender()}`);
        console.log(`  Timestamp: ${new Date(event.getTs()).toISOString()}`);
        
        // Check decryption status
        const clearEvent = event.getClearEvent();
        console.log(`  Has clear event: ${!!clearEvent}`);
        
        if (clearEvent) {
          console.log(`  Clear event type: ${clearEvent.type}`);
          console.log(`  Clear content: ${JSON.stringify(clearEvent.content).substring(0, 100)}...`);
        }
        
        // Check if event is decrypted
        console.log(`  Is decrypted: ${event.isDecryptionFailure() === false}`);
        console.log(`  Decryption failure: ${event.isDecryptionFailure()}`);
        
        if (event.isDecryptionFailure()) {
          console.log(`  Decryption error: ${event.decryptionFailureReason || 'Unknown'}`);
        }
        
        console.log("  ---");
      });
      
    } else {
      console.log("❌ Crypto module not available - encryption not supported");
    }
    
    // Provide solutions
    console.log("\n🔧 POTENTIAL SOLUTIONS:");
    console.log("1. Key backup: The messages might be from before this device was verified");
    console.log("2. Cross-signing: Verify this device with another device that has the keys");
    console.log("3. Key request: Request keys from other devices");
    console.log("4. Fresh start: Send a new message to establish new encryption session");
    
    // Test sending a new message
    console.log("\n🧪 TESTING: Try sending a new message to establish encryption");
    console.log("You can test by sending a message in the Matrix room, then reopening it");
  }
}

// Function to attempt key requests
window.requestRoomKeys = async function() {
  console.log("🔑 Attempting to request room keys...");
  
  const client = window.matrixService?.client;
  if (!client || !client.crypto) {
    console.error("❌ Crypto not available");
    return;
  }
  
  try {
    // Request keys for undecryptable events
    const room = client.getRoom(roomId);
    if (room) {
      const timeline = room.getLiveTimeline();
      const events = timeline.getEvents();
      const encryptedEvents = events.filter(e => e.getType() === 'm.room.encrypted' && e.isDecryptionFailure());
      
      console.log(`🔑 Found ${encryptedEvents.length} events that need keys`);
      
      for (const event of encryptedEvents) {
        try {
          // This might trigger key requests
          await client.crypto.requestRoomKey(event.getWireContent(), event.getRoomId());
          console.log(`🔑 Requested keys for event ${event.getId()}`);
        } catch (error) {
          console.warn(`⚠️ Failed to request keys for event ${event.getId()}:`, error);
        }
      }
      
      console.log("🔑 Key requests sent. Wait a moment and try refreshing the room.");
    }
  } catch (error) {
    console.error("❌ Failed to request keys:", error);
  }
};

// Function to force refresh room timeline
window.refreshRoomTimeline = function() {
  console.log("🔄 Refreshing room timeline...");
  
  const client = window.matrixService?.client;
  if (!client) {
    console.error("❌ Client not available");
    return;
  }
  
  const room = client.getRoom(roomId);
  if (room) {
    // Force timeline refresh
    room.resetLiveTimeline();
    console.log("✅ Timeline reset. Try opening the room again.");
  }
};

console.log("\nAvailable functions:");
console.log("- requestRoomKeys() - Request encryption keys for undecryptable messages");
console.log("- refreshRoomTimeline() - Force refresh the room timeline");
