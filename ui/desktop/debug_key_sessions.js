// Debug script to analyze key sessions and sharing
console.log('🔑 KEY SESSIONS DIAGNOSTIC START');

const matrixService = window.matrixService;
if (!matrixService || !matrixService.client) {
  console.log('❌ MatrixService or client not available');
  console.log('🔑 KEY SESSIONS DIAGNOSTIC END');
} else {
  const client = matrixService.client;
  
  console.log('🔐 Crypto available:', !!client.crypto);
  console.log('🆔 User ID:', client.getUserId());
  console.log('🔧 Device ID:', client.getDeviceId());
  
  if (client.crypto) {
    try {
      // Check our own devices
      const userId = client.getUserId();
      const ownDevices = await client.crypto.getStoredDevicesForUser(userId);
      console.log(`👤 Own devices (${ownDevices.length}):`);
      
      for (const device of ownDevices) {
        console.log(`  📱 ${device.deviceId.substring(0, 12)}... - verified: ${device.isVerified()}, blocked: ${device.isBlocked()}, known: ${device.isKnown()}`);
      }
      
      // Check encrypted rooms
      const rooms = client.getRooms();
      const encryptedRooms = rooms.filter(room => 
        room.hasEncryptionStateEvent && room.hasEncryptionStateEvent()
      );
      
      console.log(`🏠 Encrypted rooms: ${encryptedRooms.length}`);
      
      // Focus on the problematic room from the error
      const problemRoom = client.getRoom('!ezLKoBcnREgTkeLhWP:tchncs.de');
      if (problemRoom) {
        console.log(`🔍 Problem room: ${problemRoom.name || 'Unnamed'}`);
        console.log(`🔐 Room encrypted: ${problemRoom.hasEncryptionStateEvent()}`);
        
        // Check recent timeline events
        const timeline = problemRoom.getLiveTimeline();
        const events = timeline.getEvents();
        const recentEvents = events.slice(-5); // Last 5 events
        
        console.log(`📜 Recent events (${recentEvents.length}):`);
        for (const event of recentEvents) {
          const eventId = event.getId();
          const sender = event.getSender();
          const type = event.getType();
          const isEncrypted = type === 'm.room.encrypted';
          
          console.log(`  📝 ${eventId?.substring(0, 20)}... - ${sender} - ${type} - encrypted: ${isEncrypted}`);
          
          if (isEncrypted) {
            try {
              const clearEvent = event.getClearEvent();
              console.log(`    ✅ Decrypted: ${!!clearEvent}`);
              if (clearEvent) {
                console.log(`    📄 Content type: ${clearEvent.type}`);
              }
            } catch (decryptError) {
              console.log(`    ❌ Decryption failed: ${decryptError.message}`);
            }
          }
        }
        
        // Check room members and their devices
        const members = problemRoom.getMembers();
        console.log(`👥 Room members: ${members.length}`);
        
        for (const member of members.slice(0, 3)) { // First 3 members
          try {
            const memberDevices = await client.crypto.getStoredDevicesForUser(member.userId);
            console.log(`  👤 ${member.userId} - ${memberDevices.length} devices`);
            
            for (const device of memberDevices.slice(0, 2)) { // First 2 devices per member
              console.log(`    📱 ${device.deviceId.substring(0, 12)}... - verified: ${device.isVerified()}, blocked: ${device.isBlocked()}`);
            }
          } catch (memberError) {
            console.log(`  ❌ Failed to get devices for ${member.userId}: ${memberError.message}`);
          }
        }
      }
      
      // Check if we can access crypto store information
      if (client.crypto.olmDevice) {
        console.log('🔐 Olm device available:', !!client.crypto.olmDevice);
      }
      
      console.log('🔑 KEY SESSIONS DIAGNOSTIC COMPLETE');
      
    } catch (error) {
      console.error('❌ Diagnostic failed:', error);
    }
  }
}
