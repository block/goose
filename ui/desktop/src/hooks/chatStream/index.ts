/**
 * chatStream module — split from the monolithic useChatStream.ts
 *
 * - streamReducer.ts: state types, actions, reducer, initial state
 * - streamDecoder.ts: SSE stream parsing, message merging, motion prefs
 * - useChatStream.ts: the original hook (re-exported below)
 */
export { useChatStream } from '../useChatStream';
export { streamReducer, initialState } from './streamReducer';
export type { StreamState, StreamAction } from './streamReducer';
export { streamFromResponse, pushMessage, prefersReducedMotion } from './streamDecoder';
