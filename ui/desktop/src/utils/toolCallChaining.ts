import {
  getAnyToolConfirmationData,
  getTextAndImageContent,
  getToolRequests,
  getToolResponses,
  type Message,
  type NotificationEvent,
  type ToolConfirmationData,
  type ToolResponse,
} from '../types/message';

function messageHasVisibleText(message: Message): boolean {
  const { textContent } = getTextAndImageContent(message);
  return textContent.trim().length > 0;
}

function messageAffectsToolLookups(message: Message): boolean {
  return message.content.some(
    (content) =>
      content.type === 'toolRequest' ||
      content.type === 'toolResponse' ||
      content.type === 'toolConfirmationRequest' ||
      (content.type === 'actionRequired' && content.data.actionType === 'toolConfirmation')
  );
}

function sameMessageIdentities(left: Message[], right: Message[]): boolean {
  if (left === right) {
    return true;
  }
  if (left.length !== right.length) {
    return false;
  }
  return left.every((message, index) => message === right[index]);
}

let cachedToolCallChains: { messages: Message[]; chains: number[][] } | undefined;

export function identifyConsecutiveToolCalls(messages: Message[]): number[][] {
  if (cachedToolCallChains && sameMessageIdentities(cachedToolCallChains.messages, messages)) {
    return cachedToolCallChains.chains;
  }

  if (cachedToolCallChains && cachedToolCallChains.messages.length === messages.length) {
    let onlyLastChanged = true;
    for (let i = 0; i < messages.length - 1; i++) {
      if (cachedToolCallChains.messages[i] !== messages[i]) {
        onlyLastChanged = false;
        break;
      }
    }
    if (onlyLastChanged) {
      const previousLast = cachedToolCallChains.messages[messages.length - 1];
      const nextLast = messages[messages.length - 1];
      if (
        previousLast &&
        nextLast &&
        !messageAffectsToolLookups(previousLast) &&
        !messageAffectsToolLookups(nextLast) &&
        messageHasVisibleText(previousLast) === messageHasVisibleText(nextLast)
      ) {
        cachedToolCallChains = { messages, chains: cachedToolCallChains.chains };
        return cachedToolCallChains.chains;
      }
    }
  }

  if (
    cachedToolCallChains &&
    cachedToolCallChains.messages.length + 1 === messages.length &&
    cachedToolCallChains.messages.every((message, index) => message === messages[index])
  ) {
    const appended = messages[messages.length - 1];
    if (appended && !messageAffectsToolLookups(appended) && !messageHasVisibleText(appended)) {
      cachedToolCallChains = { messages, chains: cachedToolCallChains.chains };
      return cachedToolCallChains.chains;
    }
  }

  const chains: number[][] = [];
  let currentChain: number[] = [];

  for (let i = 0; i < messages.length; i++) {
    const message = messages[i];
    const toolRequests = getToolRequests(message);
    const toolResponses = getToolResponses(message);
    const hasText = messageHasVisibleText(message);

    if (toolResponses.length > 0 && toolRequests.length === 0) {
      continue;
    }

    if (toolRequests.length > 0) {
      if (hasText) {
        if (currentChain.length > 1) {
          chains.push([...currentChain]);
        }
        currentChain = [i];
      } else {
        currentChain.push(i);
      }
    } else if (hasText) {
      if (currentChain.length > 1) {
        chains.push([...currentChain]);
      }
      currentChain = [];
    } else {
      if (currentChain.length > 1) {
        chains.push([...currentChain]);
      }
      currentChain = [];
    }
  }

  if (currentChain.length > 1) {
    chains.push(currentChain);
  }

  cachedToolCallChains = { messages, chains };
  return chains;
}

const lastIndexByChain = new WeakMap<number[][], Set<number>>();
const membershipByChain = new WeakMap<number[][], Set<number>>();

function chainSets(chains: number[][]): { membership: Set<number>; lastIndexes: Set<number> } {
  const cachedMembership = membershipByChain.get(chains);
  const cachedLastIndexes = lastIndexByChain.get(chains);
  if (cachedMembership && cachedLastIndexes) {
    return { membership: cachedMembership, lastIndexes: cachedLastIndexes };
  }

  const membership = new Set<number>();
  const lastIndexes = new Set<number>();
  for (const chain of chains) {
    if (chain.length === 0) {
      continue;
    }
    lastIndexes.add(chain[chain.length - 1]);
    for (const index of chain) {
      membership.add(index);
    }
  }
  membershipByChain.set(chains, membership);
  lastIndexByChain.set(chains, lastIndexes);
  return { membership, lastIndexes };
}

export function isInChain(messageIndex: number, chains: number[][]): boolean {
  return chainSets(chains).membership.has(messageIndex);
}

export function shouldHideTimestamp(messageIndex: number, chains: number[][]): boolean {
  const { membership, lastIndexes } = chainSets(chains);
  return membership.has(messageIndex) && !lastIndexes.has(messageIndex);
}

function resolvedModelFromMessage(message: Message | undefined): string | null {
  if (!message || message.role !== 'assistant' || !message.metadata.userVisible) {
    return null;
  }
  return message.metadata.inference?.resolvedModel ?? null;
}

let cachedPreviousResolvedModels: { messages: Message[]; models: Array<string | null> } | undefined;

