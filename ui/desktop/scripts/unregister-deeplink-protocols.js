#!/usr/bin/env node

/**
 * Script to unregister ALL goose:// protocol handlers
 * Usage: node scripts/unregister-deeplink-protocols.js
 */

const { execSync } = require('child_process');

const PROTOCOL = 'goose';

function unregisterAllProtocolHandlers() {
  console.log('Unregistering ALL goose:// protocol handlers...');
  
  try {
    // Get all registered Goose apps
    console.log('Finding all registered Goose applications...');
    const lsregisterOutput = execSync(`/System/Library/Frameworks/CoreServices.framework/Versions/A/Frameworks/LaunchServices.framework/Versions/A/Support/lsregister -dump | grep -B 10 -A 10 "claimed schemes:.*${PROTOCOL}:"`, { encoding: 'utf8' });
    
    // Extract app paths from the output
    const pathMatches = lsregisterOutput.match(/path:\s+(.+\.app)/g);
    const uniquePaths = new Set();
    
    if (pathMatches) {
      pathMatches.forEach(match => {
        const path = match.replace(/path:\s+/, '').trim();
        if (path.includes('Goose') || path.includes('goose')) {
          uniquePaths.add(path);
        }
      });
    }
    
    console.log(`Found ${uniquePaths.size} Goose app(s) to unregister:`);
    uniquePaths.forEach(path => console.log(`  - ${path}`));
    
    // Unregister each app
    let unregisteredCount = 0;
    uniquePaths.forEach(appPath => {
      try {
        console.log(`Unregistering: ${appPath}`);
        execSync(`/System/Library/Frameworks/CoreServices.framework/Versions/A/Frameworks/LaunchServices.framework/Versions/A/Support/lsregister -u "${appPath}"`, { stdio: 'ignore' });
        unregisteredCount++;
      } catch (error) {
        console.log(`  Warning: Could not unregister ${appPath} (may already be unregistered)`);
      }
    });
    
    // Also try to unregister by bundle identifier
    console.log('\nUnregistering by bundle identifier...');
    const bundleIds = [
      'com.electron.goose',
      'com.block.goose',
      'com.block.goose.dev'
    ];
    
    bundleIds.forEach(bundleId => {
      try {
        console.log(`Unregistering bundle: ${bundleId}`);
        execSync(`/System/Library/Frameworks/CoreServices.framework/Versions/A/Frameworks/LaunchServices.framework/Versions/A/Support/lsregister -u "${bundleId}"`, { stdio: 'ignore' });
      } catch (error) {
        // Ignore errors for bundle IDs that don't exist
      }
    });
    
    // Clean up temporary files
    console.log('\nCleaning up temporary files...');
    try {
      execSync('rm -rf /tmp/GooseDevApp.app', { stdio: 'ignore' });
    } catch (error) {
      // Ignore cleanup errors
    }
    
    // Force Launch Services to rebuild its database
    console.log('Rebuilding Launch Services database...');
    try {
      execSync('/System/Library/Frameworks/CoreServices.framework/Versions/A/Frameworks/LaunchServices.framework/Versions/A/Support/lsregister -kill -r -domain local -domain system -domain user', { stdio: 'ignore' });
    } catch (error) {
      console.log('Warning: Could not rebuild Launch Services database');
    }
    
    console.log(`\n✅ Successfully processed ${unregisteredCount} Goose applications`);
    console.log('All goose:// protocol handlers have been unregistered.');
    console.log('\nNote: You may need to restart your system for changes to take full effect.');
    
  } catch (error) {
    console.error('Error during unregistration:', error.message);
    console.log('\nManual cleanup options:');
    console.log('1. Use Activity Monitor to quit all Goose processes');
    console.log('2. Delete Goose apps from Applications folder');
    console.log('3. Run: sudo /System/Library/Frameworks/CoreServices.framework/Versions/A/Frameworks/LaunchServices.framework/Versions/A/Support/lsregister -kill -r -domain local -domain system -domain user');
  }
}

// Run the unregistration immediately
unregisterAllProtocolHandlers();
