// Debug script to test direct Olm loading
(async () => {
    console.log('🔐 DIRECT OLM TEST START');
    
    // Test if we can fetch the olm.js file
    try {
        console.log('🔐 Testing fetch of ./olm.js...');
        const response = await fetch('./olm.js');
        console.log('🔐 Fetch response status:', response.status);
        console.log('🔐 Fetch response headers:', Object.fromEntries(response.headers.entries()));
        
        if (response.ok) {
            const olmScript = await response.text();
            console.log('🔐 Successfully fetched Olm script, length:', olmScript.length);
            console.log('🔐 Script starts with:', olmScript.substring(0, 100));
            
            // Try to execute the script
            console.log('🔐 Attempting to execute Olm script...');
            const scriptElement = document.createElement('script');
            scriptElement.textContent = olmScript;
            document.head.appendChild(scriptElement);
            
            // Wait for execution
            await new Promise(resolve => setTimeout(resolve, 500));
            
            console.log('🔐 After script execution:');
            console.log('🔐 window.Olm:', typeof window.Olm);
            console.log('🔐 global.Olm:', typeof (typeof global !== 'undefined' ? global.Olm : 'global undefined'));
            
            if (window.Olm) {
                console.log('🔐 Olm object properties:', Object.keys(window.Olm));
                console.log('🔐 Olm.init type:', typeof window.Olm.init);
                
                if (typeof window.Olm.init === 'function') {
                    console.log('🔐 Calling Olm.init()...');
                    await window.Olm.init();
                    console.log('🔐 ✅ Olm.init() completed successfully');
                } else {
                    console.log('🔐 ℹ️ No init method found on Olm object');
                }
            }
        } else {
            console.log('🔐 ❌ Failed to fetch olm.js:', response.status, response.statusText);
        }
    } catch (error) {
        console.log('🔐 ❌ Error during direct Olm test:', error);
    }
    
    console.log('🔐 DIRECT OLM TEST END');
})();
