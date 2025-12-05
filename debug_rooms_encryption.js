// Debug room encryption states
(async () => {
    console.log('🔐 ROOM ENCRYPTION DEBUG START');
    
    const matrixService = window.matrixService;
    if (!matrixService || !matrixService.client) {
        console.log('❌ Matrix service not available');
        return;
    }
    
    console.log('🔐 Crypto module active:', !!matrixService.client.crypto);
    
    const rooms = matrixService.client.getRooms();
    console.log('🔐 Total rooms:', rooms.length);
    
    // Check all rooms for encryption
    rooms.forEach((room, index) => {
        const roomId = room.roomId;
        const roomName = room.name || 'Unnamed Room';
        const hasEncryption = room.hasEncryptionStateEvent && room.hasEncryptionStateEvent();
        const isEncrypted = room.isEncrypted && room.isEncrypted();
        
        console.log(`🔐 Room ${index + 1}: ${roomName} (${roomId})`);
        console.log(`   - hasEncryptionStateEvent: ${hasEncryption}`);
        console.log(`   - isEncrypted: ${isEncrypted}`);
        
        if (hasEncryption || isEncrypted) {
            // Try to get encryption info
            try {
                const encryptionEvent = room.currentState.getStateEvents('m.room.encryption', '');
                console.log(`   - Encryption event:`, encryptionEvent ? 'Present' : 'Missing');
                if (encryptionEvent) {
                    console.log(`   - Algorithm:`, encryptionEvent.getContent().algorithm);
                }
            } catch (e) {
                console.log(`   - Error getting encryption event:`, e.message);
            }
        }
    });
    
    // Try to create a new encrypted room for testing
    console.log('🔐 Attempting to create a test encrypted room...');
    try {
        const testRoomId = await matrixService.client.createRoom({
            name: 'Goose Encryption Test Room',
            topic: 'Testing encryption functionality in Goose',
            initial_state: [{
                type: 'm.room.encryption',
                content: {
                    algorithm: 'm.megolm.v1.aes-sha2'
                }
            }],
            preset: 'private_chat'
        });
        
        console.log('🔐 ✅ Created test encrypted room:', testRoomId);
        
        // Wait a moment for the room to be ready
        await new Promise(resolve => setTimeout(resolve, 2000));
        
        // Try to send a message to the new room
        console.log('🔐 Testing message send to new encrypted room...');
        await matrixService.sendMessage(testRoomId, '🎉 SUCCESS! This is a test message in a newly created encrypted room!');
        console.log('🔐 ✅ SUCCESS! Message sent to new encrypted room');
        
    } catch (error) {
        console.log('🔐 ❌ Failed to create or send to test room:', error.message);
    }
    
    console.log('🔐 ROOM ENCRYPTION DEBUG END');
})();
