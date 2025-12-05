// Fix script for key conflict issues (One time key already exists)
console.log('🔑 FIXING KEY CONFLICT ISSUE');

const matrixService = window.matrixService;
if (!matrixService || !matrixService.client) {
  console.log('❌ MatrixService or client not available');
} else {
  const client = matrixService.client;
  
  if (!client.crypto) {
    console.log('❌ Crypto not available');
  } else {
    try {
      console.log('🔑 Starting key conflict resolution...');
      console.log('🆔 User ID:', client.getUserId());
      console.log('🔧 Device ID:', client.getDeviceId());
      
      // Step 1: Try to clear the crypto store and reinitialize
      console.log('🔑 Step 1: Attempting to clear crypto store...');
      
      // Check if we can access the crypto store methods
      if (client.crypto.olmDevice) {
        console.log('🔐 Olm device available, attempting to clear keys...');
        
        // Try to clear one-time keys if method exists
        if (typeof client.crypto.olmDevice.markKeysAsPublished === 'function') {
          try {
            client.crypto.olmDevice.markKeysAsPublished();
            console.log('✅ Marked existing keys as published');
          } catch (markError) {
            console.warn('⚠️ Could not mark keys as published:', markError.message);
          }
        }
        
        // Try to generate new one-time keys
        if (typeof client.crypto.olmDevice.generateOneTimeKeys === 'function') {
          try {
            client.crypto.olmDevice.generateOneTimeKeys(10); // Generate 10 new keys
            console.log('✅ Generated new one-time keys');
          } catch (generateError) {
            console.warn('⚠️ Could not generate new keys:', generateError.message);
          }
        }
      }
      
      // Step 2: Try to force a key upload with different parameters
      console.log('🔑 Step 2: Attempting to upload keys with retry logic...');
      
      if (typeof client.crypto.uploadKeys === 'function') {
        try {
          // Try uploading with force flag if available
          await client.crypto.uploadKeys(true); // Force upload
          console.log('✅ Successfully uploaded keys with force flag');
        } catch (uploadError) {
          console.warn('⚠️ Force upload failed:', uploadError.message);
          
          // Try regular upload
          try {
            await client.crypto.uploadKeys();
            console.log('✅ Successfully uploaded keys (regular)');
          } catch (regularUploadError) {
            console.warn('⚠️ Regular upload also failed:', regularUploadError.message);
          }
        }
      }
      
      // Step 3: Try to clear the device list and re-download
      console.log('🔑 Step 3: Refreshing device lists...');
      
      try {
        const userId = client.getUserId();
        
        // Clear device list for our own user
        if (typeof client.crypto.clearStoredDevicesForUser === 'function') {
          client.crypto.clearStoredDevicesForUser(userId);
          console.log('✅ Cleared stored devices for own user');
        }
        
        // Re-download device list
        if (typeof client.crypto.downloadKeys === 'function') {
          await client.crypto.downloadKeys([userId], true); // Force download
          console.log('✅ Re-downloaded device keys');
        }
      } catch (deviceError) {
        console.warn('⚠️ Device list refresh failed:', deviceError.message);
      }
      
      // Step 4: Try to reset the crypto session
      console.log('🔑 Step 4: Attempting crypto session reset...');
      
      try {
        // Check if we can stop and restart crypto
        if (typeof client.crypto.stop === 'function' && typeof client.initCrypto === 'function') {
          console.log('🔄 Stopping crypto...');
          client.crypto.stop();
          
          // Wait a moment
          await new Promise(resolve => setTimeout(resolve, 1000));
          
          console.log('🔄 Reinitializing crypto...');
          await client.initCrypto();
          console.log('✅ Crypto reinitialized successfully');
        }
      } catch (resetError) {
        console.warn('⚠️ Crypto reset failed:', resetError.message);
      }
      
      // Step 5: Test key upload after fixes
      console.log('🔑 Step 5: Testing key upload after fixes...');
      
      try {
        if (typeof client.crypto.uploadKeys === 'function') {
          await client.crypto.uploadKeys();
          console.log('✅ Key upload test successful!');
        }
      } catch (testError) {
        console.warn('⚠️ Key upload test failed:', testError.message);
        
        // If still failing, suggest more drastic measures
        console.log('🔑 Key conflict still exists. Suggesting device logout/login...');
        console.log('💡 RECOMMENDATION: You may need to:');
        console.log('   1. Logout from Matrix completely');
        console.log('   2. Clear browser storage/cache');
        console.log('   3. Login again to generate fresh device keys');
        console.log('   4. Or use a different device ID');
      }
      
      console.log('🔑 ✅ KEY CONFLICT FIX COMPLETE');
      
    } catch (error) {
      console.error('❌ Key conflict fix failed:', error);
      
      // Provide fallback recommendations
      console.log('🔑 FALLBACK RECOMMENDATIONS:');
      console.log('1. Try logging out and logging back in');
      console.log('2. Clear browser cache and cookies for this site');
      console.log('3. Use a different browser or incognito mode');
      console.log('4. Contact Matrix server admin if issue persists');
    }
  }
}
