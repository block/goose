import { client, methods, type ClientConnection, type Stream } from '@agentclientprotocol/sdk';
import {
  GOOSE_EXT_AGENT_REQUESTS,
  GOOSE_EXT_NOTIFICATIONS,
  GooseExtClient,
  type GooseClientCallbacks,
  zGooseSessionNotification_unstable,
  zRequestRecipeParams_unstable,
} from '@aaif/goose-sdk';

const [gooseSessionUpdate] = GOOSE_EXT_NOTIFICATIONS;
const [gooseRecipeParamsRequest] = GOOSE_EXT_AGENT_REQUESTS;

export type GooseAcpCallbacks = Required<
  Pick<
    GooseClientCallbacks,
    | 'requestPermission'
    | 'sessionUpdate'
    | 'unstable_createElicitation'
    | 'unstable_sessionRecipeRequestParams'
    | 'unstable_sessionUpdate'
  >
>;

export type GooseAcpClient = {
  connection: ClientConnection;
  goose: GooseExtClient;
};

export function connectGooseAcpClient(
  stream: Stream,
  callbacks: GooseAcpCallbacks
): GooseAcpClient {
  const app = client({ name: 'goose' })
    .onRequest(methods.client.session.requestPermission, (context) =>
      callbacks.requestPermission(context.params)
    )
    .onNotification(methods.client.session.update, (context) =>
      callbacks.sessionUpdate(context.params)
    )
    .onRequest(methods.client.elicitation.create, (context) =>
      callbacks.unstable_createElicitation(context.params)
    )
    .onRequest(gooseRecipeParamsRequest.method, zRequestRecipeParams_unstable, (context) =>
      callbacks.unstable_sessionRecipeRequestParams(context.params)
    )
    .onNotification(gooseSessionUpdate.method, zGooseSessionNotification_unstable, (context) =>
      callbacks.unstable_sessionUpdate(context.params)
    );

  const connection = app.connect(stream);
  const goose = new GooseExtClient(connection.agent);

  return { connection, goose };
}
