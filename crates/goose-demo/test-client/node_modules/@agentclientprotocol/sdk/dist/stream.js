/**
 * Creates an ACP Stream from a pair of newline-delimited JSON streams.
 *
 * This is the typical way to handle ACP connections over stdio, converting
 * between AnyMessage objects and newline-delimited JSON.
 *
 * @param output - The writable stream to send encoded messages to
 * @param input - The readable stream to receive encoded messages from
 * @returns A Stream for bidirectional ACP communication
 */
export function ndJsonStream(output, input) {
    const textEncoder = new TextEncoder();
    const textDecoder = new TextDecoder();
    const readable = new ReadableStream({
        async start(controller) {
            let content = "";
            const reader = input.getReader();
            try {
                while (true) {
                    const { value, done } = await reader.read();
                    if (done) {
                        break;
                    }
                    if (!value) {
                        continue;
                    }
                    content += textDecoder.decode(value, { stream: true });
                    const lines = content.split("\n");
                    content = lines.pop() || "";
                    for (const line of lines) {
                        const trimmedLine = line.trim();
                        if (trimmedLine) {
                            try {
                                const message = JSON.parse(trimmedLine);
                                controller.enqueue(message);
                            }
                            catch (err) {
                                console.error("Failed to parse JSON message:", trimmedLine, err);
                            }
                        }
                    }
                }
            }
            finally {
                reader.releaseLock();
                controller.close();
            }
        },
    });
    const writable = new WritableStream({
        async write(message) {
            const content = JSON.stringify(message) + "\n";
            const writer = output.getWriter();
            try {
                await writer.write(textEncoder.encode(content));
            }
            finally {
                writer.releaseLock();
            }
        },
    });
    return { readable, writable };
}
//# sourceMappingURL=stream.js.map