// Helper function to show test name overlay
async function showTestName(mainWindow: any, testName: string, providerName?: string) {
  await mainWindow.evaluate(({ name, provider }: { name: string, provider?: string }) => {
    // Remove any existing overlay
    const existing = document.getElementById('test-overlay');
    if (existing) existing.remove();

    // Create new overlay
    const overlay = document.createElement('div');
    overlay.id = 'test-overlay';
    overlay.style.cssText = `
      position: fixed;
      top: 50%;
      left: 50%;
      transform: translate(-50%, -50%);
      background: rgba(0, 0, 0, 0.8);
      color: white;
      padding: 12px 16px;
      border-radius: 6px;
      font-family: monospace;
      font-size: 14px;
      z-index: 2147483647; // maximum z-index integer value
      pointer-events: none !important;
      text-align: center;
      max-width: 80%;
      white-space: pre-wrap;
      box-shadow: 0 0 10px rgba(0, 0, 0, 0.5);
      user-select: none;
      -webkit-user-select: none;
    `;
    
    const testText = `Running: ${name}`;
    const providerText = provider ? `\nProvider: ${provider}` : '';
    overlay.textContent = testText + providerText;

    // Insert at the beginning of <html> to ensure it's above everything
    document.documentElement.insertBefore(overlay, document.documentElement.firstChild);

    // Force a repaint to ensure the overlay is visible
    overlay.getBoundingClientRect();
  }, { name: testName, provider: providerName });
}

// Helper function to clear test name overlay
async function clearTestName(mainWindow: any) {
  await mainWindow.evaluate(() => {
    const overlay = document.getElementById('test-overlay');
    if (overlay) overlay.remove();
  });
}

export { showTestName, clearTestName };
