// Direct Olm fix script
(async () => {
    console.log('🔐 DIRECT OLM FIX START');
    
    // Check what's actually in global
    console.log('🔐 Checking global object...');
    if (typeof global !== 'undefined') {
        console.log('🔐 global.Olm type:', typeof global.Olm);
        console.log('🔐 global.Olm value:', global.Olm);
        console.log('🔐 global keys containing "olm":', Object.keys(global).filter(k => k.toLowerCase().includes('olm')));
    }
    
    // Check window object
    console.log('🔐 window keys containing "olm":', Object.keys(window).filter(k => k.toLowerCase().includes('olm')));
    
    // Try to load Olm from the public directory directly
    console.log('🔐 Attempting to load Olm from public directory...');
    try {
        const response = await fetch('/olm.js');
        console.log('🔐 Fetch response status:', response.status);
        console.log('🔐 Fetch response content-type:', response.headers.get('content-type'));
        
        if (response.ok) {
            const olmScript = await response.text();
            console.log('🔐 Olm script length:', olmScript.length);
            console.log('🔐 Script starts with:', olmScript.substring(0, 200));
            
            // Check if it's actually JavaScript
            if (olmScript.includes('function') && olmScript.includes('Olm')) {
                console.log('🔐 Script looks like valid Olm JavaScript');
                
                // Execute the script
                const scriptElement = document.createElement('script');
                scriptElement.textContent = olmScript;
                document.head.appendChild(scriptElement);
                
                // Wait for execution
                await new Promise(resolve => setTimeout(resolve, 500));
                
                console.log('🔐 After script execution:');
                console.log('🔐 window.Olm:', typeof window.Olm);
                console.log('🔐 global.Olm:', typeof (typeof global !== 'undefined' ? global.Olm : 'undefined'));
                
                if (window.Olm && typeof window.Olm.init === 'function') {
                    console.log('🔐 Found Olm on window, initializing...');
                    await window.Olm.init();
                    
                    // Make sure it's available globally
                    if (typeof global !== 'undefined') {
                        global.Olm = window.Olm;
                    }
                    
                    console.log('🔐 ✅ Olm initialized successfully');
                    
                    // Now try to reinitialize Matrix client crypto
                    const matrixService = window.matrixService;
                    if (matrixService && matrixService.client) {
                        console.log('🔐 Reinitializing Matrix client crypto...');
                        try {
                            await matrixService.client.initCrypto();
                            console.log('🔐 ✅ Matrix crypto reinitialized:', !!matrixService.client.crypto);
                        } catch (cryptoError) {
                            console.log('🔐 ❌ Matrix crypto reinit failed:', cryptoError);
                        }
                    }
                } else {
                    console.log('🔐 ❌ Olm not found on window after script execution');
                }
            } else {
                console.log('🔐 ❌ Fetched content does not look like Olm JavaScript');
            }
        } else {
            console.log('🔐 ❌ Failed to fetch /olm.js:', response.status);
        }
    } catch (fetchError) {
        console.log('🔐 ❌ Fetch error:', fetchError);
    }
    
    console.log('🔐 DIRECT OLM FIX END');
})();
