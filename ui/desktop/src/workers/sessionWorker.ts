/* eslint-env worker */
/* global self, postMessage */

/**
 * Web Worker for managing chat sessions in the background
 * Handles streaming, state management, and concurrent sessions
 */

import { SessionManager } from './SessionManager';
import { WorkerCommand, WorkerResponse } from './types';

let sessionManager: SessionManager | null = null;

// Handle messages from the main thread
self.onmessage = async (event: MessageEvent<WorkerCommand>) => {
  const message = event.data;

  try {
    if (message.type === 'INIT') {
      sessionManager = new SessionManager(message.config);
      postMessage({ type: 'READY' } as WorkerResponse);
      return;
    }

    if (!sessionManager) {
      throw new Error('SessionManager not initialized');
    }

    switch (message.type) {
      case 'INIT_SESSION': {
        sessionManager.initSession(message.sessionId);
        postMessage({
          type: 'SESSION_INITIALIZED',
          sessionId: message.sessionId,
        } as WorkerResponse);
        break;
      }

      case 'LOAD_SESSION': {
        const state = await sessionManager.loadSession(message.sessionId);

        postMessage({
          type: 'SESSION_LOADED',
          sessionId: message.sessionId,
          state,
        } as WorkerResponse);

        postMessage({
          type: 'SESSION_UPDATE',
          sessionId: message.sessionId,
          state: {
            session: state.session,
            messages: state.messages,
            tokenState: state.tokenState,
            notifications: state.notifications,
            streamState: state.streamState,
          },
        } as WorkerResponse);
        break;
      }

      case 'START_STREAM': {
        postMessage({
          type: 'STREAM_STARTED',
          sessionId: message.sessionId,
        } as WorkerResponse);

        await sessionManager.startStream(
          message.sessionId,
          message.userMessage,
          message.messages,
          (stateUpdate) => {
            postMessage({
              type: 'SESSION_UPDATE',
              sessionId: message.sessionId,
              state: stateUpdate,
            } as WorkerResponse);
          }
        );

        postMessage({
          type: 'STREAM_FINISHED',
          sessionId: message.sessionId,
        } as WorkerResponse);
        break;
      }

      case 'STOP_STREAM': {
        sessionManager.stopStream(message.sessionId);
        postMessage({
          type: 'STREAM_FINISHED',
          sessionId: message.sessionId,
        } as WorkerResponse);
        break;
      }

      case 'DESTROY_SESSION': {
        sessionManager.destroySession(message.sessionId);
        break;
      }

      case 'GET_SESSION_STATE': {
        const state = sessionManager.getSessionState(message.sessionId);
        postMessage({
          type: 'SESSION_STATE',
          sessionId: message.sessionId,
          state,
        } as WorkerResponse);
        break;
      }

      case 'GET_ALL_SESSIONS': {
        const sessions = sessionManager.getAllSessions();
        postMessage({
          type: 'ALL_SESSIONS',
          sessions,
        } as WorkerResponse);
        break;
      }

      default: {
        console.warn('Unknown worker command:', (message as WorkerCommand).type);
      }
    }
  } catch (error) {
    const errorMessage = error instanceof Error ? error.message : 'Unknown error';
    console.error('Worker error:', errorMessage);

    if ('sessionId' in message) {
      postMessage({
        type: 'ERROR',
        sessionId: message.sessionId,
        error: errorMessage,
      } as WorkerResponse);
    }
  }
};

self.onerror = (error) => {
  console.error('Worker error:', error);
};

export {};
