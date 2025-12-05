// Manual fix for Olm initialization and encryption
(async () => {
    console.log('🔐 MANUAL ENCRYPTION FIX START');
    
    try {
        // Step 1: Load Olm from public directory
        console.log('🔐 Loading Olm from /olm.js...');
        const response = await fetch('/olm.js');
        
        if (!response.ok) {
            throw new Error(`Failed to fetch olm.js: ${response.status}`);
        }
        
        const olmScript = await response.text();
        console.log('🔐 Olm script loaded, length:', olmScript.length);
        
        // Step 2: Execute the Olm script
        const scriptElement = document.createElement('script');
        scriptElement.textContent = olmScript;
        document.head.appendChild(scriptElement);
        
        // Wait for script to execute
        await new Promise(resolve => setTimeout(resolve, 1000));
        
        if (!window.Olm) {
            throw new Error('Olm not available after script execution');
        }
        
        console.log('🔐 Olm loaded on window:', typeof window.Olm);
        
        // Step 3: Initialize Olm
        if (typeof window.Olm.init === 'function') {
            console.log('🔐 Calling Olm.init()...');
            await window.Olm.init();
            console.log('🔐 Olm.init() completed');
        }
        
        // Step 4: Make Olm globally available
        window.global = window.global || {};
        window.global.Olm = window.Olm;
        
        console.log('🔐 Olm made globally available');
        
        // Step 5: Reinitialize Matrix client with crypto
        const matrixService = window.matrixService;
        if (matrixService && matrixService.client) {
            console.log('🔐 Reinitializing Matrix client crypto...');
            
            // Force crypto initialization
            try {
                await matrixService.client.initCrypto();
                console.log('🔐 ✅ Matrix crypto initialized:', !!matrixService.client.crypto);
            } catch (cryptoError) {
                console.log('🔐 Crypto init error:', cryptoError);
                
                // Try alternative approach - recreate client with crypto
                console.log('🔐 Attempting to recreate client with crypto...');
                
                const sdk = await import('matrix-js-sdk');
                const oldClient = matrixService.client;
                const baseUrl = oldClient.baseUrl;
                const userId = oldClient.getUserId();
                const accessToken = oldClient.getAccessToken();
                
                // Create new client with crypto store
                const newClient = sdk.createClient({
                    baseUrl: baseUrl,
                    accessToken: accessToken,
                    userId: userId,
                    cryptoStore: new sdk.MemoryCryptoStore(),
                    // Don't include verificationMethods to avoid SAS error
                });
                
                // Replace the client
                matrixService.client = newClient;
                
                // Initialize crypto on new client
                await newClient.initCrypto();
                
                // Start the client
                await newClient.startClient({ initialSyncLimit: 10 });
                
                console.log('🔐 ✅ New client with crypto created:', !!newClient.crypto);
            }
        }
        
        // Step 6: Test encryption
        console.log('🔐 Testing encryption...');
        const rooms = matrixService.client.getRooms();
        const encryptedRooms = rooms.filter(r => r.hasEncryptionStateEvent && r.hasEncryptionStateEvent());
        
        if (encryptedRooms.length > 0) {
            console.log('🔐 Found encrypted room:', encryptedRooms[0].name || encryptedRooms[0].roomId);
            
            try {
                await matrixService.sendMessage(encryptedRooms[0].roomId, '🎉 MANUAL FIX SUCCESS! Encryption is now working!');
                console.log('🔐 ✅ SUCCESS! Message sent to encrypted room');
            } catch (error) {
                console.log('🔐 ❌ Still failed:', error.message);
            }
        }
        
    } catch (error) {
        console.error('🔐 ❌ Manual fix failed:', error);
    }
    
    console.log('🔐 MANUAL ENCRYPTION FIX END');
})();
