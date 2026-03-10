import type { Message } from '../api';

export interface LoadedSkill {
  name: string;
  content: string;
}

function getToolName(toolCallName: string): string {
  const lastIndex = toolCallName.lastIndexOf('__');
  if (lastIndex === -1) return toolCallName;
  return toolCallName.substring(lastIndex + 2);
}

export function detectLoadedSkills(messages: Message[]): LoadedSkill[] {
  const skills: LoadedSkill[] = [];
  const seenNames = new Set<string>();

  for (const message of messages) {
    for (const content of message.content) {
      if (content.type !== 'toolRequest') continue;
      const toolCall = content.toolCall as { name?: string; arguments?: Record<string, unknown> };
      if (!toolCall?.name) continue;

      const toolName = getToolName(toolCall.name);
      if (toolName !== 'load') continue;

      const args = toolCall.arguments ?? {};
      const source = args.source;
      if (typeof source !== 'string') continue;

      // Find matching tool response with content
      const requestId = (content as { id?: string }).id;
      if (!requestId) continue;

      for (const msg of messages) {
        for (const resp of msg.content) {
          if (resp.type !== 'toolResponse') continue;
          const toolResp = resp as { id?: string; toolResult?: { content?: string } };
          if (toolResp.id !== requestId) continue;

          const resultContent =
            typeof toolResp.toolResult?.content === 'string'
              ? toolResp.toolResult.content
              : '';

          if (!seenNames.has(source)) {
            seenNames.add(source);
            skills.push({ name: source, content: resultContent });
          }
        }
      }
    }
  }

  return skills;
}
