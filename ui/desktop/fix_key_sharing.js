// Script to fix key sharing issues by verifying all devices
(async () => {
    console.log('🔧 FIXING KEY SHARING ISSUES');
    
    const matrixService = window.matrixService;
    if (!matrixService || !matrixService.client) {
        console.log('❌ Matrix service or client not available.');
        return;
    }

    const client = matrixService.client;
    const crypto = client.crypto;
    
    if (!crypto) {
        console.log('❌ No crypto module - cannot fix key sharing');
        return;
    }
    
    const userId = client.getUserId();
    const deviceId = client.getDeviceId();
    
    console.log('🔧 Our user ID:', userId);
    console.log('🔧 Our device ID:', deviceId);
    
    // Find encrypted rooms
    const rooms = client.getRooms();
    const encryptedRooms = rooms.filter(r => r.hasEncryptionStateEvent && r.hasEncryptionStateEvent());
    
    if (encryptedRooms.length === 0) {
        console.log('❌ No encrypted rooms found');
        return;
    }
    
    console.log(`🔧 Found ${encryptedRooms.length} encrypted rooms to fix`);
    
    let totalDevicesProcessed = 0;
    let devicesVerified = 0;
    let devicesUnblocked = 0;
    let errors = 0;
    
    // Process each encrypted room
    for (const room of encryptedRooms) {
        console.log(`\n🏠 Processing room: ${room.name || room.roomId.substring(0, 20) + '...'}`);
        
        const members = room.getMembers();
        
        for (const member of members) {
            console.log(`👤 Processing member: ${member.userId}`);
            
            try {
                const devices = await crypto.getStoredDevicesForUser(member.userId);
                totalDevicesProcessed += devices.length;
                
                for (const device of devices) {
                    console.log(`  📱 Processing device: ${device.deviceId.substring(0, 8)}...`);
                    
                    // Verify the device if not already verified
                    if (!device.isVerified()) {
                        try {
                            console.log(`    🔐 Verifying device...`);
                            await crypto.setDeviceVerification(member.userId, device.deviceId, true);
                            devicesVerified++;
                            console.log(`    ✅ Device verified`);
                        } catch (verifyError) {
                            console.log(`    ❌ Failed to verify device: ${verifyError.message}`);
                            errors++;
                        }
                    } else {
                        console.log(`    ✅ Device already verified`);
                    }
                    
                    // Unblock the device if blocked
                    if (device.isBlocked()) {
                        try {
                            console.log(`    🔓 Unblocking device...`);
                            await crypto.setDeviceBlocked(member.userId, device.deviceId, false);
                            devicesUnblocked++;
                            console.log(`    ✅ Device unblocked`);
                        } catch (unblockError) {
                            console.log(`    ❌ Failed to unblock device: ${unblockError.message}`);
                            errors++;
                        }
                    } else {
                        console.log(`    ✅ Device not blocked`);
                    }
                }
            } catch (memberError) {
                console.log(`  ❌ Failed to process member ${member.userId}: ${memberError.message}`);
                errors++;
            }
        }
    }
    
    console.log(`\n📊 SUMMARY:`);
    console.log(`   Total devices processed: ${totalDevicesProcessed}`);
    console.log(`   Devices verified: ${devicesVerified}`);
    console.log(`   Devices unblocked: ${devicesUnblocked}`);
    console.log(`   Errors: ${errors}`);
    
    // Additional crypto configuration
    console.log(`\n🔧 Applying additional crypto configuration...`);
    
    try {
        // Ensure global blacklist is disabled
        client.setGlobalBlacklistUnverifiedDevices(false);
        console.log('✅ Global blacklist disabled');
        
        // Also disable at crypto level if available
        if (typeof crypto.setGlobalBlacklistUnverifiedDevices === 'function') {
            crypto.setGlobalBlacklistUnverifiedDevices(false);
            console.log('✅ Crypto-level blacklist disabled');
        }
        
        // Enable key sharing if available
        if (typeof crypto.setDeviceVerification === 'function') {
            console.log('✅ Device verification methods available');
        }
        
    } catch (configError) {
        console.log(`❌ Configuration error: ${configError.message}`);
    }
    
    console.log(`\n🧪 Testing message send after fixes...`);
    
    // Test sending a message to the first encrypted room
    const testRoom = encryptedRooms[0];
    try {
        await client.sendEvent(testRoom.roomId, 'm.room.message', {
            msgtype: 'm.text',
            body: '🔧 Test message after key sharing fix - ' + new Date().toISOString()
        });
        console.log('✅ SUCCESS: Test message sent to encrypted room!');
        console.log('💡 Try refreshing the chat to see if new messages decrypt properly');
    } catch (sendError) {
        console.log('❌ FAILED to send test message:', sendError.message);
        console.log('   This indicates the key sharing issue persists');
    }
    
    console.log('\n💡 NEXT STEPS:');
    console.log('1. 🔄 Refresh the page/app to reload the Matrix client');
    console.log('2. 💬 Send a new message - it should encrypt/decrypt properly');
    console.log('3. 📜 Historical messages may still show decryption errors (this is normal)');
    console.log('4. 🔍 Run the diagnostic script again to verify fixes');
    
    console.log('\n🔧 KEY SHARING FIX COMPLETE');
})();
