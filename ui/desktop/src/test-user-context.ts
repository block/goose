/**
 * Test file to demonstrate the user context functionality
 * This would be used in the browser console to test the features
 */

import { userContextService } from './services/UserContextService';

// Test introduction processing
export async function testUserIntroductions() {
  console.log('🧪 Testing User Context System');
  
  // Test various introduction patterns
  const testMessages = [
    "Hey goose, meet James. He's a developer who specializes in React and TypeScript.",
    "This is Sarah, our designer. She works on UI/UX and knows Figma really well.",
    "Let me introduce Mike - he's a senior engineer who handles backend systems and databases.",
    "Say hello to Lisa! She's our project manager and handles client relationships.",
    "@goose, meet Alex. Alex is a data scientist who works with Python and machine learning."
  ];

  for (const message of testMessages) {
    console.log(`\n📝 Processing: "${message}"`);
    
    try {
      const introductions = await userContextService.processIntroduction(
        message,
        'primary-user',
        'test-session-123'
      );
      
      console.log(`✅ Found ${introductions.length} introductions:`);
      introductions.forEach(intro => {
        console.log(`  - ${intro.extractedInfo?.name} (${intro.extractedInfo?.role || 'No role'})`);
        if (intro.extractedInfo?.expertise) {
          console.log(`    Expertise: ${intro.extractedInfo.expertise.join(', ')}`);
        }
      });
    } catch (error) {
      console.error('❌ Error processing introduction:', error);
    }
  }
  
  // Generate context summary
  console.log('\n📋 Generating context summary...');
  try {
    const summary = await userContextService.generateUserContextSummary('test-session-123');
    console.log('Context Summary:');
    console.log(summary);
  } catch (error) {
    console.error('❌ Error generating summary:', error);
  }
  
  // Show all user profiles
  console.log('\n👥 All user profiles:');
  try {
    const allUsers = await userContextService.getAllUserProfiles();
    console.log(`Found ${allUsers.length} users:`);
    allUsers.forEach(user => {
      console.log(`  - ${user.displayName || user.userId}`);
      console.log(`    Role: ${user.role || 'Not specified'}`);
      console.log(`    Expertise: ${user.expertise?.join(', ') || 'None specified'}`);
      console.log(`    Last seen: ${user.lastSeen || 'Never'}`);
    });
  } catch (error) {
    console.error('❌ Error getting user profiles:', error);
  }
}

// Test search functionality
export async function testUserSearch() {
  console.log('\n🔍 Testing user search...');
  
  const searchTerms = ['James', 'developer', 'React', 'Sarah'];
  
  for (const term of searchTerms) {
    try {
      const results = await userContextService.searchUsersByName(term);
      console.log(`Search for "${term}": ${results.length} results`);
      results.forEach(user => {
        console.log(`  - ${user.displayName || user.userId} (${user.role || 'No role'})`);
      });
    } catch (error) {
      console.error(`❌ Error searching for "${term}":`, error);
    }
  }
}

// Test data export/import
export async function testDataManagement() {
  console.log('\n💾 Testing data export/import...');
  
  try {
    // Export data
    const exported = await userContextService.exportData();
    console.log('✅ Data exported successfully');
    console.log(`Profiles: ${Object.keys(exported.profiles).length}`);
    console.log(`Introductions: ${exported.introductions.length}`);
    
    // Clear data
    await userContextService.clearAllData();
    console.log('✅ Data cleared');
    
    // Verify it's empty
    const emptyUsers = await userContextService.getAllUserProfiles();
    console.log(`Users after clear: ${emptyUsers.length}`);
    
    // Import data back
    await userContextService.importData(exported);
    console.log('✅ Data imported successfully');
    
    // Verify it's restored
    const restoredUsers = await userContextService.getAllUserProfiles();
    console.log(`Users after import: ${restoredUsers.length}`);
    
  } catch (error) {
    console.error('❌ Error in data management test:', error);
  }
}

// Run all tests
export async function runAllTests() {
  console.log('🚀 Starting User Context System Tests\n');
  
  await testUserIntroductions();
  await testUserSearch();
  await testDataManagement();
  
  console.log('\n✅ All tests completed!');
}

// Make functions available globally for browser console testing
if (typeof window !== 'undefined') {
  (window as any).testUserContext = {
    runAllTests,
    testUserIntroductions,
    testUserSearch,
    testDataManagement,
    userContextService,
  };
  
  console.log('🧪 User Context tests available at window.testUserContext');
  console.log('Run window.testUserContext.runAllTests() to test everything');
}
