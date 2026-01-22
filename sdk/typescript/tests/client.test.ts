/**
 * Tests for goosed-sdk TypeScript
 */

import { test, describe, before, after } from 'node:test';
import assert from 'node:assert';
import { GoosedClient } from '../src/index.js';

const BASE_URL = process.env.GOOSED_BASE_URL ?? 'http://127.0.0.1:3002';
const SECRET_KEY = process.env.GOOSED_SECRET_KEY ?? 'test-secret';

describe('GoosedClient', () => {
    let client: GoosedClient;

    before(() => {
        client = new GoosedClient({ baseUrl: BASE_URL, secretKey: SECRET_KEY });
    });

    describe('Status APIs', () => {
        test('status() returns ok', async () => {
            const result = await client.status();
            assert.strictEqual(result, 'ok');
        });

        test('systemInfo() returns version', async () => {
            const info = await client.systemInfo();
            assert.ok(info.app_version);
            assert.ok(info.provider);
            assert.ok(info.model);
        });
    });

    describe('Session Management', () => {
        test('create and delete session', async () => {
            const session = await client.startSession('/tmp/ts-sdk-test');
            assert.ok(session.id);
            assert.strictEqual(session.working_dir, '/tmp/ts-sdk-test');

            await client.deleteSession(session.id);
        });

        test('list sessions', async () => {
            const sessions = await client.listSessions();
            assert.ok(Array.isArray(sessions));
        });

        test('resume session', async () => {
            const session = await client.startSession('/tmp/ts-sdk-test-resume');

            try {
                const { session: resumed, extensionResults } = await client.resumeSession(session.id);
                assert.strictEqual(resumed.id, session.id);
                assert.ok(Array.isArray(extensionResults));
            } finally {
                await client.deleteSession(session.id);
            }
        });

        test('update session name', async () => {
            const session = await client.startSession('/tmp/ts-sdk-test-name');

            try {
                await client.updateSessionName(session.id, 'TS Test Session');
                const updated = await client.getSession(session.id);
                assert.strictEqual(updated.name, 'TS Test Session');
            } finally {
                await client.deleteSession(session.id);
            }
        });
    });

    describe('Agent APIs', () => {
        test('get tools', async () => {
            const session = await client.startSession('/tmp/ts-sdk-test-tools');

            try {
                await client.resumeSession(session.id);
                const tools = await client.getTools(session.id);
                assert.ok(Array.isArray(tools));
                assert.ok(tools.length > 0);
                assert.ok(tools[0].name);
            } finally {
                await client.stopSession(session.id);
                await client.deleteSession(session.id);
            }
        });

        test('call tool', async () => {
            const session = await client.startSession('/tmp/ts-sdk-test-call-tool');

            try {
                await client.resumeSession(session.id);
                const result = await client.callTool(session.id, 'todo__todo_write', {
                    content: 'TS SDK Test TODO',
                });
                assert.strictEqual(result.is_error, false);
                assert.ok(result.content.length > 0);
            } finally {
                await client.stopSession(session.id);
                await client.deleteSession(session.id);
            }
        });

        test('restart session', async () => {
            const session = await client.startSession('/tmp/ts-sdk-test-restart');

            try {
                await client.resumeSession(session.id);
                const results = await client.restartSession(session.id);
                assert.ok(Array.isArray(results));
            } finally {
                await client.stopSession(session.id);
                await client.deleteSession(session.id);
            }
        });
    });

    describe('Chat APIs', () => {
        test('send message stream', async () => {
            const session = await client.startSession('/tmp/ts-sdk-test-chat');

            try {
                await client.resumeSession(session.id);

                const events: Array<{ type: string }> = [];
                for await (const event of client.sendMessage(session.id, 'Say hello')) {
                    events.push(event);
                }

                assert.ok(events.length > 0);
                const eventTypes = events.map((e) => e.type);
                assert.ok(eventTypes.includes('Finish'));
            } finally {
                await client.stopSession(session.id);
                await client.deleteSession(session.id);
            }
        });
    });

    describe('Export APIs', () => {
        test('export session', async () => {
            const session = await client.startSession('/tmp/ts-sdk-test-export');

            try {
                const exported = await client.exportSession(session.id);
                assert.ok(typeof exported === 'string');
                assert.ok(exported.includes(session.id));
            } finally {
                await client.deleteSession(session.id);
            }
        });
    });


});
