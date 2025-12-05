// Detailed crypto debugging script
console.log('🔐 DETAILED CRYPTO DEBUGGING...');

// Check if Matrix service is available
if (typeof matrixService === 'undefined') {
  console.error('❌ matrixService not available');
} else {
  console.log('✅ matrixService available');
  
  const client = matrixService.client;
  if (!client) {
    console.error('❌ Matrix client not initialized');
  } else {
    console.log('✅ Matrix client initialized');
    
    // Check crypto module
    console.log('🔐 Crypto module check:');
    console.log('  - client.crypto exists:', !!client.crypto);
    console.log('  - client.isCryptoEnabled():', typeof client.isCryptoEnabled === 'function' ? client.isCryptoEnabled() : 'method not available');
    console.log('  - client.isRoomEncrypted exists:', typeof client.isRoomEncrypted === 'function');
    
    // Check crypto store
    if (client.crypto) {
      console.log('🔐 Crypto details:');
      console.log('  - crypto.store exists:', !!client.crypto.store);
      console.log('  - crypto.olmDevice exists:', !!client.crypto.olmDevice);
      console.log('  - crypto.deviceList exists:', !!client.crypto.deviceList);
    }
    
    // Check initCrypto method
    console.log('🔐 initCrypto method check:');
    console.log('  - initCrypto exists:', typeof client.initCrypto === 'function');
    console.log('  - initCrypto called:', client._cryptoInitialized || 'unknown');
    
    // Check Matrix SDK version and crypto capabilities
    console.log('🔐 SDK information:');
    const sdk = window.matrixcs || window.matrix || {};
    console.log('  - SDK available:', !!sdk);
    console.log('  - SDK.createClient exists:', typeof sdk.createClient === 'function');
    console.log('  - SDK.MemoryCryptoStore exists:', typeof sdk.MemoryCryptoStore === 'function');
    console.log('  - SDK.verificationMethods exists:', !!sdk.verificationMethods);
    if (sdk.verificationMethods) {
      console.log('  - SDK.verificationMethods.SAS exists:', !!sdk.verificationMethods.SAS);
    }
    
    // Test encryption on a known encrypted room
    const rooms = client.getRooms();
    const encryptedRooms = rooms.filter(room => {
      try {
        return room.hasEncryptionStateEvent && room.hasEncryptionStateEvent();
      } catch (e) {
        return false;
      }
    });
    
    console.log('🔐 Encrypted rooms found:', encryptedRooms.length);
    
    if (encryptedRooms.length > 0) {
      const testRoom = encryptedRooms[0];
      console.log('🔐 Testing encryption on room:', testRoom.roomId.substring(0, 20) + '...');
      console.log('  - Room name:', testRoom.name);
      console.log('  - Has encryption state event:', testRoom.hasEncryptionStateEvent());
      
      if (typeof client.isRoomEncrypted === 'function') {
        try {
          const isEncrypted = client.isRoomEncrypted(testRoom.roomId);
          console.log('  - client.isRoomEncrypted():', isEncrypted);
        } catch (e) {
          console.log('  - client.isRoomEncrypted() error:', e.message);
        }
      }
      
      // Try to send a test encrypted message (dry run)
      console.log('🔐 Testing message encryption capability...');
      try {
        if (client.crypto && typeof client.crypto.encryptMessage === 'function') {
          console.log('  - crypto.encryptMessage method available');
        } else {
          console.log('  - crypto.encryptMessage method NOT available');
        }
      } catch (e) {
        console.log('  - Error checking encryption methods:', e.message);
      }
    }
    
    // Check device keys
    if (client.crypto && client.crypto.deviceList) {
      try {
        const myUserId = client.getUserId();
        const myDevices = client.crypto.deviceList.getDeviceList(myUserId);
        console.log('🔐 My devices:', myDevices ? myDevices.length : 'none');
      } catch (e) {
        console.log('🔐 Error getting device list:', e.message);
      }
    }
    
    // Check if we can create encrypted content
    console.log('🔐 Testing encrypted content creation...');
    try {
      const testContent = {
        msgtype: 'm.text',
        body: 'Test message'
      };
      
      // This is what happens internally when sending encrypted messages
      if (client.crypto) {
        console.log('  - Crypto module available for content encryption');
        // Don't actually encrypt, just check if the methods exist
        if (typeof client.crypto.encryptMessage === 'function') {
          console.log('  - encryptMessage method exists');
        }
        if (typeof client.crypto.encryptEvent === 'function') {
          console.log('  - encryptEvent method exists');
        }
      }
    } catch (e) {
      console.log('  - Error testing content encryption:', e.message);
    }
  }
}

console.log('🔐 Detailed crypto debugging complete');
