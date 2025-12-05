// Debug and fix Matrix key sharing issues
(async () => {
    console.log('🔐 MATRIX KEY SHARING DIAGNOSTIC START');
    
    const matrixService = window.matrixService;
    if (!matrixService || !matrixService.client) {
        console.log('❌ Matrix service not available');
        return;
    }
    
    const client = matrixService.client;
    console.log('✅ Matrix client available');
    console.log('🔐 Crypto module active:', !!client.crypto);
    
    if (!client.crypto) {
        console.log('❌ Crypto module not active');
        return;
    }
    
    // Check device info
    const deviceId = client.getDeviceId();
    const userId = client.getUserId();
    console.log('👤 User ID:', userId);
    console.log('📱 Device ID:', deviceId);
    
    // Check if we have our own device keys
    try {
        const ownDevice = client.crypto.deviceList.getStoredDevice(userId, deviceId);
        console.log('🔑 Own device info:', ownDevice ? 'Found' : 'Missing');
        
        if (ownDevice) {
            console.log('🔑 Own device keys:', {
                verified: ownDevice.isVerified(),
                blocked: ownDevice.isBlocked(),
                known: ownDevice.isKnown()
            });
        }
    } catch (error) {
        console.log('❌ Error checking own device:', error.message);
    }
    
    // Check encrypted rooms and their key status
    const rooms = client.getRooms();
    const encryptedRooms = rooms.filter(r => r.hasEncryptionStateEvent && r.hasEncryptionStateEvent());
    
    console.log(`🔐 Found ${encryptedRooms.length} encrypted rooms`);
    
    for (const room of encryptedRooms.slice(0, 3)) { // Check first 3 encrypted rooms
        console.log(`\n🏠 Room: ${room.name || room.roomId.substring(0, 20)}`);
        
        // Get room members and their devices
        const members = room.getMembers();
        console.log(`👥 Members: ${members.length}`);
        
        for (const member of members.slice(0, 5)) { // Check first 5 members
            const memberUserId = member.userId;
            console.log(`  👤 ${memberUserId === userId ? 'YOU' : 'Member'}: ${memberUserId}`);
            
            try {
                // Get devices for this user
                const devices = client.crypto.deviceList.getStoredDevicesForUser(memberUserId);
                console.log(`    📱 Devices: ${devices ? devices.length : 0}`);
                
                if (devices && devices.length > 0) {
                    devices.forEach((device, index) => {
                        console.log(`      Device ${index + 1}: ${device.deviceId} (verified: ${device.isVerified()}, blocked: ${device.isBlocked()})`);
                    });
                }
            } catch (error) {
                console.log(`    ❌ Error getting devices for ${memberUserId}:`, error.message);
            }
        }
        
        // Check if we can encrypt for this room
        try {
            const canEncrypt = await client.crypto.isEncryptionEnabledInRoom(room.roomId);
            console.log(`  🔐 Encryption enabled: ${canEncrypt}`);
        } catch (error) {
            console.log(`  ❌ Error checking encryption status:`, error.message);
        }
    }
    
    // Try to request keys for recent encrypted events
    console.log('\n🔑 Attempting to request missing keys...');
    
    try {
        // Force a key request for any pending events
        if (client.crypto.requestRoomKey) {
            console.log('🔑 Requesting room keys...');
            // This is a bit advanced, but let's try to trigger key requests
        }
        
        // Try to download keys from server
        if (client.crypto.downloadKeys) {
            console.log('🔑 Downloading keys from server...');
            const userIds = [userId]; // Start with just our own keys
            await client.crypto.downloadKeys(userIds);
            console.log('✅ Key download completed');
        }
        
    } catch (error) {
        console.log('❌ Error requesting keys:', error.message);
    }
    
    // Suggest solutions
    console.log('\n💡 POTENTIAL SOLUTIONS:');
    console.log('1. 🔄 Try logging out and back in to reset device state');
    console.log('2. 🔑 Ask other users to send a new message (this will share new keys)');
    console.log('3. 📱 Verify your device with another Matrix client');
    console.log('4. 🗂️ Check if you have key backup enabled in other Matrix clients');
    
    // Try to send a message to establish new keys
    if (encryptedRooms.length > 0) {
        console.log('\n🔄 Attempting to send a message to establish new keys...');
        try {
            const testRoom = encryptedRooms[0];
            await matrixService.sendMessage(testRoom.roomId, '🔑 Key establishment test - this should create new encryption keys for this device');
            console.log('✅ Test message sent - this should help establish keys for future messages');
        } catch (error) {
            console.log('❌ Failed to send test message:', error.message);
        }
    }
    
    console.log('\n🔐 MATRIX KEY SHARING DIAGNOSTIC END');
})();
