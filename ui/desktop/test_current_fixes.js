// Test current fixes for device verification and space tile persistence
console.log('🧪 TESTING CURRENT FIXES');

const client = window.matrixService?.client;
const config = window.matrixService?.config;

if (!client || !config) {
  console.error('❌ Matrix client or config not available');
} else {
  console.log('✅ Testing current state...');
  
  // Test 1: Device verification status
  console.log('\n🔐 === DEVICE VERIFICATION TEST ===');
  console.log('🔐 Current user ID:', config.userId);
  console.log('🔐 Current device ID:', config.deviceId);
  console.log('🔐 Crypto module available:', !!client.crypto);
  
  if (client.crypto) {
    client.crypto.getStoredDevicesForUser(config.userId).then(devices => {
      console.log(`🔐 Found ${devices.length} devices for user:`);
      
      devices.forEach((device, index) => {
        const isOwnDevice = device.deviceId === config.deviceId;
        console.log(`📱 Device ${index + 1}${isOwnDevice ? ' (THIS DEVICE)' : ''}:`, {
          deviceId: device.deviceId.substring(0, 8) + '...',
          verified: device.isVerified(),
          blocked: device.isBlocked(),
          known: device.isKnown()
        });
      });
      
      // Check device trust
      if (typeof client.crypto.checkDeviceTrust === 'function') {
        const deviceTrust = client.crypto.checkDeviceTrust(config.userId, config.deviceId);
        console.log('🔐 Device trust status:', {
          isVerified: deviceTrust.isVerified(),
          isCrossSigningVerified: deviceTrust.isCrossSigningVerified(),
          isTofu: deviceTrust.isTofu()
        });
      }
      
    }).catch(error => {
      console.error('❌ Failed to get devices:', error);
    });
  }
  
  // Test 2: Room filtering (space tile persistence fix)
  console.log('\n🏠 === ROOM FILTERING TEST ===');
  const allRooms = client.getRooms();
  const joinedRooms = allRooms.filter(room => room.getMyMembership() === 'join');
  const leftRooms = allRooms.filter(room => room.getMyMembership() === 'leave');
  const invitedRooms = allRooms.filter(room => room.getMyMembership() === 'invite');
  
  console.log(`🏠 Total rooms known to client: ${allRooms.length}`);
  console.log(`🏠 Joined rooms: ${joinedRooms.length}`);
  console.log(`🏠 Left rooms: ${leftRooms.length}`);
  console.log(`🏠 Invited rooms: ${invitedRooms.length}`);
  
  if (leftRooms.length > 0) {
    console.log('🚪 Left rooms (should be filtered out):');
    leftRooms.forEach((room, index) => {
      console.log(`  ${index + 1}. ${room.name || 'Unnamed'} (${room.roomId.substring(0, 20)}...)`);
    });
  }
  
  // Test 3: Matrix service getRooms() method
  console.log('\n📋 === MATRIX SERVICE getRooms() TEST ===');
  const matrixServiceRooms = window.matrixService.getRooms();
  console.log(`📋 MatrixService.getRooms() returned: ${matrixServiceRooms.length} rooms`);
  
  const spaces = matrixServiceRooms.filter(room => room.isSpace);
  console.log(`🌌 Spaces in getRooms(): ${spaces.length}`);
  
  if (spaces.length > 0) {
    console.log('🌌 Spaces:');
    spaces.forEach((space, index) => {
      console.log(`  ${index + 1}. ${space.name || 'Unnamed Space'} (${space.roomId.substring(0, 20)}...)`);
    });
  }
  
  // Test 4: Check for any verification requests
  console.log('\n📝 === VERIFICATION REQUESTS TEST ===');
  if (client.crypto && typeof client.crypto.getVerificationRequestsToDeviceInProgress === 'function') {
    const verificationRequests = client.crypto.getVerificationRequestsToDeviceInProgress(config.userId);
    console.log(`📝 Pending verification requests: ${verificationRequests.length}`);
    
    if (verificationRequests.length > 0) {
      verificationRequests.forEach((req, index) => {
        console.log(`📝 Request ${index + 1}:`, {
          requestId: req.requestId,
          phase: req.phase,
          otherUserId: req.otherUserId,
          otherDeviceId: req.otherDeviceId
        });
      });
    }
  }
  
  // Test 5: Global blacklist setting
  console.log('\n🔐 === GLOBAL SETTINGS TEST ===');
  console.log('🔐 Global blacklist unverified devices:', client.getGlobalBlacklistUnverifiedDevices());
  
  console.log('\n🎉 TESTING COMPLETE');
}
