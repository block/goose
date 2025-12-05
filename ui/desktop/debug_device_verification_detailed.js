// Enhanced device verification debug script
console.log('🔐 ENHANCED DEVICE VERIFICATION DEBUG START');

const client = window.matrixService?.client;
const config = window.matrixService?.config;

if (!client) {
  console.error('❌ Matrix client not available');
} else if (!config) {
  console.error('❌ Matrix config not available');
} else {
  console.log('✅ Client and config available');
  console.log('🔐 Current user ID:', config.userId);
  console.log('🔐 Current device ID:', config.deviceId);
  console.log('🔐 Crypto module available:', !!client.crypto);
  
  if (client.crypto) {
    try {
      // Check our own device status
      console.log('🔍 Checking own device status...');
      
      // Get all devices for current user
      client.crypto.getStoredDevicesForUser(config.userId).then(devices => {
        console.log(`🔐 Found ${devices.length} devices for user ${config.userId}:`);
        
        devices.forEach((device, index) => {
          const isOwnDevice = device.deviceId === config.deviceId;
          console.log(`📱 Device ${index + 1}${isOwnDevice ? ' (THIS DEVICE)' : ''}:`, {
            deviceId: device.deviceId,
            isVerified: device.isVerified(),
            isBlocked: device.isBlocked(),
            isKnown: device.isKnown(),
            displayName: device.getDisplayName(),
            algorithms: device.algorithms,
            keys: Object.keys(device.keys || {})
          });
          
          if (isOwnDevice) {
            console.log('🔐 ⭐ OWN DEVICE DETAILED STATUS:', {
              deviceId: device.deviceId,
              verified: device.isVerified(),
              blocked: device.isBlocked(),
              known: device.isKnown(),
              displayName: device.getDisplayName(),
              fingerprintSha256: device.getFingerprint(),
              keys: device.keys
            });
          }
        });
        
        // Check device trust status using crypto methods
        if (typeof client.crypto.checkDeviceTrust === 'function') {
          const deviceTrust = client.crypto.checkDeviceTrust(config.userId, config.deviceId);
          console.log('🔐 Device trust status:', {
            isVerified: deviceTrust.isVerified(),
            isCrossSigningVerified: deviceTrust.isCrossSigningVerified(),
            isTofu: deviceTrust.isTofu(),
            isLocallyVerified: deviceTrust.isLocallyVerified(),
            trustLevel: deviceTrust.toString()
          });
        }
        
        // Check cross-signing status
        if (typeof client.crypto.isCrossSigningReady === 'function') {
          console.log('🔐 Cross-signing ready:', client.crypto.isCrossSigningReady());
        }
        
        // Check if there are any verification requests
        if (typeof client.crypto.getVerificationRequestsToDeviceInProgress === 'function') {
          const verificationRequests = client.crypto.getVerificationRequestsToDeviceInProgress(config.userId);
          console.log('🔐 Verification requests in progress:', verificationRequests.length);
          verificationRequests.forEach((req, index) => {
            console.log(`📝 Verification request ${index + 1}:`, {
              requestId: req.requestId,
              phase: req.phase,
              methods: req.methods,
              otherUserId: req.otherUserId,
              otherDeviceId: req.otherDeviceId
            });
          });
        }
        
      }).catch(error => {
        console.error('❌ Failed to get stored devices:', error);
      });
      
      // Check global blacklist setting
      console.log('🔐 Global blacklist unverified devices:', client.getGlobalBlacklistUnverifiedDevices());
      
      // Check room encryption status
      const rooms = client.getRooms();
      const encryptedRooms = rooms.filter(room => room.hasEncryptionStateEvent && room.hasEncryptionStateEvent());
      console.log(`🔐 Found ${encryptedRooms.length} encrypted rooms out of ${rooms.length} total rooms`);
      
      if (encryptedRooms.length > 0) {
        const sampleRoom = encryptedRooms[0];
        console.log(`🏠 Sample encrypted room: ${sampleRoom.name || 'Unnamed'} (${sampleRoom.roomId})`);
        
        // Check room members and their devices
        const members = sampleRoom.getMembers();
        console.log(`👥 Room has ${members.length} members`);
        
        members.slice(0, 3).forEach((member, index) => {
          console.log(`👤 Member ${index + 1}: ${member.name || member.userId} (${member.userId})`);
          
          // Get devices for this member
          client.crypto.getStoredDevicesForUser(member.userId).then(memberDevices => {
            console.log(`📱 Member ${member.userId} has ${memberDevices.length} devices:`);
            memberDevices.forEach((device, deviceIndex) => {
              console.log(`  📱 Device ${deviceIndex + 1}:`, {
                deviceId: device.deviceId,
                verified: device.isVerified(),
                blocked: device.isBlocked(),
                known: device.isKnown()
              });
            });
          }).catch(error => {
            console.warn(`⚠️ Failed to get devices for member ${member.userId}:`, error);
          });
        });
      }
      
    } catch (error) {
      console.error('❌ Error during device verification debug:', error);
    }
  } else {
    console.error('❌ Crypto module not available');
  }
}

console.log('🔐 ENHANCED DEVICE VERIFICATION DEBUG END');
