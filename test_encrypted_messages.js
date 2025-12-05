// Test Encrypted Messages Script
// Copy and paste this into your browser console to test the fix

const roomId = "!aiOnzvuIzOOXnluWRv:tchncs.de";

console.log("🧪 TESTING ENCRYPTED MESSAGE HANDLING");

const matrixService = window.matrixService;
if (!matrixService || !matrixService.client) {
  console.error("❌ matrixService or client not available");
} else {
  console.log("✅ MatrixService available");
  
  // Test the updated getRoomHistory method
  console.log("🔍 Testing getRoomHistory method...");
  
  matrixService.getRoomHistory(roomId, 10)
    .then(history => {
      console.log("📜 getRoomHistory result:");
      console.log(`  Total messages found: ${history.length}`);
      
      if (history.length > 0) {
        console.log("📜 First few messages:");
        history.slice(0, 3).forEach((msg, index) => {
          console.log(`  Message ${index + 1}:`);
          console.log(`    Type: ${msg.type}`);
          console.log(`    Sender: ${msg.senderInfo.displayName || msg.sender}`);
          console.log(`    Content: ${msg.content.substring(0, 100)}${msg.content.length > 100 ? '...' : ''}`);
          console.log(`    Is encrypted: ${msg.metadata.eventType === 'm.room.encrypted'}`);
          console.log(`    Timestamp: ${msg.timestamp.toISOString()}`);
          console.log("    ---");
        });
        
        console.log("✅ SUCCESS: Messages found and processed!");
        
        // Test message type breakdown
        const breakdown = {
          user: history.filter(m => m.type === 'user').length,
          assistant: history.filter(m => m.type === 'assistant').length,
          system: history.filter(m => m.type === 'system').length,
          encrypted: history.filter(m => m.metadata.eventType === 'm.room.encrypted').length,
          decrypted: history.filter(m => m.metadata.eventType === 'm.room.encrypted' && !m.content.includes('[Encrypted message')).length
        };
        
        console.log("📊 Message breakdown:", breakdown);
        
        if (breakdown.encrypted > 0 && breakdown.decrypted > 0) {
          console.log("🔓 SUCCESS: Encrypted messages are being decrypted!");
        } else if (breakdown.encrypted > 0 && breakdown.decrypted === 0) {
          console.log("🔒 WARNING: Encrypted messages found but not decrypted");
        }
        
      } else {
        console.log("❌ No messages found - there might still be an issue");
      }
    })
    .catch(error => {
      console.error("❌ getRoomHistory failed:", error);
    });
  
  // Also test the room inspection to see current state
  console.log("🔍 Current room state:");
  const room = matrixService.client.getRoom(roomId);
  if (room) {
    const timeline = room.getLiveTimeline();
    const events = timeline.getEvents();
    const encryptedEvents = events.filter(e => e.getType() === 'm.room.encrypted');
    
    console.log(`  Total events: ${events.length}`);
    console.log(`  Encrypted events: ${encryptedEvents.length}`);
    
    if (encryptedEvents.length > 0) {
      console.log("🔍 Testing decryption on first encrypted event:");
      const firstEncrypted = encryptedEvents[0];
      try {
        const clearEvent = firstEncrypted.getClearEvent();
        if (clearEvent && clearEvent.content && clearEvent.content.body) {
          console.log("🔓 Decryption successful:", clearEvent.content.body.substring(0, 100) + "...");
        } else {
          console.log("🔒 Decryption returned empty/null result");
          console.log("Clear event:", clearEvent);
        }
      } catch (error) {
        console.log("❌ Decryption failed:", error);
      }
    }
  }
}
