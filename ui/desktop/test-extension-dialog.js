// Test script for the extension confirmation dialog
// This simulates clicking the "Add Extension" menu item

const { ipcRenderer } = require('electron');

// Create a fake extension URL
const extensionUrl = 'goose://extension?cmd=npx&arg=-y&arg=tavily-mcp&id=tavily&name=Tavily%20Web%20Search&description=Web%20search%20capabilities%20powered%20by%20Tavily&env=TAVILY_API_KEY%3DAPI%20key%20for%20Tavily%20web%20search%20service';

// Send the add-extension event
ipcRenderer.send('add-extension', extensionUrl);