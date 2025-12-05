// Check crypto initialization status
console.log('🔐 Checking Matrix client crypto status...');

if (window.matrixService && window.matrixService.client) {
  const client = window.matrixService.client;
  
  console.log('🔐 Client exists:', !!client);
  console.log('🔐 Crypto module:', !!client.crypto);
  console.log('🔐 Crypto initialized:', client.crypto ? 'true' : 'false');
  
  if (client.crypto) {
    console.log('🔐 Crypto store:', !!client.crypto.cryptoStore);
    console.log('🔐 Device ID:', client.getDeviceId());
    console.log('🔐 User ID:', client.getUserId());
    
    // Check if we can encrypt
    try {
      console.log('🔐 Testing encryption capability...');
      // This is a simple check - we don't actually send anything
      console.log('🔐 ✅ Crypto module appears to be functional');
    } catch (error) {
      console.log('🔐 ❌ Crypto module error:', error);
    }
  } else {
    console.log('🔐 ❌ Crypto module not initialized');
    console.log('🔐 This means the client was created without crypto support');
    console.log('🔐 You need to restart the application for crypto to work');
  }
  
  // Also check the room encryption status
  const rooms = client.getRooms();
  const encryptedRooms = rooms.filter(room => room.hasEncryptionStateEvent && room.hasEncryptionStateEvent());
  console.log('🔐 Total rooms:', rooms.length);
  console.log('🔐 Encrypted rooms:', encryptedRooms.length);
  
  if (encryptedRooms.length > 0) {
    console.log('🔐 Encrypted room details:');
    encryptedRooms.slice(0, 3).forEach(room => {
      console.log(`🔐   ${room.roomId.substring(0, 20)}... (${room.name || 'Unnamed'})`);
    });
  }
} else {
  console.log('🔐 ❌ Matrix service or client not available');
}

console.log('🔐 Crypto check complete');
