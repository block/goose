// Debug crypto initialization process
console.log('🔐 DEBUGGING CRYPTO INITIALIZATION PROCESS...');

// Check if we can manually initialize crypto
const client = matrixService?.client;
if (!client) {
  console.error('❌ No Matrix client available');
} else {
  console.log('✅ Matrix client available');
  console.log('🔐 Client type:', client.constructor.name);
  console.log('🔐 Current crypto state:', !!client.crypto);
  console.log('🔐 initCrypto method:', typeof client.initCrypto);
  
  // Check the client configuration
  console.log('🔐 Client config inspection:');
  console.log('  - baseUrl:', client.baseUrl);
  console.log('  - userId:', client.getUserId());
  console.log('  - deviceId:', client.getDeviceId());
  console.log('  - accessToken exists:', !!client.getAccessToken());
  
  // Try to manually call initCrypto and see what happens
  if (typeof client.initCrypto === 'function') {
    console.log('🔐 Attempting manual crypto initialization...');
    
    client.initCrypto().then(() => {
      console.log('🔐 ✅ Manual crypto initialization successful!');
      console.log('🔐 Crypto module after manual init:', !!client.crypto);
      
      if (client.crypto) {
        console.log('🔐 Crypto details after manual init:');
        console.log('  - crypto.store:', !!client.crypto.store);
        console.log('  - crypto.olmDevice:', !!client.crypto.olmDevice);
        console.log('  - crypto.deviceList:', !!client.crypto.deviceList);
      }
    }).catch(error => {
      console.error('🔐 ❌ Manual crypto initialization failed:', error);
      console.error('🔐 Error details:', {
        name: error.name,
        message: error.message,
        stack: error.stack
      });
    });
  } else {
    console.log('🔐 ❌ initCrypto method not available');
  }
  
  // Check Matrix SDK version and crypto support
  console.log('🔐 SDK version check:');
  const sdk = window.matrixcs || {};
  console.log('  - SDK object keys:', Object.keys(sdk).slice(0, 10));
  console.log('  - createClient:', typeof sdk.createClient);
  console.log('  - MemoryCryptoStore:', typeof sdk.MemoryCryptoStore);
  console.log('  - verificationMethods:', sdk.verificationMethods);
  
  // Try creating a test client with crypto to see if it works
  console.log('🔐 Testing crypto client creation...');
  try {
    const testClient = sdk.createClient({
      baseUrl: 'https://matrix.tchncs.de',
      cryptoStore: new sdk.MemoryCryptoStore(),
    });
    
    console.log('🔐 Test client created:', !!testClient);
    console.log('🔐 Test client crypto:', !!testClient.crypto);
    console.log('🔐 Test client initCrypto:', typeof testClient.initCrypto);
    
    if (typeof testClient.initCrypto === 'function') {
      testClient.initCrypto().then(() => {
        console.log('🔐 ✅ Test client crypto init successful!');
        console.log('🔐 Test client crypto after init:', !!testClient.crypto);
      }).catch(err => {
        console.error('🔐 ❌ Test client crypto init failed:', err);
      });
    }
  } catch (testError) {
    console.error('🔐 ❌ Test client creation failed:', testError);
  }
}

console.log('🔐 Crypto initialization debugging complete');
