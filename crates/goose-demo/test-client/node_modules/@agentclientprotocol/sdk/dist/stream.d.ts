import type { AnyMessage } from "./jsonrpc.js";
/**
 * Stream interface for ACP connections.
 *
 * This type powers the bidirectional communication for an ACP connection,
 * providing readable and writable streams of messages.
 *
 * The most common way to create a Stream is using {@link ndJsonStream}.
 */
export type Stream = {
    writable: WritableStream<AnyMessage>;
    readable: ReadableStream<AnyMessage>;
};
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
export declare function ndJsonStream(output: WritableStream<Uint8Array>, input: ReadableStream<Uint8Array>): Stream;
