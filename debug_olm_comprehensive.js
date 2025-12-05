// Comprehensive Olm debugging script
(async () => {
    console.log('🔐 COMPREHENSIVE OLM DEBUG START');
    
    // Check current state
    console.log('🔐 Current window.Olm:', typeof window.Olm, window.Olm);
    console.log('🔐 Current global.Olm:', typeof (typeof global !== 'undefined' ? global.Olm : 'undefined'));
    
    // Check if MatrixService has Olm initialization
    const matrixService = window.matrixService;
    if (matrixService) {
        console.log('🔐 MatrixService exists');
        console.log('🔐 MatrixService client:', !!matrixService.client);
        console.log('🔐 MatrixService client crypto:', !!matrixService.client?.crypto);
    }
    
    // Try to manually load and initialize Olm
    console.log('🔐 Attempting manual Olm initialization...');
    
    try {
        // Try dynamic import
        console.log('🔐 Trying dynamic import...');
        const olmModule = await import('@matrix-org/olm');
        console.log('🔐 Dynamic import result:', olmModule);
        console.log('🔐 Dynamic import default:', olmModule.default);
        console.log('🔐 Dynamic import keys:', Object.keys(olmModule));
        
        let Olm = olmModule.default || olmModule;
        console.log('🔐 Selected Olm object:', typeof Olm, Olm);
        
        if (Olm && typeof Olm.init === 'function') {
            console.log('🔐 Calling Olm.init()...');
            await Olm.init();
            console.log('🔐 Olm.init() completed');
            
            // Make it globally available
            window.Olm = Olm;
            if (typeof global !== 'undefined') {
                global.Olm = Olm;
            }
            
            console.log('🔐 Made Olm globally available');
            console.log('🔐 window.Olm after init:', typeof window.Olm);
            console.log('🔐 global.Olm after init:', typeof (typeof global !== 'undefined' ? global.Olm : 'undefined'));
            
            // Try to reinitialize the Matrix client crypto
            if (matrixService && matrixService.client) {
                console.log('🔐 Attempting to reinitialize Matrix client crypto...');
                try {
                    if (typeof matrixService.client.initCrypto === 'function') {
                        await matrixService.client.initCrypto();
                        console.log('🔐 ✅ Matrix client crypto reinitialized successfully');
                        console.log('🔐 Matrix client crypto module now:', !!matrixService.client.crypto);
                    } else {
                        console.log('🔐 ❌ initCrypto method not available on client');
                    }
                } catch (cryptoError) {
                    console.log('🔐 ❌ Failed to reinitialize crypto:', cryptoError);
                }
            }
            
        } else {
            console.log('🔐 ❌ Olm object does not have init method');
            console.log('🔐 Olm object type:', typeof Olm);
            console.log('🔐 Olm object keys:', Olm ? Object.keys(Olm) : 'null');
        }
        
    } catch (error) {
        console.log('🔐 ❌ Manual Olm initialization failed:', error);
        console.log('🔐 Error details:', error.message, error.stack);
    }
    
    console.log('🔐 COMPREHENSIVE OLM DEBUG END');
})();
