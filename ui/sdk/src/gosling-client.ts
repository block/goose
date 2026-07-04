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
  type SetSessionModelRequest,
  type SetSessionModelResponse,
} from "@agentclientprotocol/sdk";
import {
  GoslingExtClient,
  installGoslingExtAgentRequestDispatcher,
  installGoslingExtNotificationDispatcher,
  type GoslingClientCallbacks,
} from "./generated/client.gen.js";
import { createHttpStream } from "./http-stream.js";

export class GoslingClient {
  private conn: ClientSideConnection;
  private ext: GoslingExtClient;

  constructor(
    toClient: () => GoslingClientCallbacks,
    streamOrUrl: Stream | string,
  ) {
    const stream =
      typeof streamOrUrl === "string"
        ? createHttpStream(streamOrUrl)
        : streamOrUrl;
    const toAcpClient = () =>
      installGoslingExtAgentRequestDispatcher(
        installGoslingExtNotificationDispatcher(toClient()),
      );
    this.conn = new ClientSideConnection(toAcpClient, stream);
    this.ext = new GoslingExtClient(this.conn);
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

  unstable_resumeSession(
    params: ResumeSessionRequest,
  ): Promise<ResumeSessionResponse> {
    return this.conn.unstable_resumeSession(params);
  }

  unstable_closeSession(
    params: CloseSessionRequest,
  ): Promise<CloseSessionResponse> {
    return this.conn.unstable_closeSession(params);
  }

  unstable_setSessionModel(
    params: SetSessionModelRequest,
  ): Promise<SetSessionModelResponse> {
    return this.conn.unstable_setSessionModel(params);
  }

  extMethod(
    method: string,
    params: Record<string, unknown>,
  ): Promise<Record<string, unknown>> {
    return this.conn.extMethod(method, params);
  }

  get gosling(): GoslingExtClient {
    return this.ext;
  }
}
