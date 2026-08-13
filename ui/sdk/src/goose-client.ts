import {
  ClientSideConnection,
  type Stream,
  type InitializeRequest,
  type InitializeResponse,
  type NewSessionRequest,
  type NewSessionResponse,
  type LoadSessionRequest,
  type LoadSessionResponse,
  type PromptRequest,
  type PromptResponse,
  type CancelNotification,
  type AuthenticateRequest,
  type AuthenticateResponse,
  type SetSessionModeRequest,
  type SetSessionModeResponse,
  type SetSessionConfigOptionRequest,
  type SetSessionConfigOptionResponse,
  type ForkSessionRequest,
  type ForkSessionResponse,
  type ListSessionsRequest,
  type ListSessionsResponse,
  type ResumeSessionRequest,
  type ResumeSessionResponse,
  type CloseSessionRequest,
  type CloseSessionResponse,
} from "@agentclientprotocol/sdk";
import {
  GooseExtClient,
  installGooseExtAgentRequestDispatcher,
  installGooseExtNotificationDispatcher,
  type GooseClientCallbacks,
} from "./generated/client.gen.js";
import { createHttpStream } from "./http-stream.js";

export class GooseClient {
  private conn: ClientSideConnection;
  private ext: GooseExtClient;

  constructor(
    toClient: () => GooseClientCallbacks,
    streamOrUrl: Stream | string,
  ) {
    const stream =
      typeof streamOrUrl === "string"
        ? createHttpStream(streamOrUrl)
        : streamOrUrl;
    const toAcpClient = () =>
      installGooseExtAgentRequestDispatcher(
        installGooseExtNotificationDispatcher(toClient()),
      );
    this.conn = new ClientSideConnection(toAcpClient, stream);
    this.ext = new GooseExtClient(this.conn);
  }

  get signal(): AbortSignal {
    return this.conn.signal;
  }

  get closed(): Promise<void> {
    return this.conn.closed;
  }

  initialize(params: InitializeRequest): Promise<InitializeResponse> {
    return this.conn.initialize(params);
  }

  newSession(params: NewSessionRequest): Promise<NewSessionResponse> {
    return this.conn.newSession(params);
  }

  loadSession(params: LoadSessionRequest): Promise<LoadSessionResponse> {
    return this.conn.loadSession(params);
  }

  prompt(params: PromptRequest): Promise<PromptResponse> {
    return this.conn.prompt(params);
  }

  cancel(params: CancelNotification): Promise<void> {
    return this.conn.cancel(params);
  }

  authenticate(params: AuthenticateRequest): Promise<AuthenticateResponse> {
    return this.conn.authenticate(params);
  }

  setSessionMode(
    params: SetSessionModeRequest,
  ): Promise<SetSessionModeResponse> {
    return this.conn.setSessionMode(params);
  }

  setSessionConfigOption(
    params: SetSessionConfigOptionRequest,
  ): Promise<SetSessionConfigOptionResponse> {
    return this.conn.setSessionConfigOption(params);
  }

  unstable_forkSession(
    params: ForkSessionRequest,
  ): Promise<ForkSessionResponse> {
    return this.conn.unstable_forkSession(params);
  }

  listSessions(params: ListSessionsRequest): Promise<ListSessionsResponse> {
    return this.conn.listSessions(params);
  }

  resumeSession(params: ResumeSessionRequest): Promise<ResumeSessionResponse> {
    return this.conn.resumeSession(params);
  }

  closeSession(params: CloseSessionRequest): Promise<CloseSessionResponse> {
    return this.conn.closeSession(params);
  }

  extMethod(
    method: string,
    params: Record<string, unknown>,
  ): Promise<Record<string, unknown>> {
    return this.conn.extMethod(method, params);
  }

  get goose(): GooseExtClient {
    return this.ext;
  }
}
