import { describe, it, expect, beforeEach } from "vitest";
import { ClientSideConnection, AgentSideConnection, PROTOCOL_VERSION, ndJsonStream, } from "./acp.js";
describe("Connection", () => {
    let clientToAgent;
    let agentToClient;
    beforeEach(() => {
        clientToAgent = new TransformStream();
        agentToClient = new TransformStream();
    });
    it("handles errors in bidirectional communication", async () => {
        // Create client that throws errors
        class TestClient {
            async writeTextFile(_) {
                throw new Error("Write failed");
            }
            async readTextFile(_) {
                throw new Error("Read failed");
            }
            async requestPermission(_) {
                throw new Error("Permission denied");
            }
            async sessionUpdate(_) {
                // no-op
            }
        }
        // Create agent that throws errors
        class TestAgent {
            async initialize(_) {
                throw new Error("Failed to initialize");
            }
            async newSession(_) {
                throw new Error("Failed to create session");
            }
            async loadSession(_) {
                throw new Error("Failed to load session");
            }
            async authenticate(_) {
                throw new Error("Authentication failed");
            }
            async prompt(_) {
                throw new Error("Prompt failed");
            }
            async cancel(_) {
                // no-op
            }
        }
        // Set up connections
        const agentConnection = new ClientSideConnection(() => new TestClient(), ndJsonStream(clientToAgent.writable, agentToClient.readable));
        const clientConnection = new AgentSideConnection(() => new TestAgent(), ndJsonStream(agentToClient.writable, clientToAgent.readable));
        // Test error handling in client->agent direction
        await expect(clientConnection.writeTextFile({
            path: "/test.txt",
            content: "test",
            sessionId: "test-session",
        })).rejects.toThrow();
        // Test error handling in agent->client direction
        await expect(agentConnection.newSession({
            cwd: "/test",
            mcpServers: [],
        })).rejects.toThrow();
    });
    it("handles concurrent requests", async () => {
        let requestCount = 0;
        // Create client
        class TestClient {
            async writeTextFile(_) {
                requestCount++;
                const currentCount = requestCount;
                await new Promise((resolve) => setTimeout(resolve, 40));
                console.log(`Write request ${currentCount} completed`);
                return {};
            }
            async readTextFile(params) {
                return { content: `Content of ${params.path}` };
            }
            async requestPermission(_) {
                return {
                    outcome: {
                        outcome: "selected",
                        optionId: "allow",
                    },
                };
            }
            async sessionUpdate(_) {
                // no-op
            }
        }
        // Create agent
        class TestAgent {
            async initialize(_) {
                return {
                    protocolVersion: 1,
                    agentCapabilities: { loadSession: false },
                    authMethods: [],
                };
            }
            async newSession(_) {
                return {
                    sessionId: "test-session",
                };
            }
            async loadSession(_) {
                return {};
            }
            async authenticate(_) {
                // no-op
            }
            async prompt(_) {
                return { stopReason: "end_turn" };
            }
            async cancel(_) {
                // no-op
            }
        }
        // Set up connections
        new ClientSideConnection(() => new TestClient(), ndJsonStream(clientToAgent.writable, agentToClient.readable));
        const clientConnection = new AgentSideConnection(() => new TestAgent(), ndJsonStream(agentToClient.writable, clientToAgent.readable));
        // Send multiple concurrent requests
        const promises = [
            clientConnection.writeTextFile({
                path: "/file1.txt",
                content: "content1",
                sessionId: "session1",
            }),
            clientConnection.writeTextFile({
                path: "/file2.txt",
                content: "content2",
                sessionId: "session1",
            }),
            clientConnection.writeTextFile({
                path: "/file3.txt",
                content: "content3",
                sessionId: "session1",
            }),
        ];
        const results = await Promise.all(promises);
        // Verify all requests completed successfully
        expect(results).toHaveLength(3);
        expect(results[0]).toEqual({});
        expect(results[1]).toEqual({});
        expect(results[2]).toEqual({});
        expect(requestCount).toBe(3);
    });
    it("handles message ordering correctly", async () => {
        const messageLog = [];
        // Create client
        class TestClient {
            async writeTextFile(params) {
                messageLog.push(`writeTextFile called: ${params.path}`);
                return {};
            }
            async readTextFile(params) {
                messageLog.push(`readTextFile called: ${params.path}`);
                return { content: "test content" };
            }
            async requestPermission(params) {
                messageLog.push(`requestPermission called: ${params.toolCall.title}`);
                return {
                    outcome: {
                        outcome: "selected",
                        optionId: "allow",
                    },
                };
            }
            async sessionUpdate(_params) {
                messageLog.push("sessionUpdate called");
            }
        }
        // Create agent
        class TestAgent {
            async initialize(_) {
                return {
                    protocolVersion: 1,
                    agentCapabilities: { loadSession: false },
                    authMethods: [],
                };
            }
            async newSession(request) {
                messageLog.push(`newSession called: ${request.cwd}`);
                return {
                    sessionId: "test-session",
                };
            }
            async loadSession(params) {
                messageLog.push(`loadSession called: ${params.sessionId}`);
                return {};
            }
            async authenticate(params) {
                messageLog.push(`authenticate called: ${params.methodId}`);
            }
            async prompt(params) {
                messageLog.push(`prompt called: ${params.sessionId}`);
                return { stopReason: "end_turn" };
            }
            async cancel(params) {
                messageLog.push(`cancelled called: ${params.sessionId}`);
            }
        }
        // Set up connections
        const agentConnection = new ClientSideConnection(() => new TestClient(), ndJsonStream(clientToAgent.writable, agentToClient.readable));
        const clientConnection = new AgentSideConnection(() => new TestAgent(), ndJsonStream(agentToClient.writable, clientToAgent.readable));
        // Send requests in specific order
        await agentConnection.newSession({
            cwd: "/test",
            mcpServers: [],
        });
        await clientConnection.writeTextFile({
            path: "/test.txt",
            content: "test",
            sessionId: "test-session",
        });
        await clientConnection.readTextFile({
            path: "/test.txt",
            sessionId: "test-session",
        });
        await clientConnection.requestPermission({
            sessionId: "test-session",
            toolCall: {
                title: "Execute command",
                kind: "execute",
                status: "pending",
                toolCallId: "tool-123",
                content: [
                    {
                        type: "content",
                        content: {
                            type: "text",
                            text: "ls -la",
                        },
                    },
                ],
            },
            options: [
                {
                    kind: "allow_once",
                    name: "Allow",
                    optionId: "allow",
                },
                {
                    kind: "reject_once",
                    name: "Reject",
                    optionId: "reject",
                },
            ],
        });
        // Verify order
        expect(messageLog).toEqual([
            "newSession called: /test",
            "writeTextFile called: /test.txt",
            "readTextFile called: /test.txt",
            "requestPermission called: Execute command",
        ]);
    });
    it("handles notifications correctly", async () => {
        const notificationLog = [];
        // Create client
        class TestClient {
            async writeTextFile(_) {
                return {};
            }
            async readTextFile(_) {
                return { content: "test" };
            }
            async requestPermission(_) {
                return {
                    outcome: {
                        outcome: "selected",
                        optionId: "allow",
                    },
                };
            }
            async sessionUpdate(notification) {
                if (notification.update &&
                    "sessionUpdate" in notification.update &&
                    notification.update.sessionUpdate === "agent_message_chunk") {
                    notificationLog.push(`agent message: ${notification.update.content.text}`);
                }
            }
        }
        // Create agent
        class TestAgent {
            async initialize(_) {
                return {
                    protocolVersion: 1,
                    agentCapabilities: { loadSession: false },
                    authMethods: [],
                };
            }
            async newSession(_) {
                return {
                    sessionId: "test-session",
                };
            }
            async loadSession(_) {
                return {};
            }
            async authenticate(_) {
                // no-op
            }
            async prompt(_) {
                return { stopReason: "end_turn" };
            }
            async cancel(params) {
                notificationLog.push(`cancelled: ${params.sessionId}`);
            }
        }
        // Create shared instances
        const testClient = () => new TestClient();
        const testAgent = () => new TestAgent();
        // Set up connections
        const agentConnection = new ClientSideConnection(testClient, ndJsonStream(clientToAgent.writable, agentToClient.readable));
        const clientConnection = new AgentSideConnection(testAgent, ndJsonStream(agentToClient.writable, clientToAgent.readable));
        // Send notifications
        await clientConnection.sessionUpdate({
            sessionId: "test-session",
            update: {
                sessionUpdate: "agent_message_chunk",
                content: {
                    type: "text",
                    text: "Hello from agent",
                },
            },
        });
        await agentConnection.cancel({
            sessionId: "test-session",
        });
        // Wait a bit for async handlers
        await new Promise((resolve) => setTimeout(resolve, 50));
        // Verify notifications were received
        expect(notificationLog).toContain("agent message: Hello from agent");
        expect(notificationLog).toContain("cancelled: test-session");
    });
    it("handles initialize method", async () => {
        // Create client
        class TestClient {
            async writeTextFile(_) {
                return {};
            }
            async readTextFile(_) {
                return { content: "test" };
            }
            async requestPermission(_) {
                return {
                    outcome: {
                        outcome: "selected",
                        optionId: "allow",
                    },
                };
            }
            async sessionUpdate(_) {
                // no-op
            }
        }
        // Create agent
        class TestAgent {
            async initialize(params) {
                return {
                    protocolVersion: params.protocolVersion,
                    agentCapabilities: { loadSession: true },
                    authMethods: [
                        {
                            id: "oauth",
                            name: "OAuth",
                            description: "Authenticate with OAuth",
                        },
                    ],
                };
            }
            async newSession(_) {
                return { sessionId: "test-session" };
            }
            async loadSession(_) {
                return {};
            }
            async authenticate(_) {
                // no-op
            }
            async prompt(_) {
                return { stopReason: "end_turn" };
            }
            async cancel(_) {
                // no-op
            }
        }
        // Set up connections
        const agentConnection = new ClientSideConnection(() => new TestClient(), ndJsonStream(clientToAgent.writable, agentToClient.readable));
        new AgentSideConnection(() => new TestAgent(), ndJsonStream(agentToClient.writable, clientToAgent.readable));
        // Test initialize request
        const response = await agentConnection.initialize({
            protocolVersion: PROTOCOL_VERSION,
            clientCapabilities: {
                fs: {
                    readTextFile: false,
                    writeTextFile: false,
                },
            },
        });
        expect(response.protocolVersion).toBe(PROTOCOL_VERSION);
        expect(response.agentCapabilities?.loadSession).toBe(true);
        expect(response.authMethods).toHaveLength(1);
        expect(response.authMethods?.[0].id).toBe("oauth");
    });
    it("handles extension methods and notifications", async () => {
        const extensionLog = [];
        // Create client with extension method support
        class TestClient {
            async writeTextFile(_) {
                return {};
            }
            async readTextFile(_) {
                return { content: "test" };
            }
            async requestPermission(_) {
                return {
                    outcome: {
                        outcome: "selected",
                        optionId: "allow",
                    },
                };
            }
            async sessionUpdate(_) {
                // no-op
            }
            async extMethod(method, params) {
                if (method === "example.com/ping") {
                    return { response: "pong", params };
                }
                throw new Error(`Unknown method: ${method}`);
            }
            async extNotification(method, _params) {
                extensionLog.push(`client extNotification: ${method}`);
            }
        }
        // Create agent with extension method support
        class TestAgent {
            async initialize(_) {
                return {
                    protocolVersion: PROTOCOL_VERSION,
                    agentCapabilities: { loadSession: false },
                };
            }
            async newSession(_) {
                return { sessionId: "test-session" };
            }
            async authenticate(_) {
                // no-op
            }
            async prompt(_) {
                return { stopReason: "end_turn" };
            }
            async cancel(_) {
                // no-op
            }
            async extMethod(method, params) {
                if (method === "example.com/echo") {
                    return { echo: params };
                }
                throw new Error(`Unknown method: ${method}`);
            }
            async extNotification(method, _params) {
                extensionLog.push(`agent extNotification: ${method}`);
            }
        }
        // Set up connections
        const agentConnection = new ClientSideConnection(() => new TestClient(), ndJsonStream(clientToAgent.writable, agentToClient.readable));
        const clientConnection = new AgentSideConnection(() => new TestAgent(), ndJsonStream(agentToClient.writable, clientToAgent.readable));
        // Test agent calling client extension method
        const clientResponse = await clientConnection.extMethod("example.com/ping", {
            data: "test",
        });
        expect(clientResponse).toEqual({
            response: "pong",
            params: { data: "test" },
        });
        // Test client calling agent extension method
        const agentResponse = await agentConnection.extMethod("example.com/echo", {
            message: "hello",
        });
        expect(agentResponse).toEqual({ echo: { message: "hello" } });
        // Test extension notifications
        await clientConnection.extNotification("example.com/client/notify", {
            info: "client notification",
        });
        await agentConnection.extNotification("example.com/agent/notify", {
            info: "agent notification",
        });
        // Wait a bit for async handlers
        await new Promise((resolve) => setTimeout(resolve, 50));
        // Verify notifications were logged
        expect(extensionLog).toContain("client extNotification: example.com/client/notify");
        expect(extensionLog).toContain("agent extNotification: example.com/agent/notify");
    });
    it("handles optional extension methods correctly", async () => {
        // Create client WITHOUT extension methods
        class TestClientWithoutExtensions {
            async writeTextFile(_) {
                return {};
            }
            async readTextFile(_) {
                return { content: "test" };
            }
            async requestPermission(_) {
                return {
                    outcome: {
                        outcome: "selected",
                        optionId: "allow",
                    },
                };
            }
            async sessionUpdate(_) {
                // no-op
            }
        }
        // Create agent WITHOUT extension methods
        class TestAgentWithoutExtensions {
            async initialize(_) {
                return {
                    protocolVersion: PROTOCOL_VERSION,
                    agentCapabilities: { loadSession: false },
                };
            }
            async newSession(_) {
                return { sessionId: "test-session" };
            }
            async authenticate(_) {
                // no-op
            }
            async prompt(_) {
                return { stopReason: "end_turn" };
            }
            async cancel(_) {
                // no-op
            }
        }
        // Set up connections
        const agentConnection = new ClientSideConnection(() => new TestClientWithoutExtensions(), ndJsonStream(clientToAgent.writable, agentToClient.readable));
        const clientConnection = new AgentSideConnection(() => new TestAgentWithoutExtensions(), ndJsonStream(agentToClient.writable, clientToAgent.readable));
        // Test that calling extension methods on connections without them throws method not found
        try {
            await clientConnection.extMethod("_example.com/ping", { data: "test" });
            expect.fail("Should have thrown method not found error");
        }
        catch (error) {
            expect(error.code).toBe(-32601); // Method not found
            expect(error.data.method).toBe("_example.com/ping");
        }
        try {
            await agentConnection.extMethod("_example.com/echo", {
                message: "hello",
            });
            expect.fail("Should have thrown method not found error");
        }
        catch (error) {
            expect(error.code).toBe(-32601); // Method not found
            expect(error.data.method).toBe("_example.com/echo");
        }
        // Notifications should be ignored when not implemented (no error thrown)
        await clientConnection.extNotification("example.com/notify", {
            info: "test",
        });
        await agentConnection.extNotification("example.com/notify", {
            info: "test",
        });
    });
    it("resolves closed promise when stream ends", async () => {
        const closeLog = [];
        // Create simple client and agent
        class TestClient {
            async writeTextFile(_) {
                return {};
            }
            async readTextFile(_) {
                return { content: "test" };
            }
            async requestPermission(_) {
                return {
                    outcome: {
                        outcome: "selected",
                        optionId: "allow",
                    },
                };
            }
            async sessionUpdate(_) {
                // no-op
            }
        }
        class TestAgent {
            async initialize(_) {
                return {
                    protocolVersion: PROTOCOL_VERSION,
                    agentCapabilities: { loadSession: false },
                };
            }
            async newSession(_) {
                return { sessionId: "test-session" };
            }
            async authenticate(_) {
                // no-op
            }
            async prompt(_) {
                return { stopReason: "end_turn" };
            }
            async cancel(_) {
                // no-op
            }
        }
        // Set up connections
        const agentConnection = new ClientSideConnection(() => new TestClient(), ndJsonStream(clientToAgent.writable, agentToClient.readable));
        const clientConnection = new AgentSideConnection(() => new TestAgent(), ndJsonStream(agentToClient.writable, clientToAgent.readable));
        // Listen for close via signal
        agentConnection.signal.addEventListener("abort", () => {
            closeLog.push("agent connection closed (signal)");
        });
        clientConnection.signal.addEventListener("abort", () => {
            closeLog.push("client connection closed (signal)");
        });
        // Verify connections are not closed yet
        expect(agentConnection.signal.aborted).toBe(false);
        expect(clientConnection.signal.aborted).toBe(false);
        expect(closeLog).toHaveLength(0);
        // Close the streams by closing the writable ends
        await clientToAgent.writable.close();
        await agentToClient.writable.close();
        // Wait for closed promises to resolve
        await agentConnection.closed;
        await clientConnection.closed;
        // Verify connections are now closed
        expect(agentConnection.signal.aborted).toBe(true);
        expect(clientConnection.signal.aborted).toBe(true);
        expect(closeLog).toContain("agent connection closed (signal)");
        expect(closeLog).toContain("client connection closed (signal)");
    });
    it("supports removing signal event listeners", async () => {
        const closeLog = [];
        // Create simple client and agent
        class TestClient {
            async writeTextFile(_) {
                return {};
            }
            async readTextFile(_) {
                return { content: "test" };
            }
            async requestPermission(_) {
                return {
                    outcome: {
                        outcome: "selected",
                        optionId: "allow",
                    },
                };
            }
            async sessionUpdate(_) {
                // no-op
            }
        }
        class TestAgent {
            async initialize(_) {
                return {
                    protocolVersion: PROTOCOL_VERSION,
                    agentCapabilities: { loadSession: false },
                };
            }
            async newSession(_) {
                return { sessionId: "test-session" };
            }
            async authenticate(_) {
                // no-op
            }
            async prompt(_) {
                return { stopReason: "end_turn" };
            }
            async cancel(_) {
                // no-op
            }
        }
        // Set up connections
        const agentConnection = new ClientSideConnection(() => new TestClient(), ndJsonStream(clientToAgent.writable, agentToClient.readable));
        new AgentSideConnection(() => new TestAgent(), ndJsonStream(agentToClient.writable, clientToAgent.readable));
        // Register and then remove a listener
        const listener = () => {
            closeLog.push("this should not be called");
        };
        agentConnection.signal.addEventListener("abort", listener);
        agentConnection.signal.removeEventListener("abort", listener);
        // Register another listener that should be called
        agentConnection.signal.addEventListener("abort", () => {
            closeLog.push("agent connection closed");
        });
        // Close the streams
        await clientToAgent.writable.close();
        await agentToClient.writable.close();
        // Wait for closed promise
        await agentConnection.closed;
        // Verify only the non-removed listener was called
        expect(closeLog).toEqual(["agent connection closed"]);
        expect(closeLog).not.toContain("this should not be called");
    });
    it("handles methods returning response objects with _meta or void", async () => {
        // Create client that returns both response objects and void
        class TestClient {
            async writeTextFile(_params) {
                // Return response object with _meta
                return {
                    _meta: {
                        timestamp: new Date().toISOString(),
                        version: "1.0.0",
                    },
                };
            }
            async readTextFile(_params) {
                return {
                    content: "test content",
                    _meta: {
                        encoding: "utf-8",
                    },
                };
            }
            async requestPermission(_params) {
                return {
                    outcome: {
                        outcome: "selected",
                        optionId: "allow",
                    },
                    _meta: {
                        userId: "test-user",
                    },
                };
            }
            async sessionUpdate(_params) {
                // Returns void
            }
        }
        // Create agent that returns both response objects and void
        class TestAgent {
            async initialize(params) {
                return {
                    protocolVersion: params.protocolVersion,
                    agentCapabilities: { loadSession: true },
                    _meta: {
                        agentVersion: "2.0.0",
                    },
                };
            }
            async newSession(_params) {
                return {
                    sessionId: "test-session",
                    _meta: {
                        sessionType: "ephemeral",
                    },
                };
            }
            async loadSession(_params) {
                // Test returning minimal response
                return {};
            }
            async authenticate(params) {
                if (params.methodId === "none") {
                    // Test returning void
                    return;
                }
                // Test returning response with _meta
                return {
                    _meta: {
                        authenticated: true,
                        method: params.methodId,
                    },
                };
            }
            async prompt(_params) {
                return { stopReason: "end_turn" };
            }
            async cancel(_params) {
                // Returns void
            }
        }
        // Set up connections
        const agentConnection = new ClientSideConnection(() => new TestClient(), ndJsonStream(clientToAgent.writable, agentToClient.readable));
        const clientConnection = new AgentSideConnection(() => new TestAgent(), ndJsonStream(agentToClient.writable, clientToAgent.readable));
        // Test writeTextFile returns response with _meta
        const writeResponse = await clientConnection.writeTextFile({
            path: "/test.txt",
            content: "test",
            sessionId: "test-session",
        });
        expect(writeResponse).toEqual({
            _meta: {
                timestamp: expect.any(String),
                version: "1.0.0",
            },
        });
        // Test readTextFile returns response with content and _meta
        const readResponse = await clientConnection.readTextFile({
            path: "/test.txt",
            sessionId: "test-session",
        });
        expect(readResponse.content).toBe("test content");
        expect(readResponse._meta).toEqual({
            encoding: "utf-8",
        });
        // Test initialize with _meta
        const initResponse = await agentConnection.initialize({
            protocolVersion: PROTOCOL_VERSION,
            clientCapabilities: {},
        });
        expect(initResponse._meta).toEqual({
            agentVersion: "2.0.0",
        });
        // Test authenticate returning void
        const authResponseVoid = await agentConnection.authenticate({
            methodId: "none",
        });
        expect(authResponseVoid).toEqual({});
        // Test authenticate returning response with _meta
        const authResponse = await agentConnection.authenticate({
            methodId: "oauth",
        });
        expect(authResponse).toEqual({
            _meta: {
                authenticated: true,
                method: "oauth",
            },
        });
        // Test newSession with _meta
        const sessionResponse = await agentConnection.newSession({
            cwd: "/test",
            mcpServers: [],
        });
        expect(sessionResponse._meta).toEqual({
            sessionType: "ephemeral",
        });
        // Test loadSession returning minimal response
        const loadResponse = await agentConnection.loadSession({
            sessionId: "test-session",
            mcpServers: [],
            cwd: "/test",
        });
        expect(loadResponse).toEqual({});
    });
});
//# sourceMappingURL=acp.test.js.map