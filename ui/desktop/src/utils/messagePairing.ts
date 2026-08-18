import type { Message } from '../types/message';

function hasPairingContent(message: Message): boolean {
  return message.content.some(
    (content) => content.type === 'toolResponse' || content.type === 'actionRequired'
  );
}

export function onlyLastMessageChanged(
  previousMessages: Message[],
  nextMessages: Message[]
): boolean {
  if (previousMessages.length !== nextMessages.length || nextMessages.length === 0) {
    return false;
  }

  for (let index = 0; index < nextMessages.length - 1; index++) {
    if (previousMessages[index] !== nextMessages[index]) {
      return false;
    }
  }

  return previousMessages[previousMessages.length - 1] !== nextMessages[nextMessages.length - 1];
}

export function laterPairingChanged(
  previousMessages: Message[],
  nextMessages: Message[],
  message: Message
): boolean {
  const previousIndex = previousMessages.indexOf(message);
  const nextIndex = nextMessages.indexOf(message);
  if (previousIndex === -1 || nextIndex === -1) {
    return true;
  }

  if (onlyLastMessageChanged(previousMessages, nextMessages)) {
    if (nextIndex === nextMessages.length - 1) {
      return true;
    }
    return (
      hasPairingContent(previousMessages[previousMessages.length - 1]) ||
      hasPairingContent(nextMessages[nextMessages.length - 1])
    );
  }

  const previousPairing = pairingContentAfter(previousMessages, previousIndex);
  const nextPairing = pairingContentAfter(nextMessages, nextIndex);
  if (previousPairing.length !== nextPairing.length) {
    return true;
  }
  return previousPairing.some((content, index) => content !== nextPairing[index]);
}

function pairingContentAfter(messages: Message[], index: number) {
  const pairing: Message['content'][number][] = [];
  for (let i = index + 1; i < messages.length; i++) {
    for (const content of messages[i].content) {
      if (content.type === 'toolResponse' || content.type === 'actionRequired') {
        pairing.push(content);
      }
    }
  }
  return pairing;
}
