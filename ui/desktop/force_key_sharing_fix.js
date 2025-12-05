// Aggressive key sharing fix for MEGOLM_UNKNOWN_INBOUND_SESSION_ID errors
console.log('🔑 AGGRESSIVE KEY SHARING FIX START');

const matrixService = window.matrixService;
if (!matrixService || !matrixService.client) {
  console.log('❌ MatrixService or client not available');
} else {
  const client = matrixService.client;
  
  if (!client.crypto) {
    console.log('❌ Crypto not available');
  } else {
    try {
      console.log('🔑 Starting aggressive key sharing fix...');
      
      const userId = client.getUserId();
      console.log('🆔 User ID:', userId);
      
      // Step 1: Force device verification for ALL our own devices
      console.log('🔐 Step 1: Verifying all own devices...');
      const ownDevices = await client.crypto.getStoredDevicesForUser(userId);
      console.log(`📱 Found ${ownDevices.length} own devices`);
      
      for (const device of ownDevices) {
        try {
          if (!device.isVerified()) {
            await client.crypto.setDeviceVerification(userId, device.deviceId, true);
            console.log(`✅ Verified device: ${device.deviceId.substring(0, 12)}...`);
          }
          if (device.isBlocked()) {
            await client.crypto.setDeviceBlocked(userId, device.deviceId, false);
            console.log(`✅ Unblocked device: ${device.deviceId.substring(0, 12)}...`);
          }
        } catch (deviceError) {
          console.warn(`⚠️ Failed to fix device ${device.deviceId}:`, deviceError.message);
        }
      }
      
      // Step 2: Force Olm sessions with our own devices
      console.log('🔑 Step 2: Establishing Olm sessions with own devices...');
      const verifiedOwnDevices = ownDevices.filter(device => device.isVerified() && !device.isBlocked());
      console.log(`🔑 Using ${verifiedOwnDevices.length} verified own devices`);
      
      if (verifiedOwnDevices.length > 0) {
        const deviceMap = { [userId]: verifiedOwnDevices };
        
        if (typeof client.crypto.ensureOlmSessionsForDevices === 'function') {
          await client.crypto.ensureOlmSessionsForDevices(deviceMap);
          console.log('✅ Established Olm sessions with own devices');
        } else {
          console.warn('⚠️ ensureOlmSessionsForDevices not available');
        }
      }
      
      // Step 3: Focus on the problematic room
      console.log('🔑 Step 3: Fixing key sharing for problematic room...');
      const problemRoomId = '!ezLKoBcnREgTkeLhWP:tchncs.de';
      const problemRoom = client.getRoom(problemRoomId);
      
      if (problemRoom) {
        console.log(`🏠 Processing room: ${problemRoom.name || 'Unnamed'}`);
        
        // Get all room members and their devices
        const members = problemRoom.getMembers();
        console.log(`👥 Room has ${members.length} members`);
        
        const memberDevices = {};
        
        for (const member of members) {
          try {
            const memberDeviceList = await client.crypto.getStoredDevicesForUser(member.userId);
            console.log(`👤 ${member.userId}: ${memberDeviceList.length} devices`);
            
            // For our own user, use all verified devices
            if (member.userId === userId) {
              const validDevices = memberDeviceList.filter(device => device.isVerified() && !device.isBlocked());
              if (validDevices.length > 0) {
                memberDevices[member.userId] = validDevices;
                console.log(`✅ Added ${validDevices.length} own devices for key sharing`);
              }
            } else {
              // For other users, use verified or known devices
              const validDevices = memberDeviceList.filter(device => 
                device.isVerified() || (!device.isBlocked() && device.isKnown())
              );
              if (validDevices.length > 0) {
                memberDevices[member.userId] = validDevices;
                console.log(`✅ Added ${validDevices.length} devices for ${member.userId}`);
              }
            }
          } catch (memberError) {
            console.warn(`⚠️ Failed to get devices for ${member.userId}:`, memberError.message);
          }
        }
        
        // Ensure Olm sessions for all room members
        if (Object.keys(memberDevices).length > 0) {
          console.log(`🔑 Establishing Olm sessions for ${Object.keys(memberDevices).length} users...`);
          
          if (typeof client.crypto.ensureOlmSessionsForDevices === 'function') {
            await client.crypto.ensureOlmSessionsForDevices(memberDevices);
            console.log('✅ Established Olm sessions for room members');
          }
        }
        
        // Step 4: Force discard existing sessions and request new keys
        console.log('🔑 Step 4: Forcing session refresh...');
        
        try {
          // Try to force discard sessions for the room
          if (typeof client.crypto.forceDiscardSession === 'function') {
            await client.crypto.forceDiscardSession(problemRoomId);
            console.log('✅ Discarded existing sessions for room');
          }
        } catch (discardError) {
          console.warn('⚠️ Could not discard sessions:', discardError.message);
        }
        
        // Step 5: Cancel and resend all key requests
        console.log('🔑 Step 5: Refreshing key requests...');
        
        try {
          if (typeof client.crypto.cancelAndResendAllOutgoingKeyRequests === 'function') {
            await client.crypto.cancelAndResendAllOutgoingKeyRequests();
            console.log('✅ Cancelled and resent all key requests');
          }
        } catch (keyRequestError) {
          console.warn('⚠️ Failed to resend key requests:', keyRequestError.message);
        }
        
        // Step 6: Try to manually request keys for recent encrypted events
        console.log('🔑 Step 6: Requesting keys for recent encrypted events...');
        
        const timeline = problemRoom.getLiveTimeline();
        const events = timeline.getEvents();
        const encryptedEvents = events.filter(event => event.getType() === 'm.room.encrypted');
        const recentEncryptedEvents = encryptedEvents.slice(-10); // Last 10 encrypted events
        
        console.log(`🔍 Found ${recentEncryptedEvents.length} recent encrypted events to process`);
        
        for (const event of recentEncryptedEvents) {
          try {
            // Check if the event is already decrypted
            const clearEvent = event.getClearEvent();
            if (!clearEvent) {
              console.log(`🔑 Requesting keys for event: ${event.getId()?.substring(0, 20)}...`);
              
              // Try to manually request decryption
              if (typeof event.attemptDecryption === 'function') {
                await event.attemptDecryption(client.crypto);
                console.log(`✅ Attempted decryption for event`);
              }
            } else {
              console.log(`✅ Event already decrypted: ${event.getId()?.substring(0, 20)}...`);
            }
          } catch (eventError) {
            console.warn(`⚠️ Failed to process event ${event.getId()}:`, eventError.message);
          }
        }
        
      } else {
        console.warn('⚠️ Problem room not found');
      }
      
      console.log('🔑 ✅ AGGRESSIVE KEY SHARING FIX COMPLETE');
      console.log('🔑 Please try sending a new message or refreshing the chat to see if decryption works');
      
    } catch (error) {
      console.error('🔑 ❌ Aggressive fix failed:', error);
    }
  }
}
