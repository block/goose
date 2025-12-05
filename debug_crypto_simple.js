// Simple crypto debugging - copy and paste this into browser console
console.log('🔐 CRYPTO DEBUG START');

const client = matrixService?.client;
if (!client) {
  console.error('❌ No Matrix client');
} else {
  console.log('✅ Client exists');
  console.log('🔐 Crypto module:', !!client.crypto);
  console.log('🔐 initCrypto method:', typeof client.initCrypto);
  
  if (client.crypto) {
    console.log('🔐 Crypto store:', !!client.crypto.store);
    console.log('🔐 OLM device:', !!client.crypto.olmDevice);
  }
  
  // Check SDK
  const sdk = window.matrixcs || {};
  console.log('🔐 SDK MemoryCryptoStore:', typeof sdk.MemoryCryptoStore);
  console.log('🔐 SDK verificationMethods:', !!sdk.verificationMethods);
  
  // Test encryption capability
  try {
    const testRoom = client.getRooms().find(r => r.hasEncryptionStateEvent && r.hasEncryptionStateEvent());
    if (testRoom) {
      console.log('🔐 Found encrypted room:', testRoom.name);
      console.log('🔐 Room encrypted:', client.isRoomEncrypted ? client.isRoomEncrypted(testRoom.roomId) : 'unknown');
    } else {
      console.log('🔐 No encrypted rooms found');
    }
  } catch (e) {
    console.log('🔐 Error checking rooms:', e.message);
  }
}

console.log('🔐 CRYPTO DEBUG END');
