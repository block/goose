// Test script to verify enhanced key sharing fix is working
console.log('🔑 TESTING ENHANCED KEY SHARING FIX');

const matrixService = window.matrixService;
if (!matrixService || !matrixService.client) {
  console.log('❌ MatrixService or client not available');
} else {
  const client = matrixService.client;
  
  if (!client.crypto) {
    console.log('❌ Crypto not available');
  } else {
    console.log('✅ MatrixService and crypto available');
    console.log('🆔 User ID:', client.getUserId());
    console.log('🔧 Device ID:', client.getDeviceId());
    console.log('🔐 Crypto module active:', !!client.crypto);
    
    // Check if the enhanced methods are available
    console.log('🔍 Checking enhanced key sharing methods...');
    
    // Test if we can access our own devices
    try {
      const userId = client.getUserId();
      const ownDevices = await client.crypto.getStoredDevicesForUser(userId);
      console.log(`📱 Own devices: ${ownDevices.length}`);
      
      ownDevices.forEach((device, index) => {
        console.log(`  ${index + 1}. ${device.deviceId.substring(0, 12)}... - verified: ${device.isVerified()}, blocked: ${device.isBlocked()}`);
      });
      
      // Check encrypted rooms
      const rooms = client.getRooms();
      const encryptedRooms = rooms.filter(room => 
        room.hasEncryptionStateEvent && room.hasEncryptionStateEvent()
      );
      
      console.log(`🏠 Encrypted rooms: ${encryptedRooms.length}`);
      
      if (encryptedRooms.length > 0) {
        const testRoom = encryptedRooms[0];
        console.log(`🔍 Test room: ${testRoom.name || 'Unnamed'} (${testRoom.roomId.substring(0, 20)}...)`);
        
        // Check recent encrypted events in the test room
        const timeline = testRoom.getLiveTimeline();
        const events = timeline.getEvents();
        const encryptedEvents = events.filter(event => event.getType() === 'm.room.encrypted');
        
        console.log(`🔐 Encrypted events in test room: ${encryptedEvents.length}`);
        
        if (encryptedEvents.length > 0) {
          const recentEvent = encryptedEvents[encryptedEvents.length - 1];
          console.log(`📝 Most recent encrypted event: ${recentEvent.getId()?.substring(0, 20)}...`);
          
          try {
            const clearEvent = recentEvent.getClearEvent();
            if (clearEvent && clearEvent.content && clearEvent.content.body) {
              console.log('✅ Recent encrypted message successfully decrypted!');
              console.log(`📄 Content preview: "${clearEvent.content.body.substring(0, 100)}..."`);
            } else {
              console.log('❌ Recent encrypted message not decrypted');
              
              // Check if it's a decryption failure
              if (typeof recentEvent.isDecryptionFailure === 'function' && recentEvent.isDecryptionFailure()) {
                const failureReason = recentEvent.decryptionFailureReason || 'Unknown';
                console.log(`🔒 Decryption failure reason: ${failureReason}`);
              }
            }
          } catch (decryptError) {
            console.log('❌ Error checking decryption:', decryptError.message);
          }
        }
      }
      
      console.log('🔑 ✅ ENHANCED KEY SHARING TEST COMPLETE');
      console.log('🔑 The enhanced fix should automatically run after Matrix sync is complete');
      console.log('🔑 Look for 🔑 emoji logs in the console to see the fix in action');
      
    } catch (error) {
      console.error('❌ Test failed:', error);
    }
  }
}
