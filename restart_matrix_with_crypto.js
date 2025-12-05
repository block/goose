// Force restart Matrix client with crypto enabled
console.log('🔄 Force restarting Matrix client with crypto support...');

async function restartMatrixWithCrypto() {
  if (!window.matrixService) {
    console.log('❌ Matrix service not available');
    return;
  }
  
  const matrixService = window.matrixService;
  
  try {
    console.log('🔄 Step 1: Disconnecting current client...');
    await matrixService.disconnect();
    
    console.log('🔄 Step 2: Clearing client reference...');
    matrixService.client = null;
    
    console.log('🔄 Step 3: Re-initializing with crypto support...');
    await matrixService.initialize();
    
    console.log('✅ Matrix client restarted with crypto support');
    
    // Wait a moment for sync to complete
    setTimeout(() => {
      console.log('🔐 Checking crypto status after restart...');
      const client = matrixService.client;
      if (client && client.crypto) {
        console.log('✅ Crypto is now available!');
        console.log('🔐 Crypto initialized:', !!client.crypto);
        console.log('🔐 Device ID:', client.getDeviceId());
      } else {
        console.log('❌ Crypto still not available after restart');
      }
    }, 3000);
    
  } catch (error) {
    console.error('❌ Failed to restart Matrix client:', error);
  }
}

// Execute the restart
restartMatrixWithCrypto();
