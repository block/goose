// Adapts the wasm RoamConnection (a raw ACP byte duplex over iroh) into the
// Web Streams pair the ACP SDK's ndJsonStream expects.
//
//   RoamConnection.recv(): Promise<Uint8Array | null>   -> ReadableStream
//   RoamConnection.send(Uint8Array): Promise<void>       <- WritableStream
//
// ndJsonStream then turns those byte streams into a typed AnyMessage stream,
// and ClientSideConnection drives the ACP protocol over it.
import type { RoamConnection } from "./wasm/goose_roaming_web.js";

export function roamByteStreams(conn: RoamConnection): {
  writable: WritableStream<Uint8Array>;
  readable: ReadableStream<Uint8Array>;
} {
  const readable = new ReadableStream<Uint8Array>({
    async pull(controller) {
      const chunk = await conn.recv();
      if (chunk == null || chunk.length === 0) {
        controller.close();
        return;
      }
      controller.enqueue(chunk);
    },
    cancel() {
      conn.close();
    },
  });

  const writable = new WritableStream<Uint8Array>({
    async write(chunk) {
      // send() awaits when the outbound channel is full -> backpressure.
      await conn.send(chunk);
    },
    close() {
      conn.close();
    },
    abort() {
      conn.close();
    },
  });

  return { writable, readable };
}