export function getPreviousResolvedModels(messages: Message[]): Array<string | null> {
  if (
    cachedPreviousResolvedModels &&
    sameMessageIdentities(cachedPreviousResolvedModels.messages, messages)
  ) {
    return cachedPreviousResolvedModels.models;
  }

  if (
    cachedPreviousResolvedModels &&
    cachedPreviousResolvedModels.messages.length === messages.length
  ) {
    let onlyLastChanged = true;
    for (let i = 0; i < messages.length - 1; i++) {
      if (cachedPreviousResolvedModels.messages[i] !== messages[i]) {
        onlyLastChanged = false;
        break;
      }
    }
    if (onlyLastChanged) {
      cachedPreviousResolvedModels = {
        messages,
        models: cachedPreviousResolvedModels.models,
      };
      return cachedPreviousResolvedModels.models;
    }
  }

  if (
    cachedPreviousResolvedModels &&
    cachedPreviousResolvedModels.messages.length + 1 === messages.length &&
    cachedPreviousResolvedModels.messages.every((message, index) => message === messages[index])
  ) {
    const previousModels = cachedPreviousResolvedModels.models;
    const lastResolved =
      resolvedModelFromMessage(messages[messages.length - 2]) ??
      previousModels[previousModels.length - 1] ??
      null;
    const models = [...previousModels, lastResolved];
    cachedPreviousResolvedModels = { messages, models };
    return models;
  }

  const models: Array<string | null> = new Array(messages.length);
  let lastResolvedModel: string | null = null;
  for (let i = 0; i < messages.length; i++) {
    models[i] = lastResolvedModel;
    const model = resolvedModelFromMessage(messages[i]);
    if (model) {
      lastResolvedModel = model;
    }
  }
  cachedPreviousResolvedModels = { messages, models };
  return models;
}

export type ToolCallLookups = {
  responsesById: Map<string, ToolResponse & { type: 'toolResponse' }>;
  confirmationsById: Map<string, ToolConfirmationData>;
  requestIds: Set<string>;
  pendingConfirmationIds: Set<string>;
};

let cachedLookups: { messages: Message[]; lookups: ToolCallLookups } | undefined;

export function buildToolCallLookups(messages: Message[]): ToolCallLookups {
  if (cachedLookups && sameMessageIdentities(cachedLookups.messages, messages)) {
    return cachedLookups.lookups;
  }

  if (cachedLookups && cachedLookups.messages.length === messages.length) {
    let toolRelatedChange = false;
    for (let i = 0; i < messages.length; i++) {
      if (cachedLookups.messages[i] === messages[i]) {
        continue;
      }
      if (
        messageAffectsToolLookups(cachedLookups.messages[i]) ||
        messageAffectsToolLookups(messages[i])
      ) {
        toolRelatedChange = true;
        break;
      }
    }
    if (!toolRelatedChange) {
      cachedLookups = { messages, lookups: cachedLookups.lookups };
      return cachedLookups.lookups;
    }
  }

  if (
    cachedLookups &&
    cachedLookups.messages.length + 1 === messages.length &&
    cachedLookups.messages.every((message, index) => message === messages[index])
  ) {
    const appended = messages[messages.length - 1];
    if (appended && !messageAffectsToolLookups(appended)) {
      cachedLookups = { messages, lookups: cachedLookups.lookups };
      return cachedLookups.lookups;
    }
  }

  const responsesById = new Map<string, ToolResponse & { type: 'toolResponse' }>();
  const confirmationsById = new Map<string, ToolConfirmationData>();
  const requestIds = new Set<string>();
  const respondedIds = new Set<string>();
  const pendingConfirmationIds = new Set<string>();

  for (const message of messages) {
    for (const request of getToolRequests(message)) {
      requestIds.add(request.id);
    }
    for (const response of getToolResponses(message)) {
      responsesById.set(response.id, response);
      respondedIds.add(response.id);
    }

    const confirmation = getAnyToolConfirmationData(message);
    if (confirmation) {
      confirmationsById.set(confirmation.id, confirmation);
    }
  }

  for (const confirmationId of confirmationsById.keys()) {
    if (!respondedIds.has(confirmationId)) {
      pendingConfirmationIds.add(confirmationId);
    }
  }

  const lookups = {
    responsesById,
    confirmationsById,
    requestIds,
    pendingConfirmationIds,
  };
  cachedLookups = { messages, lookups };
  return lookups;
}

export function messageToolLookupsChanged(
  message: Message,
  previous: ToolCallLookups,
  next: ToolCallLookups
): boolean {
  if (previous === next) {
    return false;
  }

  const confirmation = getAnyToolConfirmationData(message);
  if (
    confirmation &&
    previous.requestIds.has(confirmation.id) !== next.requestIds.has(confirmation.id)
  ) {
    return true;
  }

  for (const request of getToolRequests(message)) {
    if (
      previous.responsesById.get(request.id) !== next.responsesById.get(request.id) ||
      previous.confirmationsById.get(request.id) !== next.confirmationsById.get(request.id) ||
      previous.pendingConfirmationIds.has(request.id) !== next.pendingConfirmationIds.has(request.id)
    ) {
      return true;
    }
  }

  return false;
}

export function messageNotificationsChanged(
  message: Message,
  previous: Map<string, NotificationEvent[]>,
  next: Map<string, NotificationEvent[]>
): boolean {
  if (previous === next) {
    return false;
  }

  for (const request of getToolRequests(message)) {
    if (previous.get(request.id) !== next.get(request.id)) {
      return true;
    }
  }

  return false;
}
