// Debug Matrix room ownership and message routing
// Paste this in browser console to see the current state

console.log("=== MATRIX ROOM OWNERSHIP DEBUG ===");

// Check global registry
const registry = window.__gooseMatrixListenerRegistry;
if (registry) {
    console.log("\n📋 GLOBAL MATRIX LISTENER REGISTRY:");
    console.log("Registry size:", registry.size);
    
    if (registry.size > 0) {
        console.log("\nRoom ownership mapping:");
        for (const [roomId, sessionId] of registry.entries()) {
            console.log(`  ${roomId} → ${sessionId}`);
        }
    } else {
        console.log("  (empty registry)");
    }
} else {
    console.log("❌ No global registry found");
}

// Check active tabs
console.log("\n🏷️  ACTIVE TAB STATES:");
try {
    // Try to access tab context through React dev tools or global state
    const tabStates = JSON.parse(localStorage.getItem('goose-tab-state') || '[]');
    
    console.log(`Found ${tabStates.length} tabs in localStorage:`);
    tabStates.forEach((tabState, index) => {
        const tab = tabState.tab;
        console.log(`  Tab ${index + 1}:`, {
            id: tab.id,
            type: tab.type,
            sessionId: tab.sessionId,
            matrixRoomId: tab.matrixRoomId,
            title: tab.title,
            isActive: tab.isActive
        });
    });
    
    // Check for Matrix tabs
    const matrixTabs = tabStates.filter(ts => ts.tab.type === 'matrix');
    console.log(`\n📱 MATRIX TABS (${matrixTabs.length}):`);
    matrixTabs.forEach((tabState, index) => {
        const tab = tabState.tab;
        const isOwner = registry && registry.get(tab.matrixRoomId) === tab.sessionId;
        console.log(`  Matrix Tab ${index + 1}:`, {
            roomId: tab.matrixRoomId,
            sessionId: tab.sessionId,
            isOwner: isOwner,
            registeredOwner: registry ? registry.get(tab.matrixRoomId) : 'N/A'
        });
    });
    
} catch (error) {
    console.error("Failed to read tab states:", error);
}

// Check for duplicate rooms
console.log("\n🔍 DUPLICATE ROOM ANALYSIS:");
const roomCounts = {};
if (registry) {
    for (const roomId of registry.keys()) {
        roomCounts[roomId] = (roomCounts[roomId] || 0) + 1;
    }
    
    const duplicates = Object.entries(roomCounts).filter(([, count]) => count > 1);
    if (duplicates.length > 0) {
        console.log("⚠️  Found duplicate room registrations:");
        duplicates.forEach(([roomId, count]) => {
            console.log(`  ${roomId}: ${count} registrations`);
        });
    } else {
        console.log("✅ No duplicate room registrations found");
    }
}

// Function to manually clear registry
window.clearMatrixRegistry = function() {
    if (window.__gooseMatrixListenerRegistry) {
        window.__gooseMatrixListenerRegistry.clear();
        console.log("🧹 Cleared Matrix listener registry");
    }
};

console.log("\n💡 TIP: Run clearMatrixRegistry() to reset the registry if needed");
console.log("💡 TIP: Look for multiple tabs with the same matrixRoomId but different sessionIds");
