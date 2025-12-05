// Debug Olm availability
(async () => {
    console.log('🔐 CHECKING OLM AVAILABILITY...');
    
    // Check if Olm is available globally
    console.log('🔐 Global Olm (window):', typeof (window as any).Olm);
    console.log('🔐 Global Olm (global):', typeof (global as any).Olm);
    
    // Try to import Olm directly
    try {
        const OlmModule = await import('@matrix-org/olm');
        console.log('🔐 ✅ Olm import successful:', typeof OlmModule);
        console.log('🔐 Olm.init method:', typeof OlmModule.init);
        
        // Try to initialize Olm manually
        if (typeof OlmModule.init === 'function') {
            await OlmModule.init();
            console.log('🔐 ✅ Olm.init() completed');
            
            // Make it globally available
            (window as any).Olm = OlmModule;
            (global as any).Olm = OlmModule;
            
            console.log('🔐 ✅ Olm made globally available');
            console.log('🔐 Global Olm (window) after init:', typeof (window as any).Olm);
        }
    } catch (error) {
        console.log('🔐 ❌ Olm import failed:', error);
    }
    
    // Check matrix-js-sdk CRYPTO_ENABLED after Olm init
    try {
        const sdk = await import('matrix-js-sdk');
        console.log('🔐 SDK CRYPTO_ENABLED after Olm init:', sdk.CRYPTO_ENABLED);
    } catch (error) {
        console.log('🔐 ❌ SDK import failed:', error);
    }
    
    console.log('🔐 OLM CHECK COMPLETE');
})();
