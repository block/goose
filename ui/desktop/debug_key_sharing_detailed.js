// Comprehensive key sharing and decryption diagnostic
(async () => {
    console.log('🔐 COMPREHENSIVE KEY SHARING DIAGNOSTIC START');
    
    const matrixService = window.matrixService;
    if (!matrixService || !matrixService.client) {
        console.log('❌ Matrix service or client not available.');
        return;
    }

    const client = matrixService.client;
    const crypto = client.crypto;
    
    console.log('✅ Client exists');
    console.log('🔐 Crypto module:', !!crypto);
    console.log('🔐 window.Olm:', typeof window.Olm);
    
    if (!crypto) {
        console.log('❌ No crypto module - encryption not supported');
        return;
    }
    
    // Get our own device info
    const userId = client.getUserId();
    const deviceId = client.getDeviceId();
    console.log('🔐 Our user ID:', userId);
    console.log('🔐 Our device ID:', deviceId);
    
    // Check our own device verification status
    try {
        const ownDevices = await crypto.getStoredDevicesForUser(userId);
        const ownDevice = ownDevices.find(d => d.deviceId === deviceId);
        
        if (ownDevice) {
            console.log('🔐 Our device status:', {
                verified: ownDevice.isVerified(),
                blocked: ownDevice.isBlocked(),
                known: ownDevice.isKnown(),
                deviceId: ownDevice.deviceId
            });
        } else {
            console.log('🔐 ⚠️ Our own device not found in stored devices');
        }
    } catch (error) {
        console.log('🔐 ❌ Failed to get our own device info:', error);
    }
    
    // Find encrypted rooms and analyze them
    const rooms = client.getRooms();
    const encryptedRooms = rooms.filter(r => r.hasEncryptionStateEvent && r.hasEncryptionStateEvent());
    
    console.log(`🔐 Found ${encryptedRooms.length} encrypted rooms`);
    
    if (encryptedRooms.length === 0) {
        console.log('❌ No encrypted rooms to analyze');
        return;
    }
    
    // Analyze the first encrypted room in detail
    const room = encryptedRooms[0];
    console.log(`\n🏠 Analyzing room: ${room.name || room.roomId.substring(0, 20) + '...'}`);
    
    // Get all members and their devices
    const members = room.getMembers();
    console.log(`👥 Room has ${members.length} members`);
    
    let totalDevices = 0;
    let verifiedDevices = 0;
    let blockedDevices = 0;
    
    for (const member of members) {
        console.log(`\n👤 Member: ${member.name || member.userId}`);
        
        try {
            const devices = await crypto.getStoredDevicesForUser(member.userId);
            totalDevices += devices.length;
            
            console.log(`  📱 Has ${devices.length} devices:`);
            
            devices.forEach(device => {
                const isVerified = device.isVerified();
                const isBlocked = device.isBlocked();
                const isKnown = device.isKnown();
                
                if (isVerified) verifiedDevices++;
                if (isBlocked) blockedDevices++;
                
                console.log(`    📱 Device ${device.deviceId.substring(0, 8)}...:`);
                console.log(`       Verified: ${isVerified}`);
                console.log(`       Blocked: ${isBlocked}`);
                console.log(`       Known: ${isKnown}`);
                console.log(`       Keys: ${device.keys ? Object.keys(device.keys).length : 0}`);
            });
        } catch (error) {
            console.log(`  ❌ Error getting devices for ${member.userId}:`, error.message);
        }
    }
    
    console.log(`\n📊 Device Summary:`);
    console.log(`   Total devices: ${totalDevices}`);
    console.log(`   Verified devices: ${verifiedDevices}`);
    console.log(`   Blocked devices: ${blockedDevices}`);
    console.log(`   Unverified devices: ${totalDevices - verifiedDevices}`);
    
    // Check room key sharing status
    console.log(`\n🔑 Room Key Analysis:`);
    
    try {
        // Get room timeline to check encrypted events
        const timeline = room.getLiveTimeline();
        const events = timeline.getEvents();
        const encryptedEvents = events.filter(e => e.getType() === 'm.room.encrypted');
        
        console.log(`📜 Found ${encryptedEvents.length} encrypted events in timeline`);
        
        if (encryptedEvents.length > 0) {
            // Analyze the last few encrypted events
            const recentEncrypted = encryptedEvents.slice(-5);
            console.log(`🔍 Analyzing last ${recentEncrypted.length} encrypted events:`);
            
            recentEncrypted.forEach((event, index) => {
                const sender = event.getSender();
                const eventId = event.getId();
                const timestamp = new Date(event.getTs()).toISOString();
                
                console.log(`\n📨 Event ${index + 1}:`);
                console.log(`   ID: ${eventId}`);
                console.log(`   Sender: ${sender}`);
                console.log(`   Time: ${timestamp}`);
                
                // Check if we can decrypt this event
                try {
                    if (typeof event.getClearEvent === 'function') {
                        const clearEvent = event.getClearEvent();
                        if (clearEvent && clearEvent.content && clearEvent.content.body) {
                            console.log(`   ✅ DECRYPTED: "${clearEvent.content.body.substring(0, 50)}..."`);
                        } else {
                            console.log(`   🔄 Not yet decrypted`);
                        }
                    } else {
                        console.log(`   ❌ getClearEvent method not available`);
                    }
                    
                    if (typeof event.isDecryptionFailure === 'function' && event.isDecryptionFailure()) {
                        const reason = event.decryptionFailureReason || 'Unknown';
                        console.log(`   ❌ DECRYPTION FAILED: ${reason}`);
                    }
                } catch (decryptError) {
                    console.log(`   ❌ Decryption error: ${decryptError.message}`);
                }
            });
        }
    } catch (timelineError) {
        console.log(`❌ Failed to analyze timeline: ${timelineError.message}`);
    }
    
    // Test key request/sharing
    console.log(`\n🔄 Testing Key Request/Sharing:`);
    
    try {
        // Try to request keys for the room
        console.log('🔑 Attempting to request room keys...');
        
        // Check if there are any key sharing methods available
        if (typeof crypto.requestRoomKey === 'function') {
            console.log('🔑 requestRoomKey method available');
        } else {
            console.log('🔑 requestRoomKey method not available');
        }
        
        if (typeof crypto.shareRoomKey === 'function') {
            console.log('🔑 shareRoomKey method available');
        } else {
            console.log('🔑 shareRoomKey method not available');
        }
        
        // Check cross-signing status
        if (typeof crypto.isCrossSigningReady === 'function') {
            const crossSigningReady = await crypto.isCrossSigningReady();
            console.log('🔐 Cross-signing ready:', crossSigningReady);
        }
        
    } catch (keyError) {
        console.log(`❌ Key sharing test failed: ${keyError.message}`);
    }
    
    // Recommendations
    console.log(`\n💡 RECOMMENDATIONS:`);
    
    if (verifiedDevices === 0) {
        console.log('1. ⚠️  No devices are verified - this prevents key sharing');
        console.log('   Solution: Verify devices manually or disable verification requirements');
    }
    
    if (blockedDevices > 0) {
        console.log(`2. ⚠️  ${blockedDevices} devices are blocked`);
        console.log('   Solution: Unblock devices to allow key sharing');
    }
    
    if (totalDevices - verifiedDevices > verifiedDevices) {
        console.log('3. ⚠️  More unverified than verified devices');
        console.log('   Solution: Auto-verify all devices for development');
    }
    
    console.log('4. 💬 Try sending a NEW message to test current encryption');
    console.log('5. 🔄 Historical messages may remain undecryptable');
    
    console.log('\n🔐 COMPREHENSIVE KEY SHARING DIAGNOSTIC END');
})();
