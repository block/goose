// Auto-verify device script to handle "New login. Was this you?" popups
console.log('🔐 AUTO DEVICE VERIFICATION START');

const client = window.matrixService?.client;
const config = window.matrixService?.config;

if (!client || !config || !client.crypto) {
  console.error('❌ Matrix client, config, or crypto not available');
} else {
  console.log('✅ Starting automatic device verification process...');
  
  async function autoVerifyOwnDevice() {
    try {
      console.log('🔐 Auto-verifying own device:', config.deviceId);
      
      // Get our own device
      const devices = await client.crypto.getStoredDevicesForUser(config.userId);
      const ownDevice = devices.find(device => device.deviceId === config.deviceId);
      
      if (ownDevice) {
        console.log('📱 Found own device:', {
          deviceId: ownDevice.deviceId,
          verified: ownDevice.isVerified(),
          blocked: ownDevice.isBlocked(),
          known: ownDevice.isKnown()
        });
        
        if (!ownDevice.isVerified()) {
          console.log('🔐 Marking own device as verified...');
          await client.crypto.setDeviceVerification(config.userId, config.deviceId, true);
          console.log('✅ Own device marked as verified');
        } else {
          console.log('✅ Own device already verified');
        }
        
        if (ownDevice.isBlocked()) {
          console.log('🔓 Unblocking own device...');
          await client.crypto.setDeviceBlocked(config.userId, config.deviceId, false);
          console.log('✅ Own device unblocked');
        }
        
        // Additional verification steps
        if (typeof client.crypto.checkDeviceTrust === 'function') {
          const deviceTrust = client.crypto.checkDeviceTrust(config.userId, config.deviceId);
          console.log('🔐 Device trust after verification:', {
            isVerified: deviceTrust.isVerified(),
            isCrossSigningVerified: deviceTrust.isCrossSigningVerified(),
            isTofu: deviceTrust.isTofu()
          });
        }
        
      } else {
        console.log('⚠️ Own device not found in stored devices, attempting direct verification...');
        try {
          await client.crypto.setDeviceVerification(config.userId, config.deviceId, true);
          console.log('✅ Own device verified directly');
        } catch (directError) {
          console.error('❌ Failed to verify own device directly:', directError);
        }
      }
      
    } catch (error) {
      console.error('❌ Failed to auto-verify own device:', error);
    }
  }
  
  async function handleVerificationRequests() {
    try {
      console.log('🔍 Checking for pending verification requests...');
      
      // Check if there are verification requests
      if (typeof client.crypto.getVerificationRequestsToDeviceInProgress === 'function') {
        const verificationRequests = client.crypto.getVerificationRequestsToDeviceInProgress(config.userId);
        console.log(`📝 Found ${verificationRequests.length} verification requests`);
        
        for (const request of verificationRequests) {
          console.log('📝 Processing verification request:', {
            requestId: request.requestId,
            phase: request.phase,
            methods: request.methods,
            otherUserId: request.otherUserId,
            otherDeviceId: request.otherDeviceId
          });
          
          // If this is a self-verification request, accept it
          if (request.otherUserId === config.userId && request.otherDeviceId === config.deviceId) {
            console.log('🤝 Auto-accepting self-verification request...');
            try {
              if (typeof request.accept === 'function') {
                await request.accept();
                console.log('✅ Self-verification request accepted');
              }
            } catch (acceptError) {
              console.warn('⚠️ Failed to accept verification request:', acceptError);
            }
          }
        }
      }
      
    } catch (error) {
      console.error('❌ Failed to handle verification requests:', error);
    }
  }
  
  async function markDeviceAsTrusted() {
    try {
      console.log('🔐 Marking device as trusted to prevent future popups...');
      
      // Try to mark the device as locally verified
      if (typeof client.crypto.setDeviceVerification === 'function') {
        await client.crypto.setDeviceVerification(config.userId, config.deviceId, true);
        console.log('✅ Device marked as locally verified');
      }
      
      // Try to mark as cross-signing verified if available
      if (typeof client.crypto.checkDeviceTrust === 'function') {
        const deviceTrust = client.crypto.checkDeviceTrust(config.userId, config.deviceId);
        if (!deviceTrust.isCrossSigningVerified() && typeof client.crypto.setDeviceCrossSigningVerified === 'function') {
          try {
            await client.crypto.setDeviceCrossSigningVerified(config.userId, config.deviceId, true);
            console.log('✅ Device marked as cross-signing verified');
          } catch (crossSignError) {
            console.warn('⚠️ Could not set cross-signing verification:', crossSignError);
          }
        }
      }
      
    } catch (error) {
      console.error('❌ Failed to mark device as trusted:', error);
    }
  }
  
  // Run all verification steps
  async function runFullVerification() {
    console.log('🚀 Running full device verification process...');
    
    await autoVerifyOwnDevice();
    await handleVerificationRequests();
    await markDeviceAsTrusted();
    
    console.log('🎉 Full device verification process completed');
    
    // Check final status
    try {
      const devices = await client.crypto.getStoredDevicesForUser(config.userId);
      const ownDevice = devices.find(device => device.deviceId === config.deviceId);
      
      if (ownDevice) {
        console.log('🏁 Final device status:', {
          deviceId: ownDevice.deviceId,
          verified: ownDevice.isVerified(),
          blocked: ownDevice.isBlocked(),
          known: ownDevice.isKnown()
        });
      }
      
      if (typeof client.crypto.checkDeviceTrust === 'function') {
        const deviceTrust = client.crypto.checkDeviceTrust(config.userId, config.deviceId);
        console.log('🏁 Final trust status:', {
          isVerified: deviceTrust.isVerified(),
          isCrossSigningVerified: deviceTrust.isCrossSigningVerified(),
          isTofu: deviceTrust.isTofu()
        });
      }
      
    } catch (error) {
      console.error('❌ Failed to check final status:', error);
    }
  }
  
  // Execute the verification
  runFullVerification();
}

console.log('🔐 AUTO DEVICE VERIFICATION END');
