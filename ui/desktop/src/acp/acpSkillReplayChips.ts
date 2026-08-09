import { parseSkillInstructionPrompt } from '../components/skills/lib/skillChatPrompt';
import type { Message, MessageChip } from '../types/message';

const pendingReplayChips = new Map<string, MessageChip[]>();

export function getPendingReplayChips(messageId: string): MessageChip[] {
  return pendingReplayChips.get(messageId) ?? [];
}

export function setPendingReplayChips(messageId: string, chips: MessageChip[]): void {
  if (chips.length === 0) return;
  pendingReplayChips.set(messageId, chips);
}

export function clearPendingReplayChips(messageId: string): void {
  pendingReplayChips.delete(messageId);
}

export function skillInstructionToChips(text: string): MessageChip[] {
  return parseSkillInstructionPrompt(text).map((label) => ({
    label,
    type: 'skill' as const,
  }));
}

export function isAssistantOnlyAudience(annotations: unknown): boolean {
  if (!annotations || typeof annotations !== 'object' || Array.isArray(annotations)) {
    return false;
  }
  const audience = (annotations as { audience?: string[] }).audience;
  return Boolean(audience && audience.length > 0 && !audience.includes('user'));
}

export function attachChipsToMessage(message: Message, chips: MessageChip[]): void {
  if (chips.length === 0) return;
  const existing = message.metadata.chips ?? [];
  const labels = new Set(existing.map((chip) => chip.label));
  const merged = [...existing];
  for (const chip of chips) {
    if (!labels.has(chip.label)) {
      merged.push(chip);
      labels.add(chip.label);
    }
  }
  message.metadata.chips = merged;
}

export function clearSkillReplayChips(): void {
  pendingReplayChips.clear();
}
