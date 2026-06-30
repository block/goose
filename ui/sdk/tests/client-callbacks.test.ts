import assert from "node:assert/strict";
import { test } from "node:test";
import {
  installGooseExtAgentRequestDispatcher,
  installGooseExtNotificationDispatcher,
} from "../src/generated/client.gen.ts";
import type {
  GooseSessionNotification_unstable,
  RecipeParamsResponse_unstable,
  RequestRecipeParams_unstable,
} from "../src/generated/types.gen.ts";
import type {
  RequestPermissionRequest,
  RequestPermissionResponse,
  SessionNotification,
} from "@agentclientprotocol/sdk";

class ClassBackedCallbacks {
  #events: string[] = [];

  get events(): string[] {
    return this.#events;
  }

  async requestPermission(
    _params: RequestPermissionRequest,
  ): Promise<RequestPermissionResponse> {
    this.#events.push("requestPermission");
    return { outcome: { outcome: "cancelled" } };
  }

  async sessionUpdate(_params: SessionNotification): Promise<void> {
    this.#events.push("sessionUpdate");
  }

  async extNotification(
    method: string,
    _params: Record<string, unknown>,
  ): Promise<void> {
    this.#events.push(`extNotification:${method}`);
  }

  async unstable_sessionUpdate(
    notification: GooseSessionNotification_unstable,
  ): Promise<void> {
    this.#events.push(
      `unstable_sessionUpdate:${notification.update.sessionUpdate}`,
    );
  }
}

class MinimalCallbacks {
  async requestPermission(
    _params: RequestPermissionRequest,
  ): Promise<RequestPermissionResponse> {
    return { outcome: { outcome: "cancelled" } };
  }

  async sessionUpdate(_params: SessionNotification): Promise<void> {}
}

class AgentRequestCallbacks extends MinimalCallbacks {
  events: string[] = [];

  async unstable_sessionRecipeRequestParams(
    request: RequestRecipeParams_unstable,
  ): Promise<RecipeParamsResponse_unstable> {
    this.events.push(`typed:${request.sessionId}`);
    return { action: "submit", values: { name: "Ada" } };
  }

  async extMethod(
    method: string,
    _params: Record<string, unknown>,
  ): Promise<Record<string, unknown>> {
    this.events.push(`extMethod:${method}`);
    return { action: "cancel" };
  }
}

class GenericAgentRequestCallbacks extends MinimalCallbacks {
  events: string[] = [];

  async extMethod(
    method: string,
    _params: Record<string, unknown>,
  ): Promise<Record<string, unknown>> {
    this.events.push(`extMethod:${method}`);
    return { action: "cancel" };
  }
}

const recipeParamRequest: RequestRecipeParams_unstable = {
  sessionId: "session-1",
  parameters: [
    {
      key: "name",
      input_type: "string",
      requirement: "user_prompt",
      description: "Name",
    },
  ],
};

const recipeParamRequestParams = recipeParamRequest as unknown as Record<
  string,
  unknown
>;

test("dispatcher preserves class-backed callback receivers", async () => {
  const callbacks = new ClassBackedCallbacks();
  const client = installGooseExtNotificationDispatcher(callbacks);

  await client.requestPermission({} as RequestPermissionRequest);
  await client.sessionUpdate({} as SessionNotification);
  await client.extNotification!("_goose/unstable/session/update", {
    sessionId: "session-1",
    update: {
      sessionUpdate: "status_message",
      status: {
        type: "notice",
        message: "ready",
      },
    },
  });
  await client.extNotification!("example/unknown", {});

  assert.deepEqual(callbacks.events, [
    "requestPermission",
    "sessionUpdate",
    "unstable_sessionUpdate:status_message",
    "extNotification:example/unknown",
  ]);
});

test("raw extNotification is optional", async () => {
  const client = installGooseExtNotificationDispatcher(new MinimalCallbacks());

  await client.extNotification!("example/unknown", {});
});

test("agent request dispatcher prefers typed callbacks", async () => {
  const callbacks = new AgentRequestCallbacks();
  const client = installGooseExtAgentRequestDispatcher(callbacks);

  const response = await client.extMethod!(
    "_goose/unstable/session/recipe/request-params",
    recipeParamRequestParams,
  );

  assert.deepEqual(response, { action: "submit", values: { name: "Ada" } });
  assert.deepEqual(callbacks.events, ["typed:session-1"]);
});

test("agent request dispatcher falls back to raw extMethod", async () => {
  const callbacks = new GenericAgentRequestCallbacks();
  const client = installGooseExtAgentRequestDispatcher(callbacks);

  const response = await client.extMethod!(
    "_goose/unstable/session/recipe/request-params",
    recipeParamRequestParams,
  );

  assert.deepEqual(response, { action: "cancel" });
  assert.deepEqual(callbacks.events, [
    "extMethod:_goose/unstable/session/recipe/request-params",
  ]);
});

test("agent request dispatcher throws when a request is unhandled", async () => {
  const client = installGooseExtAgentRequestDispatcher(new MinimalCallbacks());

  await assert.rejects(
    () =>
      client.extMethod!(
        "_goose/unstable/session/recipe/request-params",
        recipeParamRequestParams,
      ),
    /unhandled ext method: _goose\/unstable\/session\/recipe\/request-params/,
  );
});
