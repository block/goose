// Debug script to check device verification status and try different approaches
(async () => {
    console.log('🔐 DEVICE VERIFICATION DEBUG START');
    
    const matrixService = window.matrixService;
    if (!matrixService || !matrixService.client) {
        console.log('❌ Matrix service or client not available.');
        return;
    }

    const client = matrixService.client;
    
    console.log('✅ Client exists');
    console.log('🔐 Crypto module:', !!client.crypto);
    console.log('🔐 window.Olm:', typeof window.Olm);
    
    // Check the blacklist setting
    const blacklistEnabled = client.getGlobalBlacklistUnverifiedDevices?.();
    console.log('🔐 Blacklist unverified devices enabled:', blacklistEnabled);
    console.log('🔐 Unverified devices allowed:', !blacklistEnabled);
    
    // Find an encrypted room
    const rooms = client.getRooms();
    const encryptedRooms = rooms.filter(r => r.hasEncryptionStateEvent && r.hasEncryptionStateEvent());
    
    if (encryptedRooms.length === 0) {
        console.log('❌ No encrypted rooms found');
        return;
    }
    
    const testRoom = encryptedRooms[0];
    console.log('🔐 Testing with room:', testRoom.name || testRoom.roomId.substring(0, 20) + '...');
    
    // Get room members and their devices
    const members = testRoom.getMembers();
    console.log('👥 Room members:', members.length);
    
    for (const member of members) {
        console.log(`\n👤 Member: ${member.name || member.userId}`);
        
        if (client.crypto && typeof client.crypto.getStoredDevicesForUser === 'function') {
            try {
                const devices = await client.crypto.getStoredDevicesForUser(member.userId);
                console.log(`  📱 Devices: ${devices.length}`);
                
                devices.forEach(device => {
                    console.log(`    Device ${device.deviceId}: verified=${device.isVerified()}, blocked=${device.isBlocked()}, known=${device.isKnown()}`);
                });
            } catch (error) {
                console.log(`  ❌ Error getting devices for ${member.userId}:`, error.message);
            }
        }
    }
    
    // Try different approaches to send a message
    console.log('\n🔐 TESTING MESSAGE SENDING APPROACHES...');
    
    // Approach 1: Try with current settings
    console.log('\n1️⃣ Testing with current settings...');
    try {
        await client.sendEvent(testRoom.roomId, 'm.room.message', {
            msgtype: 'm.text',
            body: '🧪 Test message 1: Current settings'
        });
        console.log('✅ Approach 1 SUCCESS: Message sent with current settings');
    } catch (error) {
        console.log('❌ Approach 1 FAILED:', error.message);
        console.log('   Error name:', error.name);
        console.log('   Error constructor:', error.constructor.name);
    }
    
    // Approach 2: Try setting crypto store to allow unknown devices
    if (client.crypto && typeof client.crypto.setGlobalBlacklistUnverifiedDevices === 'function') {
        console.log('\n2️⃣ Testing with crypto-level blacklist disabled...');
        try {
            client.crypto.setGlobalBlacklistUnverifiedDevices(false);
            console.log('🔧 Set crypto-level blacklist to false');
            
            await client.sendEvent(testRoom.roomId, 'm.room.message', {
                msgtype: 'm.text',
                body: '🧪 Test message 2: Crypto-level blacklist disabled'
            });
            console.log('✅ Approach 2 SUCCESS: Message sent with crypto-level blacklist disabled');
        } catch (error) {
            console.log('❌ Approach 2 FAILED:', error.message);
        }
    }
    
    // Approach 3: Try to mark all devices as verified
    if (client.crypto && typeof client.crypto.setDeviceVerification === 'function') {
        console.log('\n3️⃣ Testing with all devices marked as verified...');
        try {
            const allMembers = testRoom.getMembers();
            for (const member of allMembers) {
                try {
                    const devices = await client.crypto.getStoredDevicesForUser(member.userId);
                    for (const device of devices) {
                        if (!device.isVerified()) {
                            console.log(`🔧 Marking device ${device.deviceId} as verified for ${member.userId}`);
                            await client.crypto.setDeviceVerification(member.userId, device.deviceId, true);
                        }
                    }
                } catch (deviceError) {
                    console.log(`⚠️ Could not verify devices for ${member.userId}:`, deviceError.message);
                }
            }
            
            await client.sendEvent(testRoom.roomId, 'm.room.message', {
                msgtype: 'm.text',
                body: '🧪 Test message 3: All devices verified'
            });
            console.log('✅ Approach 3 SUCCESS: Message sent with all devices verified');
        } catch (error) {
            console.log('❌ Approach 3 FAILED:', error.message);
        }
    }
    
    // Approach 4: Try to send to an unencrypted room
    console.log('\n4️⃣ Testing with unencrypted room...');
    const unencryptedRooms = rooms.filter(r => !r.hasEncryptionStateEvent || !r.hasEncryptionStateEvent());
    
    if (unencryptedRooms.length > 0) {
        const unencryptedRoom = unencryptedRooms[0];
        console.log('🔓 Testing with unencrypted room:', unencryptedRoom.name || unencryptedRoom.roomId.substring(0, 20) + '...');
        
        try {
            await client.sendEvent(unencryptedRoom.roomId, 'm.room.message', {
                msgtype: 'm.text',
                body: '🧪 Test message 4: Unencrypted room'
            });
            console.log('✅ Approach 4 SUCCESS: Message sent to unencrypted room');
        } catch (error) {
            console.log('❌ Approach 4 FAILED:', error.message);
        }
    } else {
        console.log('⚠️ No unencrypted rooms found to test with');
    }
    
    // Final status check
    console.log('\n📊 FINAL STATUS:');
    console.log('🔐 Crypto available:', !!client.crypto);
    console.log('🔐 Global blacklist (client level):', client.getGlobalBlacklistUnverifiedDevices?.());
    if (client.crypto) {
        console.log('🔐 Global blacklist (crypto level):', client.crypto.getGlobalBlacklistUnverifiedDevices?.());
    }
    
    console.log('🔐 DEVICE VERIFICATION DEBUG END');
})();
