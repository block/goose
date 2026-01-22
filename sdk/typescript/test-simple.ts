/**
 * Simple test script for goosed SDK
 */

import { GoosedClient } from './src/index.js';

async function testGoosedSDK() {
    const client = new GoosedClient({
        baseUrl: 'http://127.0.0.1:3000',
        secretKey: 'test',
    });

    try {
        // 1. Check server status
        console.log('1. Checking server status...');
        const status = await client.status();
        console.log(`   Status: ${status}`);

        // 2. Get system info
        console.log('\n2. Getting system info...');
        const info = await client.systemInfo();
        console.log(`   Version: ${info.app_version}`);
        console.log(`   Provider: ${info.provider}`);
        console.log(`   Model: ${info.model}`);
        console.log(`   OS: ${info.os} ${info.os_version}`);

        // 3. Create a session
        console.log('\n3. Creating session...');
        const session = await client.startSession('/tmp/sdk-test');
        console.log(`   Session ID: ${session.id}`);

        // 4. Resume session (load extensions)
        console.log('\n4. Resuming session...');
        const { session: resumed, extensionResults } = await client.resumeSession(session.id);
        console.log(`   Session resumed: ${resumed.id}`);
        console.log(`   Extensions loaded: ${extensionResults.length}`);
        for (const ext of extensionResults) {
            console.log(`   - ${ext.name}: ${ext.success ? 'OK' : 'FAILED'}`);
        }

        // 5. Get available tools
        console.log('\n5. Getting available tools...');
        const tools = await client.getTools(session.id);
        console.log(`   Available tools: ${tools.length}`);
        console.log(`   First 5 tools:`);
        for (const tool of tools.slice(0, 5)) {
            console.log(`   - ${tool.name}: ${tool.description.slice(0, 60)}...`);
        }

        // 6. Call a tool
        console.log('\n6. Testing tool call...');
        const toolResult = await client.callTool(session.id, 'todo__todo_write', {
            content: 'SDK Test TODO',
        });
        console.log(`   Tool call success: ${!toolResult.is_error}`);

        // 7. Test chat
        console.log('\n7. Testing chat...');
        console.log('   Sending: "Hello! Please respond with a short greeting."');
        const response = await client.chat(session.id, 'Hello! Please respond with a short greeting.');
        console.log(`   Response: ${response.slice(0, 200)}${response.length > 200 ? '...' : ''}`);

        // 8. Test streaming chat
        console.log('\n8. Testing streaming chat...');
        console.log('   Sending: "Count from 1 to 5"');
        for await (const event of client.sendMessage(session.id, 'Count from 1 to 5')) {
            if (event.type === 'Message' && event.message) {
                const content = event.message.content as Array<{ type: string; text?: string }>;
                for (const c of content) {
                    if (c.type === 'text' && c.text) {
                        process.stdout.write(c.text);
                    }
                }
            } else if (event.type === 'Finish') {
                console.log(`\n   Tokens used: ${event.token_state?.totalTokens || 'N/A'}`);
            } else if (event.type === 'Error') {
                console.error(`   Error: ${event.error}`);
            }
        }

        // 9. List sessions
        console.log('\n9. Listing sessions...');
        const sessions = await client.listSessions();
        console.log(`   Total sessions: ${sessions.length}`);

        // 10. Export session
        console.log('\n10. Exporting session...');
        const exported = await client.exportSession(session.id);
        console.log(`   Exported data length: ${exported.length} bytes`);

        // 11. Clean up
        console.log('\n11. Cleaning up...');
        await client.stopSession(session.id);
        await client.deleteSession(session.id);
        console.log('   Session deleted');

        console.log('\n✅ All tests passed!');
    } catch (error) {
        console.error('\n❌ Test failed:', error);
        if (error instanceof Error) {
            console.error('   Message:', error.message);
            console.error('   Stack:', error.stack);
        }
        process.exit(1);
    }
}

testGoosedSDK();
